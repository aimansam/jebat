pub mod implants;
pub mod listeners;
pub mod tasks;

use crate::state::AppState;
use axum::Router;

pub fn build_api_router(state: AppState) -> Router {
    Router::new()
        .merge(implants::routes(state.clone()))
        .merge(tasks::routes(state.clone()))
        .merge(listeners::routes(state))
}
