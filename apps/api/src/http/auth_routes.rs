use axum::{
    http::{header, HeaderMap, HeaderValue},
    Json,
};
use rag_debugger_core::{
    AuthResponse, CurrentWorkspaceResponse, LoginRequest, Organization, OrganizationId,
    SignupRequest, User, UserId, Workspace, WorkspaceId, WorkspaceRole,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{auth, error::ApiError, state::AppState};

pub async fn signup(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<SignupRequest>,
) -> Result<([(header::HeaderName, HeaderValue); 1], Json<AuthResponse>), ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let email = auth::normalize_email(&request.email)?;
    if request.password.len() < 12 {
        return Err(ApiError::BadRequest(
            "password must be at least 12 characters".to_owned(),
        ));
    }
    let workspace_name = request.workspace_name.trim();
    if workspace_name.is_empty() {
        return Err(ApiError::BadRequest(
            "workspace name must not be empty".to_owned(),
        ));
    }

    let now = OffsetDateTime::now_utc();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: workspace_name.to_owned(),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: workspace_name.to_owned(),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: email.clone(),
        name: auth::display_name(&email, request.name),
        created_at: now,
    };
    let password_hash = auth::hash_password(&request.password)?;
    let authenticated = repository
        .create_user_workspace(
            organization,
            workspace,
            user,
            WorkspaceRole::Owner,
            password_hash,
        )
        .await?;
    let cookie =
        auth::create_session(repository.as_ref(), &authenticated, &state.config().auth).await?;

    Ok((
        [(header::SET_COOKIE, auth::set_cookie_header(&cookie)?)],
        Json(AuthResponse {
            user: authenticated,
        }),
    ))
}

pub async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<([(header::HeaderName, HeaderValue); 1], Json<AuthResponse>), ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let email = auth::normalize_email(&request.email)?;
    let user = repository
        .find_user_by_email(&email)
        .await?
        .ok_or(ApiError::Unauthorized("invalid credentials".to_owned()))?;
    if !auth::verify_password(&request.password, &user.password_hash) {
        return Err(ApiError::Unauthorized("invalid credentials".to_owned()));
    }
    let cookie =
        auth::create_session(repository.as_ref(), &user.auth, &state.config().auth).await?;

    Ok((
        [(header::SET_COOKIE, auth::set_cookie_header(&cookie)?)],
        Json(AuthResponse { user: user.auth }),
    ))
}

pub async fn logout(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: HeaderMap,
) -> Result<
    (
        [(header::HeaderName, HeaderValue); 1],
        Json<serde_json::Value>,
    ),
    ApiError,
> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    auth::revoke_session(repository.as_ref(), &headers, &state.config().auth).await?;
    Ok((
        [(
            header::SET_COOKIE,
            auth::clear_cookie_header(&state.config().auth)?,
        )],
        Json(serde_json::json!({ "ok": true })),
    ))
}

pub async fn me(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let user =
        auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    Ok(Json(AuthResponse { user }))
}

pub async fn current_workspace(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CurrentWorkspaceResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let user =
        auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    Ok(Json(CurrentWorkspaceResponse {
        organization: user.organization,
        workspace: user.workspace,
        role: user.role,
    }))
}
