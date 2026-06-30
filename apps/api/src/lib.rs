pub mod auth;
pub mod config;
pub(crate) mod demo;
pub mod error;
pub mod http;
pub(crate) mod ingestion;
pub mod state;
pub mod telemetry;

use axum::Router;

pub fn app(state: state::AppState) -> Router {
    http::router(state)
}
