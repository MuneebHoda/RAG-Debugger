use axum::{extract::State, Json};
use rag_debugger_core::ProductConfig;

use crate::state::AppState;

pub async fn get_config(State(state): State<AppState>) -> Json<ProductConfig> {
    Json(state.config().product.clone())
}
