use std::path::Path;

use async_trait::async_trait;
use rag_debugger_core::{
    ByteRange, Chunk, ChunkEmbedding, ChunkId, ChunkQualityFlag, ChunkSplitReason, ChunkingConfig,
    ChunkingStrategy, Document, DocumentId, DocumentProfile, DocumentSummary, DocumentWarning,
    EmbeddingIndexCandidate, EmbeddingIndexRequest, EmbeddingModelInfo, EmbeddingStatus,
    EvidenceStrength, ExtractionQuality, ExtractiveAnswerStatus, FailureLabel, IngestionRun,
    IngestionRunId, IngestionRunStatus, IngestionTotals, PrivacyMode, Project, ProjectId,
    RetrievalEvalCase, RetrievalEvalCaseId, RetrievalEvalDataset, RetrievalEvalDatasetId,
    RetrievalEvalDatasetSummary, RetrievalEvalExperiment, RetrievalEvalExperimentId,
    RetrievalEvalGateStatus, RetrievalEvalResult, RetrievalEvalRun, RetrievalEvalRunId,
    RetrievalMatchedTerm, RetrievalMode, RetrievalQueryRequest, RetrievalQueryResponse,
    RetrievalQueryRunId, SearchableChunk, Source, SourceId, SourceKind, SourceSummary,
    SourceSyncPolicy, Trace, TraceId, TraceStatus, TraceSummary,
};
use sqlx::{postgres::PgPoolOptions, types::Json, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{repository::IngestionRepository, StorageError};

#[derive(Debug, Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn run_migrations(&self, migrations_path: &Path) -> Result<(), StorageError> {
        let migrator = sqlx::migrate::Migrator::new(migrations_path).await?;
        migrator.run(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl IngestionRepository for PostgresStore {
    async fn ping(&self) -> Result<(), StorageError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    async fn ensure_default_project(&self) -> Result<Project, StorageError> {
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

    async fn create_source(&self, source: Source) -> Result<Source, StorageError> {
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

    async fn create_ingestion_run(&self, run: IngestionRun) -> Result<IngestionRun, StorageError> {
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

    async fn complete_ingestion_run(
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

    async fn insert_document_with_chunks(
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

    async fn list_sources(&self) -> Result<Vec<SourceSummary>, StorageError> {
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

    async fn list_document_chunks(
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

    async fn list_searchable_chunks(
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

    async fn save_retrieval_query(
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

    async fn get_retrieval_query(
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

    async fn latest_retrieval_query(&self) -> Result<RetrievalQueryResponse, StorageError> {
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

    async fn save_trace(&self, trace: Trace) -> Result<Trace, StorageError> {
        let mut transaction = self.pool.begin().await?;
        let now = OffsetDateTime::now_utc();
        let retrieval = trace.retrieval.as_ref();
        let retrieval_mode = retrieval
            .map(|response| response.run.retrieval_mode)
            .unwrap_or_default();
        let latency_ms = retrieval.map_or(0, |response| response.run.latency_ms);
        let evidence_strength = trace
            .evidence_strength
            .or_else(|| {
                retrieval
                    .and_then(|response| response.hits.first().map(|hit| hit.evidence_strength))
            })
            .unwrap_or(EvidenceStrength::Weak);

        sqlx::query(
            "INSERT INTO debug_traces (
                id, project_id, source_run_id, query, retrieval_mode, summary, status,
                evidence_strength, failure_labels, span_count, rerun_count, latency_ms,
                trace_json, created_at, updated_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (id) DO UPDATE SET
                source_run_id = EXCLUDED.source_run_id,
                query = EXCLUDED.query,
                retrieval_mode = EXCLUDED.retrieval_mode,
                summary = EXCLUDED.summary,
                status = EXCLUDED.status,
                evidence_strength = EXCLUDED.evidence_strength,
                failure_labels = EXCLUDED.failure_labels,
                span_count = EXCLUDED.span_count,
                rerun_count = EXCLUDED.rerun_count,
                latency_ms = EXCLUDED.latency_ms,
                trace_json = EXCLUDED.trace_json,
                updated_at = EXCLUDED.updated_at",
        )
        .bind(trace.id.0)
        .bind(trace.project_id.0)
        .bind(trace.source_run_id.map(|id| id.0))
        .bind(&trace.input)
        .bind(retrieval_mode_to_str(retrieval_mode))
        .bind(&trace.summary)
        .bind(trace_status_to_str(trace.status))
        .bind(evidence_strength_to_str(evidence_strength))
        .bind(failure_labels_to_text(&trace.failure_labels))
        .bind(trace.spans.len() as i32)
        .bind(trace.reruns.len() as i32)
        .bind(latency_ms as i64)
        .bind(Json(&trace))
        .bind(trace.started_at)
        .bind(now)
        .execute(&mut *transaction)
        .await?;

        sqlx::query("DELETE FROM trace_rerun_experiments WHERE trace_id = $1")
            .bind(trace.id.0)
            .execute(&mut *transaction)
            .await?;

        for comparison in &trace.reruns {
            sqlx::query(
                "INSERT INTO trace_rerun_experiments (
                    id, trace_id, retrieval_mode, top_k, score_delta, latency_delta_ms,
                    overlap_count, changed_rank_count, comparison_json, created_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(comparison.id.0)
            .bind(trace.id.0)
            .bind(retrieval_mode_to_str(
                comparison.response.run.retrieval_mode,
            ))
            .bind(comparison.response.run.top_k as i32)
            .bind(comparison.score_delta)
            .bind(comparison.latency_delta_ms)
            .bind(comparison.overlap_count as i32)
            .bind(comparison.changed_rank_count as i32)
            .bind(Json(comparison))
            .bind(comparison.created_at)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(trace)
    }

    async fn list_traces(&self) -> Result<Vec<TraceSummary>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, query, retrieval_mode, latency_ms, evidence_strength,
                    failure_labels, span_count, rerun_count, created_at
             FROM debug_traces
             ORDER BY created_at DESC
             LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(trace_summary_from_row).collect()
    }

    async fn get_trace_detail(&self, id: TraceId) -> Result<Trace, StorageError> {
        let row = sqlx::query(
            "SELECT trace_json
             FROM debug_traces
             WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        trace_from_row(&row)
    }

    async fn embedding_status(
        &self,
        request: &EmbeddingIndexRequest,
        model: &EmbeddingModelInfo,
    ) -> Result<EmbeddingStatus, StorageError> {
        let source_ids = source_filter_ids(&request.source_ids);
        let document_ids = document_filter_ids(&request.document_ids);
        let row = sqlx::query(
            "SELECT
                COUNT(*)::INT AS total_chunks,
                (COUNT(*) FILTER (
                    WHERE e.chunk_id IS NOT NULL
                      AND e.model_provider = $5
                      AND e.model_name = $6
                      AND e.dimension = $7
                      AND e.chunk_checksum = c.checksum
                ))::INT AS indexed_chunks,
                (COUNT(*) FILTER (WHERE e.chunk_id IS NULL))::INT AS missing_chunks,
                (COUNT(*) FILTER (
                    WHERE e.chunk_id IS NOT NULL
                      AND (
                        e.model_provider <> $5
                        OR e.model_name <> $6
                        OR e.dimension <> $7
                        OR e.chunk_checksum <> c.checksum
                      )
                ))::INT AS stale_chunks,
                MAX(e.indexed_at) FILTER (
                    WHERE e.chunk_id IS NOT NULL
                      AND e.model_provider = $5
                      AND e.model_name = $6
                      AND e.dimension = $7
                      AND e.chunk_checksum = c.checksum
                ) AS last_indexed_at
             FROM chunks c
             LEFT JOIN chunk_embeddings e ON e.chunk_id = c.id
             WHERE ($1 OR c.source_id = ANY($2))
               AND ($3 OR c.document_id = ANY($4))",
        )
        .bind(source_ids.is_empty())
        .bind(source_ids)
        .bind(document_ids.is_empty())
        .bind(document_ids)
        .bind(&model.provider)
        .bind(&model.model_name)
        .bind(model.dimension as i32)
        .fetch_one(&self.pool)
        .await?;

        Ok(EmbeddingStatus {
            model: model.clone(),
            total_chunks: as_u32(row.try_get("total_chunks")?, "total_chunks")?,
            indexed_chunks: as_u32(row.try_get("indexed_chunks")?, "indexed_chunks")?,
            missing_chunks: as_u32(row.try_get("missing_chunks")?, "missing_chunks")?,
            stale_chunks: as_u32(row.try_get("stale_chunks")?, "stale_chunks")?,
            last_indexed_at: row.try_get("last_indexed_at")?,
        })
    }

    async fn list_embedding_candidates(
        &self,
        request: &EmbeddingIndexRequest,
    ) -> Result<Vec<EmbeddingIndexCandidate>, StorageError> {
        let source_ids = source_filter_ids(&request.source_ids);
        let document_ids = document_filter_ids(&request.document_ids);
        let rows = sqlx::query(
            "SELECT c.id, c.source_id, c.document_id, c.text, c.checksum, c.strategy
             FROM chunks c
             WHERE ($1 OR c.source_id = ANY($2))
               AND ($3 OR c.document_id = ANY($4))
             ORDER BY c.document_id ASC, c.ordinal ASC",
        )
        .bind(source_ids.is_empty())
        .bind(source_ids)
        .bind(document_ids.is_empty())
        .bind(document_ids)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                Ok(EmbeddingIndexCandidate {
                    chunk_id: ChunkId(row.try_get("id")?),
                    source_id: SourceId(row.try_get("source_id")?),
                    document_id: DocumentId(row.try_get("document_id")?),
                    text: row.try_get("text")?,
                    checksum: row.try_get("checksum")?,
                    chunking_strategy: chunking_strategy_from_str(
                        row.try_get::<String, _>("strategy")?.as_str(),
                    )?,
                })
            })
            .collect()
    }

    async fn upsert_chunk_embeddings(
        &self,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;

        for embedding in embeddings {
            sqlx::query(
                "INSERT INTO chunk_embeddings (
                    chunk_id, model_provider, model_name, dimension, vector, chunk_checksum, indexed_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (chunk_id) DO UPDATE SET
                    model_provider = EXCLUDED.model_provider,
                    model_name = EXCLUDED.model_name,
                    dimension = EXCLUDED.dimension,
                    vector = EXCLUDED.vector,
                    chunk_checksum = EXCLUDED.chunk_checksum,
                    indexed_at = EXCLUDED.indexed_at",
            )
            .bind(embedding.chunk_id.0)
            .bind(&embedding.model.provider)
            .bind(&embedding.model.model_name)
            .bind(embedding.model.dimension as i32)
            .bind(&embedding.vector)
            .bind(&embedding.chunk_checksum)
            .bind(embedding.indexed_at)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    async fn create_retrieval_eval_case(
        &self,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        ensure_default_eval_dataset(&self.pool).await?;
        let expected_chunk_ids = eval_case
            .expected_chunk_ids
            .iter()
            .map(|chunk_id| chunk_id.0)
            .collect::<Vec<_>>();
        let expected_document_ids = eval_case
            .expected_document_ids
            .iter()
            .map(|document_id| document_id.0)
            .collect::<Vec<_>>();

        sqlx::query(
            "INSERT INTO retrieval_eval_cases (
                id, dataset_id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes, created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(eval_case.id.0)
        .bind(default_eval_dataset_id().0)
        .bind(&eval_case.name)
        .bind(&eval_case.query)
        .bind(eval_case.top_k as i32)
        .bind(expected_chunk_ids)
        .bind(expected_document_ids)
        .bind(&eval_case.notes)
        .bind(eval_case.created_at)
        .execute(&self.pool)
        .await?;

        Ok(eval_case)
    }

    async fn list_retrieval_eval_cases(&self) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes, created_at
             FROM retrieval_eval_cases
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(retrieval_eval_case_from_row).collect()
    }

    async fn list_retrieval_eval_cases_by_id(
        &self,
        case_ids: &[RetrievalEvalCaseId],
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        let ids = case_ids.iter().map(|case_id| case_id.0).collect::<Vec<_>>();
        let rows = sqlx::query(
            "SELECT id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes, created_at
             FROM retrieval_eval_cases
             WHERE id = ANY($1)
             ORDER BY created_at DESC",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(retrieval_eval_case_from_row).collect()
    }

    async fn save_retrieval_eval_run(
        &self,
        eval_run: &RetrievalEvalRun,
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO retrieval_eval_runs (
                id, retrieval_mode, case_count, passed_count,
                average_recall_at_k, average_precision_at_k, created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(eval_run.id.0)
        .bind(retrieval_mode_to_str(eval_run.retrieval_mode))
        .bind(eval_run.case_count as i32)
        .bind(eval_run.passed_count as i32)
        .bind(eval_run.average_recall_at_k)
        .bind(eval_run.average_precision_at_k)
        .bind(eval_run.created_at)
        .execute(&mut *transaction)
        .await?;

        for result in &eval_run.results {
            let expected_chunk_ids = result
                .expected_chunk_ids
                .iter()
                .map(|chunk_id| chunk_id.0)
                .collect::<Vec<_>>();
            let expected_document_ids = result
                .expected_document_ids
                .iter()
                .map(|document_id| document_id.0)
                .collect::<Vec<_>>();
            let retrieved_chunk_ids = result
                .retrieved_chunk_ids
                .iter()
                .map(|chunk_id| chunk_id.0)
                .collect::<Vec<_>>();

            sqlx::query(
                "INSERT INTO retrieval_eval_results (
                    id, run_id, case_id, query, top_k, recall_at_k, precision_at_k,
                    top_hit_rank, passed, expected_chunk_ids, expected_document_ids,
                    retrieved_chunk_ids, latency_ms, created_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            )
            .bind(Uuid::now_v7())
            .bind(eval_run.id.0)
            .bind(result.case_id.0)
            .bind(&result.query)
            .bind(result.top_k as i32)
            .bind(result.recall_at_k)
            .bind(result.precision_at_k)
            .bind(result.top_hit_rank.map(|rank| rank as i32))
            .bind(result.passed)
            .bind(expected_chunk_ids)
            .bind(expected_document_ids)
            .bind(retrieved_chunk_ids)
            .bind(result.latency_ms as i64)
            .bind(eval_run.created_at)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    async fn latest_retrieval_eval_run(&self) -> Result<Option<RetrievalEvalRun>, StorageError> {
        let Some(row) = sqlx::query(
            "SELECT id, retrieval_mode, case_count, passed_count,
                    average_recall_at_k, average_precision_at_k, created_at
             FROM retrieval_eval_runs
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        let run_id = RetrievalEvalRunId(row.try_get("id")?);
        let result_rows = sqlx::query(
            "SELECT case_id, query, top_k, recall_at_k, precision_at_k,
                    top_hit_rank, passed, expected_chunk_ids, expected_document_ids,
                    retrieved_chunk_ids, latency_ms
             FROM retrieval_eval_results
             WHERE run_id = $1
             ORDER BY created_at ASC",
        )
        .bind(run_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(RetrievalEvalRun {
            id: run_id,
            retrieval_mode: retrieval_mode_from_str(
                row.try_get::<String, _>("retrieval_mode")?.as_str(),
            )?,
            case_count: as_u32(row.try_get("case_count")?, "case_count")?,
            passed_count: as_u32(row.try_get("passed_count")?, "passed_count")?,
            average_recall_at_k: row.try_get("average_recall_at_k")?,
            average_precision_at_k: row.try_get("average_precision_at_k")?,
            created_at: row.try_get("created_at")?,
            results: result_rows
                .iter()
                .map(retrieval_eval_result_from_row)
                .collect::<Result<Vec<_>, _>>()?,
        }))
    }

    async fn create_retrieval_eval_dataset(
        &self,
        dataset: RetrievalEvalDataset,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        sqlx::query(
            "INSERT INTO retrieval_eval_datasets (id, name, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(dataset.id.0)
        .bind(&dataset.name)
        .bind(&dataset.description)
        .bind(dataset.created_at)
        .bind(dataset.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(dataset)
    }

    async fn list_retrieval_eval_datasets(
        &self,
    ) -> Result<Vec<RetrievalEvalDatasetSummary>, StorageError> {
        ensure_default_eval_dataset(&self.pool).await?;
        let rows = sqlx::query(
            "SELECT d.id, d.name, d.description, d.updated_at,
                    COUNT(c.id)::INT AS case_count,
                    e.experiment_json AS latest_experiment_json
             FROM retrieval_eval_datasets d
             LEFT JOIN retrieval_eval_cases c ON c.dataset_id = d.id
             LEFT JOIN LATERAL (
                SELECT experiment_json
                FROM retrieval_eval_experiments
                WHERE dataset_id = d.id
                ORDER BY created_at DESC
                LIMIT 1
             ) e ON TRUE
             GROUP BY d.id, e.experiment_json
             ORDER BY d.updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(eval_dataset_summary_from_row).collect()
    }

    async fn get_retrieval_eval_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        ensure_default_eval_dataset(&self.pool).await?;
        let row = sqlx::query(
            "SELECT id, name, description, created_at, updated_at
             FROM retrieval_eval_datasets
             WHERE id = $1",
        )
        .bind(dataset_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        let case_rows = sqlx::query(
            "SELECT id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes, created_at
             FROM retrieval_eval_cases
             WHERE dataset_id = $1
             ORDER BY created_at DESC",
        )
        .bind(dataset_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(RetrievalEvalDataset {
            id: RetrievalEvalDatasetId(row.try_get("id")?),
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            cases: case_rows
                .iter()
                .map(retrieval_eval_case_from_row)
                .collect::<Result<Vec<_>, _>>()?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    async fn create_retrieval_eval_case_in_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let expected_chunk_ids = eval_case
            .expected_chunk_ids
            .iter()
            .map(|chunk_id| chunk_id.0)
            .collect::<Vec<_>>();
        let expected_document_ids = eval_case
            .expected_document_ids
            .iter()
            .map(|document_id| document_id.0)
            .collect::<Vec<_>>();
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO retrieval_eval_cases (
                id, dataset_id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes, created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(eval_case.id.0)
        .bind(dataset_id.0)
        .bind(&eval_case.name)
        .bind(&eval_case.query)
        .bind(eval_case.top_k as i32)
        .bind(expected_chunk_ids)
        .bind(expected_document_ids)
        .bind(&eval_case.notes)
        .bind(eval_case.created_at)
        .execute(&mut *transaction)
        .await?;

        sqlx::query("UPDATE retrieval_eval_datasets SET updated_at = $1 WHERE id = $2")
            .bind(OffsetDateTime::now_utc())
            .bind(dataset_id.0)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(eval_case)
    }

    async fn update_retrieval_eval_case(
        &self,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let expected_chunk_ids = eval_case
            .expected_chunk_ids
            .iter()
            .map(|chunk_id| chunk_id.0)
            .collect::<Vec<_>>();
        let expected_document_ids = eval_case
            .expected_document_ids
            .iter()
            .map(|document_id| document_id.0)
            .collect::<Vec<_>>();
        let row = sqlx::query(
            "UPDATE retrieval_eval_cases
             SET name = $2, query = $3, top_k = $4, expected_chunk_ids = $5,
                 expected_document_ids = $6, notes = $7
             WHERE id = $1
             RETURNING dataset_id",
        )
        .bind(eval_case.id.0)
        .bind(&eval_case.name)
        .bind(&eval_case.query)
        .bind(eval_case.top_k as i32)
        .bind(expected_chunk_ids)
        .bind(expected_document_ids)
        .bind(&eval_case.notes)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        if let Some(dataset_id) = row.try_get::<Option<Uuid>, _>("dataset_id")? {
            sqlx::query("UPDATE retrieval_eval_datasets SET updated_at = $1 WHERE id = $2")
                .bind(OffsetDateTime::now_utc())
                .bind(dataset_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(eval_case)
    }

    async fn delete_retrieval_eval_case(
        &self,
        case_id: RetrievalEvalCaseId,
    ) -> Result<(), StorageError> {
        let row =
            sqlx::query("DELETE FROM retrieval_eval_cases WHERE id = $1 RETURNING dataset_id")
                .bind(case_id.0)
                .fetch_optional(&self.pool)
                .await?
                .ok_or(StorageError::NotFound)?;

        if let Some(dataset_id) = row.try_get::<Option<Uuid>, _>("dataset_id")? {
            sqlx::query("UPDATE retrieval_eval_datasets SET updated_at = $1 WHERE id = $2")
                .bind(OffsetDateTime::now_utc())
                .bind(dataset_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    async fn save_retrieval_eval_experiment(
        &self,
        experiment: RetrievalEvalExperiment,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        let best_mode = experiment.comparison.best_mode.map(retrieval_mode_to_str);
        let best_result = experiment.mode_results.iter().max_by(|left, right| {
            left.average_recall_at_k
                .partial_cmp(&right.average_recall_at_k)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let modes = experiment
            .modes
            .iter()
            .map(|mode| retrieval_mode_to_str(*mode).to_owned())
            .collect::<Vec<_>>();

        sqlx::query(
            "INSERT INTO retrieval_eval_experiments (
                id, dataset_id, name, modes, top_k, best_mode, gate_status,
                average_recall_at_k, average_precision_at_k, failure_count,
                experiment_json, created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(experiment.id.0)
        .bind(experiment.dataset_id.0)
        .bind(&experiment.name)
        .bind(modes)
        .bind(experiment.top_k as i32)
        .bind(best_mode)
        .bind(eval_gate_status_to_str(experiment.gate.status))
        .bind(best_result.map_or(0.0, |result| result.average_recall_at_k))
        .bind(best_result.map_or(0.0, |result| result.average_precision_at_k))
        .bind(experiment.failures.len() as i32)
        .bind(Json(&experiment))
        .bind(experiment.created_at)
        .execute(&self.pool)
        .await?;

        sqlx::query("UPDATE retrieval_eval_datasets SET updated_at = $1 WHERE id = $2")
            .bind(experiment.created_at)
            .bind(experiment.dataset_id.0)
            .execute(&self.pool)
            .await?;

        Ok(experiment)
    }

    async fn list_retrieval_eval_experiments(
        &self,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        let rows = sqlx::query(
            "SELECT experiment_json
             FROM retrieval_eval_experiments
             ORDER BY created_at DESC
             LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(eval_experiment_from_row).collect()
    }

    async fn get_retrieval_eval_experiment(
        &self,
        experiment_id: RetrievalEvalExperimentId,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        let row = sqlx::query(
            "SELECT experiment_json
             FROM retrieval_eval_experiments
             WHERE id = $1",
        )
        .bind(experiment_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        eval_experiment_from_row(&row)
    }

    async fn latest_retrieval_eval_experiment(
        &self,
    ) -> Result<Option<RetrievalEvalExperiment>, StorageError> {
        let Some(row) = sqlx::query(
            "SELECT experiment_json
             FROM retrieval_eval_experiments
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        Ok(Some(eval_experiment_from_row(&row)?))
    }
}

impl PostgresStore {
    async fn list_document_summaries(
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

fn project_from_row(row: &sqlx::postgres::PgRow) -> Result<Project, StorageError> {
    Ok(Project {
        id: ProjectId(row.try_get("id")?),
        name: row.try_get("name")?,
        privacy_mode: privacy_mode_from_str(row.try_get::<String, _>("privacy_mode")?.as_str())?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn source_from_row(row: &sqlx::postgres::PgRow) -> Result<Source, StorageError> {
    let source_kind = row.try_get::<String, _>("source_kind")?;
    let sync_policy = row.try_get::<String, _>("sync_policy")?;
    let target_tokens = row.try_get::<i32, _>("target_tokens")?;
    let overlap_tokens = row.try_get::<i32, _>("overlap_tokens")?;
    let chunking_strategy = row.try_get::<String, _>("chunking_strategy")?;

    Ok(Source {
        id: SourceId(row.try_get("id")?),
        project_id: ProjectId(row.try_get("project_id")?),
        name: row.try_get("name")?,
        kind: source_kind_from_columns(
            &source_kind,
            row.try_get("root_hint")?,
            row.try_get("github_owner")?,
            row.try_get("github_repo")?,
        )?,
        sync_policy: sync_policy_from_columns(&sync_policy, row.try_get("sync_cron")?)?,
        chunking: ChunkingConfig {
            target_tokens: as_u32(target_tokens, "target_tokens")?,
            overlap_tokens: as_u32(overlap_tokens, "overlap_tokens")?,
            strategy: chunking_strategy_from_str(&chunking_strategy)?,
        },
    })
}

fn document_from_row(row: &sqlx::postgres::PgRow) -> Result<Document, StorageError> {
    let byte_size = row.try_get::<i64, _>("byte_size")?;
    let warnings = row
        .try_get::<Vec<String>, _>("warnings")
        .unwrap_or_default();
    Ok(Document {
        id: DocumentId(row.try_get("id")?),
        source_id: SourceId(row.try_get("source_id")?),
        path: row.try_get("path")?,
        mime_type: row.try_get("mime_type")?,
        checksum: row.try_get("checksum")?,
        byte_size: as_u64(byte_size, "byte_size")?,
        profile: document_profile_from_str(
            row.try_get::<String, _>("document_profile")
                .unwrap_or_else(|_| "general".to_owned())
                .as_str(),
        )?,
        extraction_quality: extraction_quality_from_str(
            row.try_get::<String, _>("extraction_quality")
                .unwrap_or_else(|_| "unknown".to_owned())
                .as_str(),
        )?,
        warnings: document_warnings_from_text(warnings),
    })
}

fn chunk_from_row(row: &sqlx::postgres::PgRow) -> Result<Chunk, StorageError> {
    let ordinal = row.try_get::<i32, _>("ordinal")?;
    let token_count = row.try_get::<i32, _>("token_count")?;
    let byte_start = row.try_get::<i64, _>("byte_start")?;
    let byte_end = row.try_get::<i64, _>("byte_end")?;
    Ok(Chunk {
        id: ChunkId(row.try_get("id")?),
        source_id: SourceId(row.try_get("source_id")?),
        document_id: DocumentId(row.try_get("document_id")?),
        ordinal: as_u32(ordinal, "ordinal")?,
        text: row.try_get("text")?,
        token_count: as_u32(token_count, "token_count")?,
        byte_range: ByteRange {
            start: as_u64(byte_start, "byte_start")?,
            end: as_u64(byte_end, "byte_end")?,
        },
        checksum: row.try_get("checksum")?,
        strategy: chunking_strategy_from_str(row.try_get::<String, _>("strategy")?.as_str())?,
        section_title: row.try_get("section_title")?,
        split_reason: chunk_split_reason_from_str(
            row.try_get::<String, _>("split_reason")?.as_str(),
        )?,
        quality_flags: chunk_quality_flags_from_text(
            row.try_get::<Vec<String>, _>("quality_flags")
                .unwrap_or_default(),
        )?,
        is_duplicate: row.try_get("is_duplicate").unwrap_or(false),
        text_density: row.try_get("text_density").unwrap_or(0.0),
        evidence_score_hint: row.try_get("evidence_score_hint").unwrap_or(0.0),
    })
}

fn searchable_chunk_from_row(row: &sqlx::postgres::PgRow) -> Result<SearchableChunk, StorageError> {
    let source_kind = row.try_get::<String, _>("source_kind")?;
    let sync_policy = row.try_get::<String, _>("sync_policy")?;
    let target_tokens = row.try_get::<i32, _>("target_tokens")?;
    let overlap_tokens = row.try_get::<i32, _>("overlap_tokens")?;
    let chunking_strategy = row.try_get::<String, _>("chunking_strategy")?;
    let byte_size = row.try_get::<i64, _>("byte_size")?;
    let ordinal = row.try_get::<i32, _>("ordinal")?;
    let token_count = row.try_get::<i32, _>("token_count")?;
    let byte_start = row.try_get::<i64, _>("byte_start")?;
    let byte_end = row.try_get::<i64, _>("byte_end")?;
    let document_warnings = row
        .try_get::<Vec<String>, _>("warnings")
        .unwrap_or_default();

    let source = Source {
        id: SourceId(row.try_get("source_id")?),
        project_id: ProjectId(row.try_get("project_id")?),
        name: row.try_get("source_name")?,
        kind: source_kind_from_columns(
            &source_kind,
            row.try_get("root_hint")?,
            row.try_get("github_owner")?,
            row.try_get("github_repo")?,
        )?,
        sync_policy: sync_policy_from_columns(&sync_policy, row.try_get("sync_cron")?)?,
        chunking: ChunkingConfig {
            target_tokens: as_u32(target_tokens, "target_tokens")?,
            overlap_tokens: as_u32(overlap_tokens, "overlap_tokens")?,
            strategy: chunking_strategy_from_str(&chunking_strategy)?,
        },
    };
    let document = Document {
        id: DocumentId(row.try_get("document_id")?),
        source_id: SourceId(row.try_get("document_source_id")?),
        path: row.try_get("document_path")?,
        mime_type: row.try_get("mime_type")?,
        checksum: row.try_get("document_checksum")?,
        byte_size: as_u64(byte_size, "byte_size")?,
        profile: document_profile_from_str(
            row.try_get::<String, _>("document_profile")
                .unwrap_or_else(|_| "general".to_owned())
                .as_str(),
        )?,
        extraction_quality: extraction_quality_from_str(
            row.try_get::<String, _>("extraction_quality")
                .unwrap_or_else(|_| "unknown".to_owned())
                .as_str(),
        )?,
        warnings: document_warnings_from_text(document_warnings),
    };
    let chunk = Chunk {
        id: ChunkId(row.try_get("chunk_id")?),
        source_id: SourceId(row.try_get("chunk_source_id")?),
        document_id: DocumentId(row.try_get("chunk_document_id")?),
        ordinal: as_u32(ordinal, "ordinal")?,
        text: row.try_get("text")?,
        token_count: as_u32(token_count, "token_count")?,
        byte_range: ByteRange {
            start: as_u64(byte_start, "byte_start")?,
            end: as_u64(byte_end, "byte_end")?,
        },
        checksum: row.try_get("chunk_checksum")?,
        strategy: chunking_strategy_from_str(row.try_get::<String, _>("strategy")?.as_str())?,
        section_title: row.try_get("section_title")?,
        split_reason: chunk_split_reason_from_str(
            row.try_get::<String, _>("split_reason")?.as_str(),
        )?,
        quality_flags: chunk_quality_flags_from_text(
            row.try_get::<Vec<String>, _>("quality_flags")
                .unwrap_or_default(),
        )?,
        is_duplicate: row.try_get("is_duplicate").unwrap_or(false),
        text_density: row.try_get("text_density").unwrap_or(0.0),
        evidence_score_hint: row.try_get("evidence_score_hint").unwrap_or(0.0),
    };
    let embedding = chunk_embedding_from_row(row)?;

    Ok(SearchableChunk {
        source,
        document,
        chunk,
        embedding,
    })
}

fn chunk_embedding_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<ChunkEmbedding>, StorageError> {
    let Some(model_provider) = row.try_get::<Option<String>, _>("embedding_model_provider")? else {
        return Ok(None);
    };
    let model_name = row
        .try_get::<Option<String>, _>("embedding_model_name")?
        .ok_or_else(|| StorageError::InvalidData("embedding model name is missing".to_owned()))?;
    let dimension = row
        .try_get::<Option<i32>, _>("embedding_dimension")?
        .ok_or_else(|| StorageError::InvalidData("embedding dimension is missing".to_owned()))?;
    let vector = row
        .try_get::<Option<Vec<f32>>, _>("embedding_vector")?
        .ok_or_else(|| StorageError::InvalidData("embedding vector is missing".to_owned()))?;
    let chunk_checksum = row
        .try_get::<Option<String>, _>("embedding_chunk_checksum")?
        .ok_or_else(|| StorageError::InvalidData("embedding checksum is missing".to_owned()))?;
    let indexed_at = row
        .try_get::<Option<OffsetDateTime>, _>("embedding_indexed_at")?
        .ok_or_else(|| StorageError::InvalidData("embedding indexed_at is missing".to_owned()))?;

    Ok(Some(ChunkEmbedding {
        chunk_id: ChunkId(row.try_get("chunk_id")?),
        chunk_checksum,
        model: EmbeddingModelInfo {
            provider: model_provider,
            model_name,
            dimension: as_u32(dimension, "embedding_dimension")?,
        },
        vector,
        indexed_at,
    }))
}

fn retrieval_eval_case_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalCase, StorageError> {
    let top_k = row.try_get::<i32, _>("top_k")?;
    let expected_chunk_ids = row
        .try_get::<Vec<Uuid>, _>("expected_chunk_ids")?
        .into_iter()
        .map(ChunkId)
        .collect();
    let expected_document_ids = row
        .try_get::<Vec<Uuid>, _>("expected_document_ids")?
        .into_iter()
        .map(DocumentId)
        .collect();

    Ok(RetrievalEvalCase {
        id: RetrievalEvalCaseId(row.try_get("id")?),
        name: row.try_get("name")?,
        query: row.try_get("query")?,
        top_k: as_u32(top_k, "top_k")?,
        expected_chunk_ids,
        expected_document_ids,
        notes: row.try_get("notes")?,
        created_at: row.try_get("created_at")?,
    })
}

fn retrieval_eval_result_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalResult, StorageError> {
    let expected_chunk_ids = row
        .try_get::<Vec<Uuid>, _>("expected_chunk_ids")?
        .into_iter()
        .map(ChunkId)
        .collect();
    let expected_document_ids = row
        .try_get::<Vec<Uuid>, _>("expected_document_ids")?
        .into_iter()
        .map(DocumentId)
        .collect();
    let retrieved_chunk_ids = row
        .try_get::<Vec<Uuid>, _>("retrieved_chunk_ids")?
        .into_iter()
        .map(ChunkId)
        .collect();

    Ok(RetrievalEvalResult {
        case_id: RetrievalEvalCaseId(row.try_get("case_id")?),
        query: row.try_get("query")?,
        top_k: as_u32(row.try_get("top_k")?, "top_k")?,
        recall_at_k: row.try_get("recall_at_k")?,
        precision_at_k: row.try_get("precision_at_k")?,
        top_hit_rank: row
            .try_get::<Option<i32>, _>("top_hit_rank")?
            .map(|rank| as_u32(rank, "top_hit_rank"))
            .transpose()?,
        passed: row.try_get("passed")?,
        expected_chunk_ids,
        expected_document_ids,
        retrieved_chunk_ids,
        latency_ms: as_u64(row.try_get("latency_ms")?, "latency_ms")?,
    })
}

fn eval_dataset_summary_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalDatasetSummary, StorageError> {
    let case_count = row.try_get::<i32, _>("case_count")?;
    let latest_experiment = row
        .try_get::<Option<Json<RetrievalEvalExperiment>>, _>("latest_experiment_json")?
        .map(|json| json.0);
    let best_result = latest_experiment.as_ref().and_then(|experiment| {
        experiment.mode_results.iter().max_by(|left, right| {
            left.average_recall_at_k
                .partial_cmp(&right.average_recall_at_k)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    Ok(RetrievalEvalDatasetSummary {
        id: RetrievalEvalDatasetId(row.try_get("id")?),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        case_count: as_u32(case_count, "case_count")?,
        latest_experiment_id: latest_experiment.as_ref().map(|experiment| experiment.id),
        latest_gate: latest_experiment
            .as_ref()
            .map(|experiment| experiment.gate.clone()),
        latest_average_recall_at_k: best_result.map(|result| result.average_recall_at_k),
        latest_average_precision_at_k: best_result.map(|result| result.average_precision_at_k),
        updated_at: row.try_get("updated_at")?,
    })
}

fn eval_experiment_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalExperiment, StorageError> {
    Ok(row
        .try_get::<Json<RetrievalEvalExperiment>, _>("experiment_json")?
        .0)
}

fn default_eval_dataset_id() -> RetrievalEvalDatasetId {
    RetrievalEvalDatasetId(Uuid::from_u128(0x018f_7a2a_6e2e_7000_a000_0000_0000_e001))
}

async fn ensure_default_eval_dataset(pool: &PgPool) -> Result<(), StorageError> {
    let now = OffsetDateTime::now_utc();
    sqlx::query(
        "INSERT INTO retrieval_eval_datasets (id, name, description, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(default_eval_dataset_id().0)
    .bind("Default retrieval dataset")
    .bind("Backfilled and manually saved retrieval eval cases.")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE retrieval_eval_cases
         SET dataset_id = $1
         WHERE dataset_id IS NULL",
    )
    .bind(default_eval_dataset_id().0)
    .execute(pool)
    .await?;

    Ok(())
}

fn retrieval_response_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalQueryResponse, StorageError> {
    let response = row
        .try_get::<Option<Json<RetrievalQueryResponse>>, _>("response_json")?
        .ok_or_else(|| {
            StorageError::InvalidData(
                "retrieval response JSON was not stored for this run".to_owned(),
            )
        })?;
    Ok(response.0)
}

fn trace_from_row(row: &sqlx::postgres::PgRow) -> Result<Trace, StorageError> {
    let trace = row.try_get::<Json<Trace>, _>("trace_json")?;
    Ok(trace.0)
}

fn trace_summary_from_row(row: &sqlx::postgres::PgRow) -> Result<TraceSummary, StorageError> {
    Ok(TraceSummary {
        id: TraceId(row.try_get("id")?),
        query: row.try_get("query")?,
        retrieval_mode: retrieval_mode_from_str(
            row.try_get::<String, _>("retrieval_mode")?.as_str(),
        )?,
        latency_ms: as_u64(row.try_get("latency_ms")?, "latency_ms")?,
        evidence_strength: evidence_strength_from_str(
            row.try_get::<String, _>("evidence_strength")?.as_str(),
        )?,
        failure_labels: failure_labels_from_text(
            row.try_get::<Vec<String>, _>("failure_labels")
                .unwrap_or_default(),
        )?,
        span_count: as_u32(row.try_get("span_count")?, "span_count")?,
        rerun_count: as_u32(row.try_get("rerun_count")?, "rerun_count")?,
        created_at: row.try_get("created_at")?,
    })
}

fn ingestion_run_from_row(row: &sqlx::postgres::PgRow) -> Result<IngestionRun, StorageError> {
    Ok(IngestionRun {
        id: IngestionRunId(row.try_get("id")?),
        source_id: SourceId(row.try_get("source_id")?),
        status: ingestion_status_from_str(row.try_get::<String, _>("status")?.as_str())?,
        totals: IngestionTotals {
            files_received: as_u32(row.try_get("files_received")?, "files_received")?,
            documents_created: as_u32(row.try_get("documents_created")?, "documents_created")?,
            chunks_created: as_u32(row.try_get("chunks_created")?, "chunks_created")?,
            failed_files: as_u32(row.try_get("failed_files")?, "failed_files")?,
        },
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
    })
}

fn source_kind_columns(
    kind: &SourceKind,
) -> (&'static str, Option<String>, Option<String>, Option<String>) {
    match kind {
        SourceKind::FileSet { root_hint } => ("file_set", Some(root_hint.clone()), None, None),
        SourceKind::GitHubRepository { owner, repo } => (
            "github_repository",
            None,
            Some(owner.clone()),
            Some(repo.clone()),
        ),
    }
}

fn source_kind_from_columns(
    kind: &str,
    root_hint: Option<String>,
    github_owner: Option<String>,
    github_repo: Option<String>,
) -> Result<SourceKind, StorageError> {
    match kind {
        "file_set" => Ok(SourceKind::FileSet {
            root_hint: root_hint.unwrap_or_default(),
        }),
        "github_repository" => Ok(SourceKind::GitHubRepository {
            owner: github_owner.unwrap_or_default(),
            repo: github_repo.unwrap_or_default(),
        }),
        _ => Err(StorageError::InvalidData(format!(
            "unknown source kind: {kind}"
        ))),
    }
}

fn sync_policy_columns(policy: &SourceSyncPolicy) -> (&'static str, Option<String>) {
    match policy {
        SourceSyncPolicy::Manual => ("manual", None),
        SourceSyncPolicy::OnDemand => ("on_demand", None),
        SourceSyncPolicy::Scheduled { cron } => ("scheduled", Some(cron.clone())),
    }
}

fn sync_policy_from_columns(
    policy: &str,
    cron: Option<String>,
) -> Result<SourceSyncPolicy, StorageError> {
    match policy {
        "manual" => Ok(SourceSyncPolicy::Manual),
        "on_demand" => Ok(SourceSyncPolicy::OnDemand),
        "scheduled" => Ok(SourceSyncPolicy::Scheduled {
            cron: cron.unwrap_or_default(),
        }),
        _ => Err(StorageError::InvalidData(format!(
            "unknown sync policy: {policy}"
        ))),
    }
}

fn privacy_mode_to_str(mode: PrivacyMode) -> &'static str {
    match mode {
        PrivacyMode::LocalOnly => "local_only",
        PrivacyMode::RedactedCloudSync => "redacted_cloud_sync",
        PrivacyMode::ExplicitSnippetSync => "explicit_snippet_sync",
    }
}

fn privacy_mode_from_str(mode: &str) -> Result<PrivacyMode, StorageError> {
    match mode {
        "local_only" => Ok(PrivacyMode::LocalOnly),
        "redacted_cloud_sync" => Ok(PrivacyMode::RedactedCloudSync),
        "explicit_snippet_sync" => Ok(PrivacyMode::ExplicitSnippetSync),
        _ => Err(StorageError::InvalidData(format!(
            "unknown privacy mode: {mode}"
        ))),
    }
}

fn ingestion_status_to_str(status: IngestionRunStatus) -> &'static str {
    match status {
        IngestionRunStatus::Running => "running",
        IngestionRunStatus::Completed => "completed",
        IngestionRunStatus::Partial => "partial",
        IngestionRunStatus::Failed => "failed",
    }
}

fn ingestion_status_from_str(status: &str) -> Result<IngestionRunStatus, StorageError> {
    match status {
        "running" => Ok(IngestionRunStatus::Running),
        "completed" => Ok(IngestionRunStatus::Completed),
        "partial" => Ok(IngestionRunStatus::Partial),
        "failed" => Ok(IngestionRunStatus::Failed),
        _ => Err(StorageError::InvalidData(format!(
            "unknown ingestion status: {status}"
        ))),
    }
}

fn chunking_strategy_to_str(strategy: ChunkingStrategy) -> &'static str {
    match strategy.normalized() {
        ChunkingStrategy::Structured | ChunkingStrategy::SmartSections => "structured",
        ChunkingStrategy::Whitespace => "whitespace",
    }
}

fn chunking_strategy_from_str(strategy: &str) -> Result<ChunkingStrategy, StorageError> {
    match strategy {
        "structured" | "smart_sections" => Ok(ChunkingStrategy::Structured),
        "whitespace" => Ok(ChunkingStrategy::Whitespace),
        _ => Err(StorageError::InvalidData(format!(
            "unknown chunking strategy: {strategy}"
        ))),
    }
}

fn document_profile_to_str(profile: DocumentProfile) -> &'static str {
    match profile {
        DocumentProfile::General => "general",
        DocumentProfile::TechnicalDocs => "technical_docs",
        DocumentProfile::PolicyOrLegal => "policy_or_legal",
        DocumentProfile::SupportKb => "support_kb",
        DocumentProfile::ResearchPaper => "research_paper",
        DocumentProfile::CodeDocs => "code_docs",
        DocumentProfile::Resume => "resume",
    }
}

fn document_profile_from_str(profile: &str) -> Result<DocumentProfile, StorageError> {
    match profile {
        "general" => Ok(DocumentProfile::General),
        "technical_docs" => Ok(DocumentProfile::TechnicalDocs),
        "policy_or_legal" => Ok(DocumentProfile::PolicyOrLegal),
        "support_kb" => Ok(DocumentProfile::SupportKb),
        "research_paper" => Ok(DocumentProfile::ResearchPaper),
        "code_docs" => Ok(DocumentProfile::CodeDocs),
        "resume" => Ok(DocumentProfile::Resume),
        _ => Err(StorageError::InvalidData(format!(
            "unknown document profile: {profile}"
        ))),
    }
}

fn extraction_quality_to_str(quality: ExtractionQuality) -> &'static str {
    match quality {
        ExtractionQuality::High => "high",
        ExtractionQuality::Medium => "medium",
        ExtractionQuality::Low => "low",
        ExtractionQuality::Unknown => "unknown",
    }
}

fn extraction_quality_from_str(quality: &str) -> Result<ExtractionQuality, StorageError> {
    match quality {
        "high" => Ok(ExtractionQuality::High),
        "medium" => Ok(ExtractionQuality::Medium),
        "low" => Ok(ExtractionQuality::Low),
        "unknown" => Ok(ExtractionQuality::Unknown),
        _ => Err(StorageError::InvalidData(format!(
            "unknown extraction quality: {quality}"
        ))),
    }
}

fn chunk_quality_flag_to_str(flag: ChunkQualityFlag) -> &'static str {
    match flag {
        ChunkQualityFlag::HeadingOnly => "heading_only",
        ChunkQualityFlag::TooShort => "too_short",
        ChunkQualityFlag::TooLong => "too_long",
        ChunkQualityFlag::Duplicate => "duplicate",
        ChunkQualityFlag::LowTextDensity => "low_text_density",
        ChunkQualityFlag::ExtractionWarning => "extraction_warning",
        ChunkQualityFlag::GoodEvidenceCandidate => "good_evidence_candidate",
    }
}

fn chunk_quality_flag_from_str(flag: &str) -> Result<ChunkQualityFlag, StorageError> {
    match flag {
        "heading_only" => Ok(ChunkQualityFlag::HeadingOnly),
        "too_short" => Ok(ChunkQualityFlag::TooShort),
        "too_long" => Ok(ChunkQualityFlag::TooLong),
        "duplicate" => Ok(ChunkQualityFlag::Duplicate),
        "low_text_density" => Ok(ChunkQualityFlag::LowTextDensity),
        "extraction_warning" => Ok(ChunkQualityFlag::ExtractionWarning),
        "good_evidence_candidate" => Ok(ChunkQualityFlag::GoodEvidenceCandidate),
        _ => Err(StorageError::InvalidData(format!(
            "unknown chunk quality flag: {flag}"
        ))),
    }
}

fn chunk_quality_flags_to_text(flags: &[ChunkQualityFlag]) -> Vec<String> {
    flags
        .iter()
        .map(|flag| chunk_quality_flag_to_str(*flag).to_owned())
        .collect()
}

fn chunk_quality_flags_from_text(
    flags: Vec<String>,
) -> Result<Vec<ChunkQualityFlag>, StorageError> {
    flags
        .iter()
        .map(|flag| chunk_quality_flag_from_str(flag))
        .collect()
}

fn document_warnings_to_text(warnings: &[DocumentWarning]) -> Vec<String> {
    warnings
        .iter()
        .map(|warning| format!("{}:{}", warning.code, warning.message))
        .collect()
}

fn document_warnings_from_text(warnings: Vec<String>) -> Vec<DocumentWarning> {
    warnings
        .into_iter()
        .map(|warning| {
            let (code, message) = warning
                .split_once(':')
                .map(|(code, message)| (code.to_owned(), message.to_owned()))
                .unwrap_or_else(|| ("warning".to_owned(), warning));
            DocumentWarning { code, message }
        })
        .collect()
}

fn chunk_split_reason_to_str(reason: ChunkSplitReason) -> &'static str {
    match reason {
        ChunkSplitReason::SectionBoundary => "section_boundary",
        ChunkSplitReason::TokenLimit => "token_limit",
        ChunkSplitReason::DocumentEnd => "document_end",
        ChunkSplitReason::FallbackWhitespace => "fallback_whitespace",
    }
}

fn chunk_split_reason_from_str(reason: &str) -> Result<ChunkSplitReason, StorageError> {
    match reason {
        "section_boundary" => Ok(ChunkSplitReason::SectionBoundary),
        "token_limit" => Ok(ChunkSplitReason::TokenLimit),
        "document_end" => Ok(ChunkSplitReason::DocumentEnd),
        "fallback_whitespace" => Ok(ChunkSplitReason::FallbackWhitespace),
        _ => Err(StorageError::InvalidData(format!(
            "unknown chunk split reason: {reason}"
        ))),
    }
}

fn answer_status_to_str(status: ExtractiveAnswerStatus) -> &'static str {
    match status {
        ExtractiveAnswerStatus::Answered => "answered",
        ExtractiveAnswerStatus::InsufficientEvidence => "insufficient_evidence",
    }
}

fn retrieval_mode_to_str(mode: RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Lexical => "lexical",
        RetrievalMode::Vector => "vector",
        RetrievalMode::Hybrid => "hybrid",
    }
}

fn retrieval_mode_from_str(mode: &str) -> Result<RetrievalMode, StorageError> {
    match mode {
        "lexical" => Ok(RetrievalMode::Lexical),
        "vector" => Ok(RetrievalMode::Vector),
        "hybrid" => Ok(RetrievalMode::Hybrid),
        _ => Err(StorageError::InvalidData(format!(
            "unknown retrieval mode: {mode}"
        ))),
    }
}

fn eval_gate_status_to_str(status: RetrievalEvalGateStatus) -> &'static str {
    match status {
        RetrievalEvalGateStatus::Passed => "passed",
        RetrievalEvalGateStatus::Failed => "failed",
    }
}

fn trace_status_to_str(status: TraceStatus) -> &'static str {
    match status {
        TraceStatus::Completed => "completed",
        TraceStatus::Warning => "warning",
        TraceStatus::Failed => "failed",
    }
}

fn evidence_strength_to_str(strength: EvidenceStrength) -> &'static str {
    match strength {
        EvidenceStrength::Strong => "strong",
        EvidenceStrength::Medium => "medium",
        EvidenceStrength::Weak => "weak",
    }
}

fn evidence_strength_from_str(strength: &str) -> Result<EvidenceStrength, StorageError> {
    match strength {
        "strong" => Ok(EvidenceStrength::Strong),
        "medium" => Ok(EvidenceStrength::Medium),
        "weak" => Ok(EvidenceStrength::Weak),
        _ => Err(StorageError::InvalidData(format!(
            "unknown evidence strength: {strength}"
        ))),
    }
}

fn failure_label_to_str(label: &FailureLabel) -> &'static str {
    match label {
        FailureLabel::MissingDocument => "missing_document",
        FailureLabel::BadChunking => "bad_chunking",
        FailureLabel::BadEmbedding => "bad_embedding",
        FailureLabel::BadRanking => "bad_ranking",
        FailureLabel::BadPrompt => "bad_prompt",
        FailureLabel::UnsupportedQuestion => "unsupported_question",
        FailureLabel::HallucinatedAnswer => "hallucinated_answer",
        FailureLabel::WeakEvidence => "weak_evidence",
        FailureLabel::MissingEmbeddingIndex => "missing_embedding_index",
        FailureLabel::DuplicateEvidence => "duplicate_evidence",
        FailureLabel::HeadingOnlyEvidence => "heading_only_evidence",
    }
}

fn failure_label_from_str(label: &str) -> Result<FailureLabel, StorageError> {
    match label {
        "missing_document" => Ok(FailureLabel::MissingDocument),
        "bad_chunking" => Ok(FailureLabel::BadChunking),
        "bad_embedding" => Ok(FailureLabel::BadEmbedding),
        "bad_ranking" => Ok(FailureLabel::BadRanking),
        "bad_prompt" => Ok(FailureLabel::BadPrompt),
        "unsupported_question" => Ok(FailureLabel::UnsupportedQuestion),
        "hallucinated_answer" => Ok(FailureLabel::HallucinatedAnswer),
        "weak_evidence" => Ok(FailureLabel::WeakEvidence),
        "missing_embedding_index" => Ok(FailureLabel::MissingEmbeddingIndex),
        "duplicate_evidence" => Ok(FailureLabel::DuplicateEvidence),
        "heading_only_evidence" => Ok(FailureLabel::HeadingOnlyEvidence),
        _ => Err(StorageError::InvalidData(format!(
            "unknown failure label: {label}"
        ))),
    }
}

fn failure_labels_to_text(labels: &[FailureLabel]) -> Vec<String> {
    labels
        .iter()
        .map(|label| failure_label_to_str(label).to_owned())
        .collect()
}

fn failure_labels_from_text(labels: Vec<String>) -> Result<Vec<FailureLabel>, StorageError> {
    labels
        .iter()
        .map(|label| failure_label_from_str(label))
        .collect()
}

fn source_filter_ids(source_ids: &[SourceId]) -> Vec<Uuid> {
    source_ids.iter().map(|source_id| source_id.0).collect()
}

fn document_filter_ids(document_ids: &[DocumentId]) -> Vec<Uuid> {
    document_ids
        .iter()
        .map(|document_id| document_id.0)
        .collect()
}

fn matched_terms_to_text(terms: &[RetrievalMatchedTerm]) -> String {
    terms
        .iter()
        .map(|term| format!("{}:{}", term.term, term.count))
        .collect::<Vec<_>>()
        .join(",")
}

fn as_u32(value: i32, field: &str) -> Result<u32, StorageError> {
    value
        .try_into()
        .map_err(|_| StorageError::InvalidData(format!("{field} cannot be negative")))
}

fn as_u64(value: i64, field: &str) -> Result<u64, StorageError> {
    value
        .try_into()
        .map_err(|_| StorageError::InvalidData(format!("{field} cannot be negative")))
}
