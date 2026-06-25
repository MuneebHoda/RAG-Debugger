use rag_debugger_core::*;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn ping(&self) -> Result<(), StorageError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub(super) async fn ensure_default_project(&self) -> Result<Project, StorageError> {
        if let Some(row) = sqlx::query(
            "SELECT id, name, privacy_mode, created_at, updated_at FROM projects ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        {
            return project_from_row(&row);
        }

        let now = OffsetDateTime::now_utc();
        let project = Project {
            id: ProjectId(Uuid::now_v7()),
            name: "Corpus Workspace".to_owned(),
            privacy_mode: PrivacyMode::LocalOnly,
            created_at: now,
            updated_at: now,
        };

        sqlx::query(
            "INSERT INTO projects (id, name, privacy_mode, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(project.id.0)
        .bind(&project.name)
        .bind(privacy_mode_to_str(project.privacy_mode))
        .bind(project.created_at)
        .bind(project.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(project)
    }
}
