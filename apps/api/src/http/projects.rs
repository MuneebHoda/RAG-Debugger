use axum::{
    extract::{Extension, State},
    Json,
};
use rag_debugger_core::{AuthenticatedUser, Project};

use crate::{error::ApiError, state::AppState};

pub async fn current_project(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Project>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository.ensure_default_project(user.workspace.id).await?,
    ))
}
