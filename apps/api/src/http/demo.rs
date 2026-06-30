use axum::{extract::State, http::HeaderMap, http::StatusCode, Json};
use rag_debugger_core::{DemoLoadResponse, DemoStatus};
use rag_debugger_rag::embedding::{EmbeddingProvider, LocalHashEmbeddingProvider};

use crate::{auth, demo, error::ApiError, state::AppState};

pub async fn get_demo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DemoStatus>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let user =
        auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    let model =
        LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone()).model();
    Ok(Json(
        demo::demo_status(repository.as_ref(), user.workspace.id, &model).await?,
    ))
}

pub async fn load_demo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<DemoLoadResponse>), ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let user =
        auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    let model =
        LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone()).model();
    let response = demo::load_demo(repository.as_ref(), user.workspace.id, &model).await?;
    let status = if response.created_documents > 0 {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(response)))
}
