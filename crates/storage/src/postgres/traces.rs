use rag_debugger_core::*;
use sqlx::types::Json;
use time::OffsetDateTime;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn save_trace(&self, trace: Trace) -> Result<Trace, StorageError> {
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

    pub(super) async fn list_traces(&self) -> Result<Vec<TraceSummary>, StorageError> {
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

    pub(super) async fn get_trace_detail(&self, id: TraceId) -> Result<Trace, StorageError> {
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
}
