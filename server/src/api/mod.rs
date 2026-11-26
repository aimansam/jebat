pub mod auth;
pub mod files;
pub mod implants;
pub mod listeners;
pub mod tasks;
pub mod ws;

use crate::state::AppState;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

pub fn build_api_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(auth::routes(state.clone()))
        .merge(implants::routes(state.clone()))
        .merge(tasks::routes(state.clone()))
        .merge(listeners::routes(state.clone()))
        .merge(files::routes(state.clone()))
        .merge(ws::routes(state))
        .fallback_service(ServeDir::new("web/dist"))
        .layer(cors)
}
