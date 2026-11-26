use crate::storage::db::{Database, ImplantRow};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct Implant {
    pub agent_id: String,
    pub hostname: String,
    pub os_version: String,
    pub username: String,
    pub internal_ip: String,
    pub external_ip: String,
    pub first_seen: String,
    pub last_checkin: String,
    pub sleep_ms: i64,
    pub jitter_pct: i64,
    pub listener_id: String,
    pub status: String,
}

impl From<ImplantRow> for Implant {
    fn from(row: ImplantRow) -> Self {
        let status = compute_status(&row.last_checkin, row.sleep_ms);
        Self {
            agent_id: hex::encode(row.agent_id),
            hostname: row.hostname,
            os_version: row.os_version,
            username: row.username,
            internal_ip: row.internal_ip,
            external_ip: row.external_ip,
            first_seen: row.first_seen,
            last_checkin: row.last_checkin,
            sleep_ms: row.sleep_ms,
            jitter_pct: row.jitter_pct,
            listener_id: row.listener_id,
            status,
        }
    }
}

fn compute_status(last_checkin: &str, sleep_ms: i64) -> String {
    let Ok(last) = chrono::DateTime::parse_from_rfc3339(last_checkin) else {
        return "unknown".into();
    };
    let elapsed = chrono::Utc::now()
        .signed_duration_since(last)
        .num_milliseconds();
    let interval = sleep_ms.max(1000);
    if elapsed < interval * 3 {
        "active".into()
    } else if elapsed < interval * 10 {
        "stale".into()
    } else {
        "dead".into()
    }
}

pub fn parse_agent_id(hex_str: &str) -> Result<[u8; 16], String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid agent id hex: {}", e))?;
    if bytes.len() != 16 {
        return Err(format!("agent id must be 16 bytes, got {}", bytes.len()));
    }
    let mut id = [0u8; 16];
    id.copy_from_slice(&bytes);
    Ok(id)
}

pub struct ImplantRegistry {
    db: Arc<Database>,
}

impl ImplantRegistry {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn handle_checkin(
        &self,
        agent_id: &[u8; 16],
        hostname: &str,
        sleep_ms: u32,
        external_ip: &str,
        listener_id: &str,
    ) -> Result<bool, String> {
        let is_new = self
            .db
            .upsert_implant(agent_id, hostname, sleep_ms, external_ip, listener_id)
            .map_err(|e| format!("db error: {}", e))?;

        let event = if is_new {
            "implant_new"
        } else {
            "implant_checkin"
        };
        let detail = format!("host={} sleep={}ms", hostname, sleep_ms);
        let _ = self.db.log_event(event, Some(agent_id), None, &detail);

        Ok(is_new)
    }

    pub fn list(&self) -> Result<Vec<Implant>, String> {
        self.db
            .list_implants()
            .map(|rows| rows.into_iter().map(Implant::from).collect())
            .map_err(|e| format!("db error: {}", e))
    }

    pub fn get(&self, agent_id: &[u8; 16]) -> Result<Option<Implant>, String> {
        self.db
            .get_implant(agent_id)
            .map(|opt| opt.map(Implant::from))
            .map_err(|e| format!("db error: {}", e))
    }

    pub fn delete(&self, agent_id: &[u8; 16]) -> Result<bool, String> {
        self.db
            .delete_implant(agent_id)
            .map_err(|e| format!("db error: {}", e))
    }
}
