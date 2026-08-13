use rag_debugger_core::*;
use sqlx::types::Json;
use time::OffsetDateTime;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn save_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<Trace, StorageError> {
        let mut transaction = self.pool.begin().await?;
        let owns_trace_inputs: bool = sqlx::query_scalar(
            "SELECT
                EXISTS (
                    SELECT 1 FROM projects
                    WHERE id = $1 AND workspace_id = $2
                )
                AND (
                    $3::uuid IS NULL
                    OR EXISTS (
                        SELECT 1 FROM retrieval_playground_runs
                        WHERE id = $3 AND workspace_id = $2
                    )
                )",
        )
        .bind(trace.project_id.0)
        .bind(workspace_id.0)
        .bind(trace.source_run_id.map(|id| id.0))
        .fetch_one(&mut *transaction)
        .await?;
        if !owns_trace_inputs {
            return Err(StorageError::NotFound);
        }
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

        let trace_write = sqlx::query(
            "INSERT INTO debug_traces (
                id, workspace_id, project_id, source_run_id, query, retrieval_mode, summary, status,
                evidence_strength, failure_labels, span_count, rerun_count, latency_ms,
                trace_json, created_at, updated_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
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
                updated_at = EXCLUDED.updated_at
             WHERE debug_traces.workspace_id = EXCLUDED.workspace_id",
        )
        .bind(trace.id.0)
        .bind(workspace_id.0)
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
        if trace_write.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }

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

    pub(super) async fn list_traces(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<TraceSummary>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, query, retrieval_mode, latency_ms, evidence_strength,
                    failure_labels, span_count, rerun_count, created_at,
                    ingestion_source, external_trace_id, ingestion_mapping_status
             FROM debug_traces
             WHERE workspace_id = $1
             ORDER BY created_at DESC
             LIMIT 100",
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(trace_summary_from_row).collect()
    }

    pub(super) async fn get_trace_detail(
        &self,
        workspace_id: WorkspaceId,
        id: TraceId,
    ) -> Result<Trace, StorageError> {
        let row = sqlx::query(
            "SELECT trace_json
             FROM debug_traces
             WHERE workspace_id = $1 AND id = $2",
        )
        .bind(workspace_id.0)
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        trace_from_row(&row)
    }

    pub(super) async fn upsert_imported_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<ImportedTraceUpsertResult, StorageError> {
        let identity = trace.ingestion.as_ref().ok_or_else(|| {
            StorageError::InvalidData("imported trace metadata is required".to_owned())
        })?;
        let mut transaction = self.pool.begin().await?;
        let owns_project: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM projects WHERE id = $1 AND workspace_id = $2)",
        )
        .bind(trace.project_id.0)
        .bind(workspace_id.0)
        .fetch_one(&mut *transaction)
        .await?;
        if !owns_project {
            return Err(StorageError::NotFound);
        }

        let lock_key = format!(
            "{}:{}:{}:{}",
            workspace_id.0,
            trace.project_id.0,
            identity.source.as_str(),
            identity.external_trace_id
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(lock_key)
            .execute(&mut *transaction)
            .await?;

        let existing = sqlx::query(
            "SELECT trace_json FROM debug_traces
             WHERE workspace_id = $1 AND project_id = $2
               AND ingestion_source = $3 AND external_trace_id = $4
             FOR UPDATE",
        )
        .bind(workspace_id.0)
        .bind(trace.project_id.0)
        .bind(identity.source.as_str())
        .bind(&identity.external_trace_id)
        .fetch_optional(&mut *transaction)
        .await?
        .map(|row| trace_from_row(&row))
        .transpose()?;

        if existing.is_none() {
            if let Some(same_workspace) = sqlx::query_scalar::<_, bool>(
                "SELECT COALESCE(workspace_id = $2, FALSE) FROM debug_traces WHERE id = $1",
            )
            .bind(trace.id.0)
            .bind(workspace_id.0)
            .fetch_optional(&mut *transaction)
            .await?
            {
                if !same_workspace {
                    return Err(StorageError::NotFound);
                }
                return Err(StorageError::Conflict(
                    "trace ID is already assigned to another trace identity".to_owned(),
                ));
            }
        }

        let (saved, disposition) = if let Some(existing) = existing {
            let merged = merge_imported_trace(&existing, trace).map_err(|error| match error {
                rag_debugger_core::TraceMergeError::IdentityImmutable => {
                    StorageError::Conflict(error.to_string())
                }
                _ => StorageError::TraceMerge(error),
            })?;
            let disposition = if merged == existing {
                TraceIngestionDisposition::Unchanged
            } else {
                TraceIngestionDisposition::Updated
            };
            (merged, disposition)
        } else {
            (trace, TraceIngestionDisposition::Created)
        };
        let metadata = saved.ingestion.as_ref().ok_or_else(|| {
            StorageError::InvalidData("imported trace metadata is required".to_owned())
        })?;
        let retrieval_mode = metadata.retrieval_mode.unwrap_or_default();
        let latency_ms = saved
            .completed_at
            .and_then(|end| {
                (end - saved.started_at)
                    .whole_milliseconds()
                    .max(0)
                    .try_into()
                    .ok()
            })
            .unwrap_or(0_i64);
        let now = OffsetDateTime::now_utc();

        let result = sqlx::query(
            "INSERT INTO debug_traces (
                id, workspace_id, project_id, source_run_id, query, retrieval_mode, summary, status,
                evidence_strength, failure_labels, span_count, rerun_count, latency_ms,
                trace_json, created_at, updated_at, ingestion_source, external_trace_id,
                ingestion_schema_version, ingestion_mapper_version, ingestion_mapping_status,
                ingestion_privacy_mode
             ) VALUES ($1,$2,$3,NULL,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21)
             ON CONFLICT (id) DO UPDATE SET
                query=EXCLUDED.query, retrieval_mode=EXCLUDED.retrieval_mode,
                summary=EXCLUDED.summary, status=EXCLUDED.status,
                evidence_strength=EXCLUDED.evidence_strength, failure_labels=EXCLUDED.failure_labels,
                span_count=EXCLUDED.span_count, rerun_count=EXCLUDED.rerun_count,
                latency_ms=EXCLUDED.latency_ms, trace_json=EXCLUDED.trace_json,
                created_at=EXCLUDED.created_at, updated_at=EXCLUDED.updated_at,
                ingestion_mapper_version=EXCLUDED.ingestion_mapper_version,
                ingestion_mapping_status=EXCLUDED.ingestion_mapping_status
             WHERE debug_traces.workspace_id=EXCLUDED.workspace_id
               AND debug_traces.project_id=EXCLUDED.project_id
               AND debug_traces.ingestion_source=EXCLUDED.ingestion_source
               AND debug_traces.external_trace_id=EXCLUDED.external_trace_id",
        )
        .bind(saved.id.0)
        .bind(workspace_id.0)
        .bind(saved.project_id.0)
        .bind(&saved.input)
        .bind(retrieval_mode_to_str(retrieval_mode))
        .bind(&saved.summary)
        .bind(trace_status_to_str(saved.status))
        .bind(evidence_strength_to_str(saved.evidence_strength.unwrap_or(EvidenceStrength::Weak)))
        .bind(failure_labels_to_text(&saved.failure_labels))
        .bind(metadata.spans.len() as i32)
        .bind(saved.reruns.len() as i32)
        .bind(latency_ms)
        .bind(Json(&saved))
        .bind(saved.started_at)
        .bind(now)
        .bind(metadata.source.as_str())
        .bind(&metadata.external_trace_id)
        .bind(&metadata.schema_version)
        .bind(&metadata.mapper_version)
        .bind(metadata.mapping_status.as_str())
        .bind(metadata.privacy_mode.as_str())
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        transaction.commit().await?;
        Ok(ImportedTraceUpsertResult {
            trace: saved,
            disposition,
        })
    }
}
