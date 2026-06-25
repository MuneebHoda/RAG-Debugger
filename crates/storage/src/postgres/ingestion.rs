use rag_debugger_core::*;
use sqlx::Row;
use time::OffsetDateTime;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn create_source(&self, source: Source) -> Result<Source, StorageError> {
        let (source_kind, root_hint, github_owner, github_repo) = source_kind_columns(&source.kind);
        let (sync_policy, sync_cron) = sync_policy_columns(&source.sync_policy);

        sqlx::query(
            "INSERT INTO sources (
                id, project_id, name, source_kind, root_hint, github_owner, github_repo,
                sync_policy, sync_cron, target_tokens, overlap_tokens, chunking_strategy,
                created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
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

        Ok(source)
    }

    pub(super) async fn create_ingestion_run(
        &self,
        run: IngestionRun,
    ) -> Result<IngestionRun, StorageError> {
        sqlx::query(
            "INSERT INTO ingestion_runs (
                id, source_id, status, files_received, documents_created, chunks_created,
                failed_files, started_at, completed_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(run.id.0)
        .bind(run.source_id.0)
        .bind(ingestion_status_to_str(run.status))
        .bind(run.totals.files_received as i32)
        .bind(run.totals.documents_created as i32)
        .bind(run.totals.chunks_created as i32)
        .bind(run.totals.failed_files as i32)
        .bind(run.started_at)
        .bind(run.completed_at)
        .execute(&self.pool)
        .await?;

        Ok(run)
    }

    pub(super) async fn complete_ingestion_run(
        &self,
        id: IngestionRunId,
        status: IngestionRunStatus,
        totals: IngestionTotals,
    ) -> Result<IngestionRun, StorageError> {
        let completed_at = OffsetDateTime::now_utc();
        let row = sqlx::query(
            "UPDATE ingestion_runs
             SET status = $2, files_received = $3, documents_created = $4,
                 chunks_created = $5, failed_files = $6, completed_at = $7
             WHERE id = $1
             RETURNING id, source_id, status, files_received, documents_created, chunks_created,
                       failed_files, started_at, completed_at",
        )
        .bind(id.0)
        .bind(ingestion_status_to_str(status))
        .bind(totals.files_received as i32)
        .bind(totals.documents_created as i32)
        .bind(totals.chunks_created as i32)
        .bind(totals.failed_files as i32)
        .bind(completed_at)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        ingestion_run_from_row(&row)
    }

    pub(super) async fn insert_document_with_chunks(
        &self,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<Document, StorageError> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO documents (
                id, source_id, path, mime_type, checksum, byte_size,
                document_profile, extraction_quality, warnings, created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
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

        for chunk in chunks {
            sqlx::query(
                "INSERT INTO chunks (
                    id, source_id, document_id, ordinal, text, token_count,
                    byte_start, byte_end, checksum, strategy, section_title, split_reason,
                    quality_flags, is_duplicate, text_density, evidence_score_hint, created_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
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
        Ok(document)
    }

    pub(super) async fn list_sources(&self) -> Result<Vec<SourceSummary>, StorageError> {
        let source_rows = sqlx::query(
            "SELECT id, project_id, name, source_kind, root_hint, github_owner, github_repo,
                    sync_policy, sync_cron, target_tokens, overlap_tokens, chunking_strategy
             FROM sources
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::with_capacity(source_rows.len());
        for row in source_rows {
            let source = source_from_row(&row)?;
            let documents = self.list_document_summaries(source.id).await?;
            let chunk_count = documents.iter().map(|document| document.chunk_count).sum();

            summaries.push(SourceSummary {
                source,
                document_count: documents.len() as u32,
                chunk_count,
                documents,
            });
        }

        Ok(summaries)
    }

    pub(super) async fn list_document_chunks(
        &self,
        document_id: DocumentId,
    ) -> Result<Vec<Chunk>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, source_id, document_id, ordinal, text, token_count, byte_start, byte_end,
                    checksum, strategy, section_title, split_reason,
                    quality_flags, is_duplicate, text_density, evidence_score_hint
             FROM chunks
             WHERE document_id = $1
             ORDER BY ordinal ASC",
        )
        .bind(document_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(chunk_from_row).collect()
    }

    pub(super) async fn list_document_summaries(
        &self,
        source_id: SourceId,
    ) -> Result<Vec<DocumentSummary>, StorageError> {
        let rows = sqlx::query(
            "SELECT d.id, d.source_id, d.path, d.mime_type, d.checksum, d.byte_size,
                    d.document_profile, d.extraction_quality, d.warnings,
                    COUNT(c.id)::INT AS chunk_count
             FROM documents d
             LEFT JOIN chunks c ON c.document_id = d.id
             WHERE d.source_id = $1
             GROUP BY d.id
             ORDER BY d.created_at DESC",
        )
        .bind(source_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let chunk_count = row.try_get::<i32, _>("chunk_count")?;
                Ok(DocumentSummary {
                    document: document_from_row(row)?,
                    chunk_count: as_u32(chunk_count, "chunk_count")?,
                })
            })
            .collect()
    }
}
