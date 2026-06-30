use rag_debugger_core::*;
use sqlx::Row;
use time::OffsetDateTime;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn ensure_demo_project(
        &self,
        workspace_id: WorkspaceId,
        project: Project,
    ) -> Result<Project, StorageError> {
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, name, privacy_mode, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(project.id.0)
        .bind(workspace_id.0)
        .bind(&project.name)
        .bind(privacy_mode_to_str(project.privacy_mode))
        .bind(project.created_at)
        .bind(project.updated_at)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query(
            "SELECT id, name, privacy_mode, created_at, updated_at
             FROM projects WHERE id = $1 AND workspace_id = $2",
        )
        .bind(project.id.0)
        .bind(workspace_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;
        project_from_row(&row)
    }

    pub(super) async fn ensure_demo_source(&self, source: Source) -> Result<Source, StorageError> {
        let (source_kind, root_hint, github_owner, github_repo) = source_kind_columns(&source.kind);
        let (sync_policy, sync_cron) = sync_policy_columns(&source.sync_policy);
        sqlx::query(
            "INSERT INTO sources (
                id, project_id, name, source_kind, root_hint, github_owner, github_repo,
                sync_policy, sync_cron, target_tokens, overlap_tokens, chunking_strategy,
                created_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(source.id.0)
        .bind(source.project_id.0)
        .bind(&source.name)
        .bind(source_kind)
        .bind(root_hint)
        .bind(github_owner)
        .bind(github_repo)
        .bind(sync_policy)
        .bind(sync_cron)
        .bind(source.chunking.target_tokens as i32)
        .bind(source.chunking.overlap_tokens as i32)
        .bind(chunking_strategy_to_str(source.chunking.strategy))
        .bind(OffsetDateTime::now_utc())
        .execute(&self.pool)
        .await?;

        let row = sqlx::query(
            "SELECT id, project_id, name, source_kind, root_hint, github_owner, github_repo,
                    sync_policy, sync_cron, target_tokens, overlap_tokens, chunking_strategy
             FROM sources WHERE id = $1",
        )
        .bind(source.id.0)
        .fetch_one(&self.pool)
        .await?;
        source_from_row(&row)
    }

    pub(super) async fn upsert_demo_document_with_chunks(
        &self,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<bool, StorageError> {
        let mut transaction = self.pool.begin().await?;
        let document_insert = sqlx::query(
            "INSERT INTO documents (
                id, source_id, path, mime_type, checksum, byte_size,
                document_profile, extraction_quality, warnings, created_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(document.id.0)
        .bind(document.source_id.0)
        .bind(&document.path)
        .bind(&document.mime_type)
        .bind(&document.checksum)
        .bind(document.byte_size as i64)
        .bind(document_profile_to_str(document.profile))
        .bind(extraction_quality_to_str(document.extraction_quality))
        .bind(document_warnings_to_text(&document.warnings))
        .bind(OffsetDateTime::now_utc())
        .execute(&mut *transaction)
        .await?;
        let created = document_insert.rows_affected() == 1;
        for chunk in chunks {
            sqlx::query(
                "INSERT INTO chunks (
                    id, source_id, document_id, ordinal, text, token_count,
                    byte_start, byte_end, checksum, strategy, section_title, split_reason,
                    quality_flags, is_duplicate, text_density, evidence_score_hint, created_at
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                 ON CONFLICT (id) DO UPDATE SET
                    text = EXCLUDED.text,
                    token_count = EXCLUDED.token_count,
                    byte_start = EXCLUDED.byte_start,
                    byte_end = EXCLUDED.byte_end,
                    checksum = EXCLUDED.checksum,
                    strategy = EXCLUDED.strategy,
                    section_title = EXCLUDED.section_title,
                    split_reason = EXCLUDED.split_reason,
                    quality_flags = EXCLUDED.quality_flags,
                    is_duplicate = EXCLUDED.is_duplicate,
                    text_density = EXCLUDED.text_density,
                    evidence_score_hint = EXCLUDED.evidence_score_hint",
            )
            .bind(chunk.id.0)
            .bind(chunk.source_id.0)
            .bind(chunk.document_id.0)
            .bind(chunk.ordinal as i32)
            .bind(&chunk.text)
            .bind(chunk.token_count as i32)
            .bind(chunk.byte_range.start as i64)
            .bind(chunk.byte_range.end as i64)
            .bind(&chunk.checksum)
            .bind(chunking_strategy_to_str(chunk.strategy))
            .bind(&chunk.section_title)
            .bind(chunk_split_reason_to_str(chunk.split_reason))
            .bind(chunk_quality_flags_to_text(&chunk.quality_flags))
            .bind(chunk.is_duplicate)
            .bind(chunk.text_density)
            .bind(chunk.evidence_score_hint)
            .bind(OffsetDateTime::now_utc())
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(created)
    }

    pub(super) async fn get_demo_source(
        &self,
        workspace_id: WorkspaceId,
        version_marker: &str,
    ) -> Result<Option<SourceSummary>, StorageError> {
        let row = sqlx::query(
            "SELECT s.id, s.project_id, s.name, s.source_kind, s.root_hint,
                    s.github_owner, s.github_repo, s.sync_policy, s.sync_cron,
                    s.target_tokens, s.overlap_tokens, s.chunking_strategy
             FROM sources s
             INNER JOIN projects p ON p.id = s.project_id
             WHERE p.workspace_id = $1 AND s.root_hint = $2
             ORDER BY s.created_at DESC LIMIT 1",
        )
        .bind(workspace_id.0)
        .bind(version_marker)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };
        let source = source_from_row(&row)?;
        let documents = self.list_document_summaries(source.id).await?;
        Ok(Some(SourceSummary {
            source,
            document_count: documents.len() as u32,
            chunk_count: documents.iter().map(|item| item.chunk_count).sum(),
            documents,
        }))
    }

    pub(super) async fn latest_retrieval_query_for_source(
        &self,
        source_id: SourceId,
    ) -> Result<Option<RetrievalQueryResponse>, StorageError> {
        let row = sqlx::query(
            "SELECT r.response_json
             FROM retrieval_playground_runs r
             WHERE EXISTS (
                SELECT 1 FROM retrieval_playground_hits h
                INNER JOIN chunks c ON c.id = h.chunk_id
                WHERE h.run_id = r.id AND c.source_id = $1
             )
             ORDER BY r.created_at DESC LIMIT 1",
        )
        .bind(source_id.0)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(retrieval_response_from_row).transpose()
    }

    pub(super) async fn latest_trace_for_source(
        &self,
        source_id: SourceId,
    ) -> Result<Option<Trace>, StorageError> {
        let row = sqlx::query(
            "SELECT t.trace_json
             FROM debug_traces t
             WHERE EXISTS (
                SELECT 1 FROM retrieval_playground_hits h
                INNER JOIN chunks c ON c.id = h.chunk_id
                WHERE h.run_id = t.source_run_id AND c.source_id = $1
             )
             ORDER BY t.created_at DESC LIMIT 1",
        )
        .bind(source_id.0)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            serde_json::from_value::<Trace>(row.try_get::<serde_json::Value, _>("trace_json")?)
                .map_err(|error| StorageError::InvalidData(error.to_string()))
        })
        .transpose()
    }
}
