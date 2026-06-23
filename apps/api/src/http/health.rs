use axum::Json;
use rag_debugger_core::health::{HealthResponse, ReadinessResponse};

use crate::{error::ApiError, state::AppState};

pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse::alive())
}

pub async fn readyz(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<Json<ReadinessResponse>, ApiError> {
    if state.is_ready() {
        Ok(Json(ReadinessResponse::ready()))
    } else {
        Err(ApiError::NotReady)
    }
}
