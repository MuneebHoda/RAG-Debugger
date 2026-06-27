use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct OrganizationId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct WorkspaceId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct AuthSessionId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ApiKeyId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub organization_id: OrganizationId,
    pub name: String,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub name: String,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AuthenticatedUser {
    pub user: User,
    pub organization: Organization,
    pub workspace: Workspace,
    pub role: WorkspaceRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct UserWithPassword {
    pub auth: AuthenticatedUser,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AuthSessionRecord {
    pub id: AuthSessionId,
    pub user_id: UserId,
    pub workspace_id: WorkspaceId,
    pub token_hash: String,
    #[serde(with = "crate::wire_time")]
    pub expires_at: OffsetDateTime,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::wire_time::option")]
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub workspace_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AuthResponse {
    pub user: AuthenticatedUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CurrentWorkspaceResponse {
    pub organization: Organization,
    pub workspace: Workspace,
    pub role: WorkspaceRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<ApiKeyScope>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyScope {
    CiEvalRuns,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<ApiKeyScope>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::wire_time::option")]
    pub last_used_at: Option<OffsetDateTime>,
    #[serde(with = "crate::wire_time::option")]
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreatedApiKey {
    pub api_key: ApiKey,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ApiKeyRecord {
    pub api_key: ApiKey,
    pub secret_hash: String,
}
