use rag_debugger_core::*;
use sqlx::types::Json;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn list_searchable_chunks(
        &self,
        request: &RetrievalQueryRequest,
    ) -> Result<Vec<SearchableChunk>, StorageError> {
        let source_ids = request
            .source_ids
            .iter()
            .map(|source_id| source_id.0)
            .collect::<Vec<_>>();
        let document_ids = request
            .document_ids
            .iter()
            .map(|document_id| document_id.0)
            .collect::<Vec<_>>();

        let rows = sqlx::query(
            "SELECT
                s.id AS source_id, s.project_id, s.name AS source_name,
                s.source_kind, s.root_hint, s.github_owner, s.github_repo,
                s.sync_policy, s.sync_cron, s.target_tokens, s.overlap_tokens,
                s.chunking_strategy,
                d.id AS document_id, d.source_id AS document_source_id,
                d.path AS document_path, d.mime_type, d.checksum AS document_checksum,
                d.byte_size, d.document_profile, d.extraction_quality, d.warnings,
                c.id AS chunk_id, c.source_id AS chunk_source_id,
                c.document_id AS chunk_document_id, c.ordinal, c.text,
                c.token_count, c.byte_start, c.byte_end,
                c.checksum AS chunk_checksum, c.strategy, c.section_title,
                c.split_reason, c.quality_flags, c.is_duplicate,
                c.text_density, c.evidence_score_hint,
                e.model_provider AS embedding_model_provider,
                e.model_name AS embedding_model_name,
                e.dimension AS embedding_dimension,
                e.vector AS embedding_vector,
                e.chunk_checksum AS embedding_chunk_checksum,
                e.indexed_at AS embedding_indexed_at
             FROM chunks c
             INNER JOIN documents d ON d.id = c.document_id
             INNER JOIN sources s ON s.id = c.source_id
             LEFT JOIN chunk_embeddings e ON e.chunk_id = c.id
             WHERE ($1 OR c.source_id = ANY($2))
               AND ($3 OR c.document_id = ANY($4))
             ORDER BY d.path ASC, c.ordinal ASC",
        )
        .bind(source_ids.is_empty())
        .bind(source_ids)
        .bind(document_ids.is_empty())
        .bind(document_ids)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(searchable_chunk_from_row).collect()
    }

    pub(super) async fn save_retrieval_query(
        &self,
        response: &RetrievalQueryResponse,
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO retrieval_playground_runs (
                id, query, top_k, retrieval_mode, answer_status, answer_text, latency_ms, created_at,
                response_json
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(response.run.id.0)
        .bind(&response.run.query)
        .bind(response.run.top_k as i32)
        .bind(retrieval_mode_to_str(response.run.retrieval_mode))
        .bind(answer_status_to_str(response.answer.status))
        .bind(&response.answer.text)
        .bind(response.run.latency_ms as i64)
        .bind(response.run.created_at)
        .bind(Json(response))
        .execute(&mut *transaction)
        .await?;

        for hit in &response.hits {
            sqlx::query(
                "INSERT INTO retrieval_playground_hits (
                    id, run_id, chunk_id, rank, score, lexical_score,
                    semantic_score, phrase_score, section_score, path_score, metadata_score,
                    matched_terms, snippet, citation_label, created_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            )
            .bind(Uuid::now_v7())
            .bind(response.run.id.0)
            .bind(hit.chunk.id.0)
            .bind(hit.rank as i32)
            .bind(hit.score)
            .bind(hit.score_breakdown.lexical)
            .bind(hit.score_breakdown.semantic)
            .bind(hit.score_breakdown.phrase)
            .bind(hit.score_breakdown.section)
            .bind(hit.score_breakdown.path)
            .bind(hit.score_breakdown.metadata)
            .bind(matched_terms_to_text(&hit.matched_terms))
            .bind(&hit.snippet)
            .bind(&hit.citation.label)
            .bind(OffsetDateTime::now_utc())
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub(super) async fn get_retrieval_query(
        &self,
        id: RetrievalQueryRunId,
    ) -> Result<RetrievalQueryResponse, StorageError> {
        let row = sqlx::query(
            "SELECT response_json
             FROM retrieval_playground_runs
             WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        retrieval_response_from_row(&row)
    }

    pub(super) async fn latest_retrieval_query(
        &self,
    ) -> Result<RetrievalQueryResponse, StorageError> {
        let row = sqlx::query(
            "SELECT response_json
             FROM retrieval_playground_runs
             WHERE response_json IS NOT NULL
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        retrieval_response_from_row(&row)
    }
}
