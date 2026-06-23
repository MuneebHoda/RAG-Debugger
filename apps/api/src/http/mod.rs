mod health;

use axum::{routing::get, Router};

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", api_v1())
        .with_state(state)
}

fn api_v1() -> Router<AppState> {
    Router::new().route("/health", get(health::healthz))
}
