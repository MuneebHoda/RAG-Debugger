pub mod auth;
pub mod config;
pub mod error;
pub mod http;
pub mod state;
pub mod telemetry;

use axum::Router;

pub fn app(state: state::AppState) -> Router {
    http::router(state)
}
