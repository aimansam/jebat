use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/listeners", get(list_listeners))
        .with_state(state)
}

async fn list_listeners(State(_state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "listeners": [],
            "note": "listener management via API planned for Phase 2"
        })),
    )
}
