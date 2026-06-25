use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use rag_debugger_core::{ApiKey, ApiKeyId, CreateApiKeyRequest, CreatedApiKey};
use uuid::Uuid;

use crate::{auth, error::ApiError, state::AppState};

pub async fn list_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKey>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let user =
        auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    Ok(Json(repository.list_api_keys(user.workspace.id).await?))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<CreatedApiKey>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let user =
        auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest(
            "API key name must not be empty".to_owned(),
        ));
    }
    Ok(Json(
        auth::create_api_key(repository.as_ref(), &user, name.to_owned(), request.scopes).await?,
    ))
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(api_key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    repository
        .revoke_api_key(ApiKeyId(api_key_id))
        .await
        .map_err(not_found_to_api("API key"))?;
    Ok(Json(serde_json::json!({ "revoked": true })))
}

fn not_found_to_api(
    label: &'static str,
) -> impl FnOnce(rag_debugger_storage::StorageError) -> ApiError {
    move |error| match error {
        rag_debugger_storage::StorageError::NotFound => {
            ApiError::NotFound(format!("{label} not found"))
        }
        other => ApiError::Storage(other),
    }
}
