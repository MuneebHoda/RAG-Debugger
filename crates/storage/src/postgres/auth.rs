use rag_debugger_core::*;
use sqlx::Row;
use time::OffsetDateTime;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn bootstrap_identity(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError> {
        if let Some(existing) = self.find_user_by_email(&user.email).await? {
            return Ok(existing.auth);
        }
        self.create_user_workspace(organization, workspace, user, role, password_hash)
            .await
    }

    pub(super) async fn create_user_workspace(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO organizations (id, name, created_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(organization.id.0)
        .bind(&organization.name)
        .bind(organization.created_at)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO workspaces (id, organization_id, name, created_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(workspace.id.0)
        .bind(workspace.organization_id.0)
        .bind(&workspace.name)
        .bind(workspace.created_at)
        .execute(&mut *transaction)
        .await?;

        let user_result = sqlx::query(
            "INSERT INTO users (id, email, name, password_hash, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user.id.0)
        .bind(&user.email)
        .bind(&user.name)
        .bind(password_hash)
        .bind(user.created_at)
        .execute(&mut *transaction)
        .await;

        if let Err(sqlx::Error::Database(error)) = &user_result {
            if error.is_unique_violation() {
                return Err(StorageError::Conflict(
                    "user email already exists".to_owned(),
                ));
            }
        }
        user_result?;

        sqlx::query(
            "INSERT INTO workspace_memberships (user_id, workspace_id, role, created_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (user_id, workspace_id) DO UPDATE SET role = EXCLUDED.role",
        )
        .bind(user.id.0)
        .bind(workspace.id.0)
        .bind(workspace_role_to_str(role))
        .bind(OffsetDateTime::now_utc())
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "UPDATE projects
             SET workspace_id = $1
             WHERE workspace_id IS NULL",
        )
        .bind(workspace.id.0)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(AuthenticatedUser {
            user,
            organization,
            workspace,
            role,
        })
    }

    pub(super) async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithPassword>, StorageError> {
        let Some(row) = sqlx::query(
            "SELECT
                u.id AS user_id, u.email, u.name AS user_name, u.password_hash, u.created_at AS user_created_at,
                w.id AS workspace_id, w.name AS workspace_name, w.created_at AS workspace_created_at,
                o.id AS organization_id, o.name AS organization_name, o.created_at AS organization_created_at,
                m.role
             FROM users u
             INNER JOIN workspace_memberships m ON m.user_id = u.id
             INNER JOIN workspaces w ON w.id = m.workspace_id
             INNER JOIN organizations o ON o.id = w.organization_id
             WHERE u.email = $1
             ORDER BY m.created_at ASC
             LIMIT 1",
        )
        .bind(email.trim().to_ascii_lowercase())
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        Ok(Some(user_with_password_from_row(&row)?))
    }

    pub(super) async fn get_authenticated_user(
        &self,
        user_id: UserId,
        workspace_id: WorkspaceId,
    ) -> Result<AuthenticatedUser, StorageError> {
        let row = sqlx::query(
            "SELECT
                u.id AS user_id, u.email, u.name AS user_name, u.created_at AS user_created_at,
                w.id AS workspace_id, w.name AS workspace_name, w.created_at AS workspace_created_at,
                o.id AS organization_id, o.name AS organization_name, o.created_at AS organization_created_at,
                m.role
             FROM users u
             INNER JOIN workspace_memberships m ON m.user_id = u.id
             INNER JOIN workspaces w ON w.id = m.workspace_id
             INNER JOIN organizations o ON o.id = w.organization_id
             WHERE u.id = $1 AND w.id = $2",
        )
        .bind(user_id.0)
        .bind(workspace_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        authenticated_user_from_row(&row)
    }

    pub(super) async fn create_auth_session(
        &self,
        session: AuthSessionRecord,
    ) -> Result<AuthSessionRecord, StorageError> {
        sqlx::query(
            "INSERT INTO auth_sessions (id, user_id, workspace_id, token_hash, expires_at, created_at, revoked_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(session.id.0)
        .bind(session.user_id.0)
        .bind(session.workspace_id.0)
        .bind(&session.token_hash)
        .bind(session.expires_at)
        .bind(session.created_at)
        .bind(session.revoked_at)
        .execute(&self.pool)
        .await?;
        Ok(session)
    }

    pub(super) async fn find_auth_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<AuthSessionRecord>, StorageError> {
        let row = sqlx::query(
            "SELECT id, user_id, workspace_id, token_hash, expires_at, created_at, revoked_at
             FROM auth_sessions
             WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| auth_session_from_row(&row)).transpose()
    }

    pub(super) async fn revoke_auth_session(&self, token_hash: &str) -> Result<(), StorageError> {
        sqlx::query("UPDATE auth_sessions SET revoked_at = $2 WHERE token_hash = $1")
            .bind(token_hash)
            .bind(OffsetDateTime::now_utc())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(super) async fn create_api_key(
        &self,
        record: ApiKeyRecord,
    ) -> Result<ApiKeyRecord, StorageError> {
        sqlx::query(
            "INSERT INTO api_keys (
                id, workspace_id, name, prefix, secret_hash, scopes, created_at, last_used_at, revoked_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(record.api_key.id.0)
        .bind(record.api_key.workspace_id.0)
        .bind(&record.api_key.name)
        .bind(&record.api_key.prefix)
        .bind(&record.secret_hash)
        .bind(api_key_scopes_to_text(&record.api_key.scopes))
        .bind(record.api_key.created_at)
        .bind(record.api_key.last_used_at)
        .bind(record.api_key.revoked_at)
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    pub(super) async fn list_api_keys(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<ApiKey>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, workspace_id, name, prefix, scopes, created_at, last_used_at, revoked_at
             FROM api_keys
             WHERE workspace_id = $1
             ORDER BY created_at DESC",
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(api_key_from_row).collect()
    }

    pub(super) async fn find_api_key(
        &self,
        secret_hash: &str,
    ) -> Result<Option<ApiKeyRecord>, StorageError> {
        let row = sqlx::query(
            "SELECT id, workspace_id, name, prefix, secret_hash, scopes, created_at, last_used_at, revoked_at
             FROM api_keys
             WHERE secret_hash = $1",
        )
        .bind(secret_hash)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| api_key_record_from_row(&row)).transpose()
    }

    pub(super) async fn touch_api_key(&self, api_key_id: ApiKeyId) -> Result<(), StorageError> {
        sqlx::query("UPDATE api_keys SET last_used_at = $2 WHERE id = $1")
            .bind(api_key_id.0)
            .bind(OffsetDateTime::now_utc())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(super) async fn revoke_api_key(&self, api_key_id: ApiKeyId) -> Result<(), StorageError> {
        let result = sqlx::query("UPDATE api_keys SET revoked_at = $2 WHERE id = $1")
            .bind(api_key_id.0)
            .bind(OffsetDateTime::now_utc())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

fn user_with_password_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<UserWithPassword, StorageError> {
    Ok(UserWithPassword {
        auth: authenticated_user_from_row(row)?,
        password_hash: row.try_get("password_hash")?,
    })
}

fn authenticated_user_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<AuthenticatedUser, StorageError> {
    let organization = Organization {
        id: OrganizationId(row.try_get("organization_id")?),
        name: row.try_get("organization_name")?,
        created_at: row.try_get("organization_created_at")?,
    };
    let workspace = Workspace {
        id: WorkspaceId(row.try_get("workspace_id")?),
        organization_id: organization.id,
        name: row.try_get("workspace_name")?,
        created_at: row.try_get("workspace_created_at")?,
    };
    let user = User {
        id: UserId(row.try_get("user_id")?),
        email: row.try_get("email")?,
        name: row.try_get("user_name")?,
        created_at: row.try_get("user_created_at")?,
    };
    let role = workspace_role_from_str(row.try_get::<String, _>("role")?.as_str())?;
    Ok(AuthenticatedUser {
        user,
        organization,
        workspace,
        role,
    })
}

fn auth_session_from_row(row: &sqlx::postgres::PgRow) -> Result<AuthSessionRecord, StorageError> {
    Ok(AuthSessionRecord {
        id: AuthSessionId(row.try_get("id")?),
        user_id: UserId(row.try_get("user_id")?),
        workspace_id: WorkspaceId(row.try_get("workspace_id")?),
        token_hash: row.try_get("token_hash")?,
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}

fn api_key_record_from_row(row: &sqlx::postgres::PgRow) -> Result<ApiKeyRecord, StorageError> {
    Ok(ApiKeyRecord {
        api_key: api_key_from_row(row)?,
        secret_hash: row.try_get("secret_hash")?,
    })
}

fn api_key_from_row(row: &sqlx::postgres::PgRow) -> Result<ApiKey, StorageError> {
    Ok(ApiKey {
        id: ApiKeyId(row.try_get("id")?),
        workspace_id: WorkspaceId(row.try_get("workspace_id")?),
        name: row.try_get("name")?,
        prefix: row.try_get("prefix")?,
        scopes: api_key_scopes_from_text(row.try_get("scopes")?)?,
        created_at: row.try_get("created_at")?,
        last_used_at: row.try_get("last_used_at")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}
