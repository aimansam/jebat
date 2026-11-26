use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/builder/profiles", get(list_profiles))
        .route("/api/builder/profiles/{name}", get(get_profile))
        .with_state(state)
}

async fn list_profiles(State(_state): State<AppState>) -> impl IntoResponse {
    let profiles = vec!["stealth", "noisy", "pivot"];
    (StatusCode::OK, Json(json!({ "profiles": profiles })))
}

async fn get_profile(Path(name): Path<String>) -> impl IntoResponse {
    let profile = match name.as_str() {
        "stealth" => Some(json!({
            "name": "stealth",
            "description": "Low and slow — 60s sleep, indirect syscalls, full evasion",
            "config": {
                "sleep_ms": 60000,
                "jitter_pct": 40,
                "protocol": "https",
                "evasion": {
                    "unhook_ntdll": true,
                    "patch_amsi": true,
                    "patch_etw": true,
                    "syscall_mode": "indirect"
                },
                "inject": { "method": "apc_queue", "target": "svchost.exe" },
                "persist": { "method": "scheduled_task", "name": "WindowsUpdate" }
            }
        })),
        "noisy" => Some(json!({
            "name": "noisy",
            "description": "Fast beacon — 5s sleep, minimal evasion, quick results",
            "config": {
                "sleep_ms": 5000,
                "jitter_pct": 10,
                "protocol": "https",
                "evasion": {
                    "unhook_ntdll": false,
                    "patch_amsi": true,
                    "patch_etw": false,
                    "syscall_mode": "direct"
                },
                "inject": { "method": "create_remote_thread", "target": "notepad.exe" },
                "persist": { "method": "registry_run", "name": "Updater" }
            }
        })),
        "pivot" => Some(json!({
            "name": "pivot",
            "description": "Internal pivot — SMB pipe, no persistence, 15s beacon",
            "config": {
                "sleep_ms": 15000,
                "jitter_pct": 25,
                "protocol": "smb",
                "evasion": {
                    "unhook_ntdll": true,
                    "patch_amsi": true,
                    "patch_etw": true,
                    "syscall_mode": "indirect"
                },
                "inject": { "method": "apc_queue", "target": "svchost.exe" },
                "persist": { "method": "none" }
            }
        })),
        _ => None,
    };

    match profile {
        Some(p) => (StatusCode::OK, Json(p)),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "unknown profile" })),
        ),
    }
}
