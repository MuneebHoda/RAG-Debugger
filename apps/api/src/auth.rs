use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::http::{header, HeaderMap, HeaderValue};
use rag_debugger_core::{
    ApiKey, ApiKeyId, ApiKeyRecord, ApiKeyScope, AuthSessionId, AuthSessionRecord,
    AuthenticatedUser, CreatedApiKey, Organization, OrganizationId, User, UserId, Workspace,
    WorkspaceId, WorkspaceRole,
};
use rag_debugger_storage::repository::AppRepository;
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{config::AuthConfig, error::ApiError};

#[derive(Debug, Clone)]
pub struct SessionCookie {
    pub name: String,
    pub value: String,
    pub max_age_seconds: i64,
    pub secure: bool,
}

pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::Internal)
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub async fn bootstrap_identity(
    repository: &dyn AppRepository,
    config: &AuthConfig,
) -> Result<AuthenticatedUser, ApiError> {
    let now = OffsetDateTime::now_utc();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: config.bootstrap_organization_name.clone(),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: config.bootstrap_workspace_name.clone(),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: normalize_email(&config.bootstrap_email)?,
        name: config.bootstrap_user_name.clone(),
        created_at: now,
    };
    let password_hash = hash_password(&config.bootstrap_password)?;
    Ok(repository
        .bootstrap_identity(
            organization,
            workspace,
            user,
            WorkspaceRole::Owner,
            password_hash,
        )
        .await?)
}

pub async fn create_session(
    repository: &dyn AppRepository,
    auth: &AuthenticatedUser,
    config: &AuthConfig,
) -> Result<SessionCookie, ApiError> {
    let token = generate_secret("sess", 32);
    let now = OffsetDateTime::now_utc();
    let session = AuthSessionRecord {
        id: AuthSessionId(Uuid::now_v7()),
        user_id: auth.user.id,
        workspace_id: auth.workspace.id,
        token_hash: hash_secret(&token),
        expires_at: now + Duration::hours(config.session_ttl_hours),
        created_at: now,
        revoked_at: None,
    };
    repository.create_auth_session(session).await?;
    Ok(SessionCookie {
        name: config.session_cookie_name.clone(),
        value: token,
        max_age_seconds: config.session_ttl_hours * 60 * 60,
        secure: config.cookie_secure,
    })
}

pub async fn authenticate_session(
    repository: &dyn AppRepository,
    headers: &HeaderMap,
    config: &AuthConfig,
) -> Result<AuthenticatedUser, ApiError> {
    let token = session_token(headers, &config.session_cookie_name)
        .ok_or(ApiError::Unauthorized("authentication required".to_owned()))?;
    let token_hash = hash_secret(&token);
    let session = repository
        .find_auth_session(&token_hash)
        .await?
        .ok_or(ApiError::Unauthorized("invalid session".to_owned()))?;
    if session.revoked_at.is_some() || session.expires_at <= OffsetDateTime::now_utc() {
        return Err(ApiError::Unauthorized("session expired".to_owned()));
    }
    Ok(repository
        .get_authenticated_user(session.user_id, session.workspace_id)
        .await?)
}

pub async fn revoke_session(
    repository: &dyn AppRepository,
    headers: &HeaderMap,
    config: &AuthConfig,
) -> Result<(), ApiError> {
    if let Some(token) = session_token(headers, &config.session_cookie_name) {
        repository.revoke_auth_session(&hash_secret(&token)).await?;
    }
    Ok(())
}

pub async fn create_api_key(
    repository: &dyn AppRepository,
    auth: &AuthenticatedUser,
    name: String,
    scopes: Vec<ApiKeyScope>,
) -> Result<CreatedApiKey, ApiError> {
    let secret = generate_secret("clab", 32);
    let prefix = secret.chars().take(14).collect::<String>();
    let api_key = ApiKey {
        id: ApiKeyId(Uuid::now_v7()),
        workspace_id: auth.workspace.id,
        name,
        prefix,
        scopes: if scopes.is_empty() {
            vec![ApiKeyScope::CiEvalRuns]
        } else {
            scopes
        },
        created_at: OffsetDateTime::now_utc(),
        last_used_at: None,
        revoked_at: None,
    };
    let record = ApiKeyRecord {
        api_key,
        secret_hash: hash_secret(&secret),
    };
    let saved = repository.create_api_key(record).await?;
    Ok(CreatedApiKey {
        api_key: saved.api_key,
        secret,
    })
}

pub async fn authenticate_api_key(
    repository: &dyn AppRepository,
    headers: &HeaderMap,
    required_scope: ApiKeyScope,
) -> Result<ApiKey, ApiError> {
    let secret =
        bearer_token(headers).ok_or(ApiError::Unauthorized("API key required".to_owned()))?;
    let record = repository
        .find_api_key(&hash_secret(&secret))
        .await?
        .ok_or(ApiError::Unauthorized("invalid API key".to_owned()))?;
    if record.api_key.revoked_at.is_some() {
        return Err(ApiError::Unauthorized("API key revoked".to_owned()));
    }
    if !record.api_key.scopes.contains(&required_scope) {
        return Err(ApiError::Forbidden("API key scope denied".to_owned()));
    }
    repository.touch_api_key(record.api_key.id).await?;
    Ok(record.api_key)
}

pub fn set_cookie_header(cookie: &SessionCookie) -> Result<HeaderValue, ApiError> {
    let secure = if cookie.secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Lax{}",
        cookie.name, cookie.value, cookie.max_age_seconds, secure
    ))
    .map_err(|_| ApiError::Internal)
}

pub fn clear_cookie_header(config: &AuthConfig) -> Result<HeaderValue, ApiError> {
    HeaderValue::from_str(&format!(
        "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax",
        config.session_cookie_name
    ))
    .map_err(|_| ApiError::Internal)
}

pub fn normalize_email(email: &str) -> Result<String, ApiError> {
    let email = email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.len() < 5 {
        return Err(ApiError::BadRequest("valid email is required".to_owned()));
    }
    Ok(email)
}

pub fn display_name(email: &str, name: Option<String>) -> String {
    name.filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_owned())
}

pub fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_secret(prefix: &str, byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    OsRng.fill_bytes(&mut bytes);
    format!("{prefix}_{}", hex::encode(bytes))
}

fn session_token(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == cookie_name).then(|| value.to_owned())
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
}
