use rag_debugger_core::*;
use sqlx::{types::Json, Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{codec::*, PostgresStore};
use crate::{repository::SubmittedExpectedEvidence, StorageError};

impl PostgresStore {
    pub(super) async fn create_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let dataset_id = ensure_default_eval_dataset(&self.pool, workspace_id).await?;
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
        validate_expected_evidence(
            &mut transaction,
            workspace_id,
            &expected_document_ids,
            &expected_chunk_ids,
        )
        .await?;
        validate_eval_case_provenance(
            &mut transaction,
            workspace_id,
            &eval_case.provenance,
            &eval_case.query,
        )
        .await?;
        sqlx::query(
            "INSERT INTO retrieval_eval_cases (
                id, dataset_id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes,
                source_trace_id, source_ingestion_source, source_privacy_mode, created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(eval_case.id.0)
        .bind(dataset_id.0)
        .bind(&eval_case.name)
        .bind(&eval_case.query)
        .bind(eval_case.top_k as i32)
        .bind(expected_chunk_ids)
        .bind(expected_document_ids)
        .bind(&eval_case.notes)
        .bind(eval_case.provenance.as_ref().map(|value| value.source_trace_id.0))
        .bind(eval_case.provenance.as_ref().map(|value| value.source.as_str()))
        .bind(eval_case.provenance.as_ref().map(|value| value.privacy_mode.as_str()))
        .bind(eval_case.created_at)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;

        Ok(eval_case)
    }

    pub(super) async fn list_retrieval_eval_cases(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        let rows = sqlx::query(
            "SELECT c.id, c.name, c.query, c.top_k, c.expected_chunk_ids,
                    c.expected_document_ids, c.notes, c.source_trace_id,
                    c.source_ingestion_source, c.source_privacy_mode, c.created_at
             FROM retrieval_eval_cases c
             INNER JOIN retrieval_eval_datasets d ON d.id = c.dataset_id
             WHERE d.workspace_id = $1
             ORDER BY c.created_at DESC",
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(retrieval_eval_case_from_row).collect()
    }

    pub(super) async fn list_retrieval_eval_cases_by_id(
        &self,
        workspace_id: WorkspaceId,
        case_ids: &[RetrievalEvalCaseId],
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        let ids = case_ids.iter().map(|case_id| case_id.0).collect::<Vec<_>>();
        let rows = sqlx::query(
            "SELECT c.id, c.name, c.query, c.top_k, c.expected_chunk_ids,
                    c.expected_document_ids, c.notes, c.source_trace_id,
                    c.source_ingestion_source, c.source_privacy_mode, c.created_at
             FROM retrieval_eval_cases c
             INNER JOIN retrieval_eval_datasets d ON d.id = c.dataset_id
             WHERE d.workspace_id = $1 AND c.id = ANY($2)
             ORDER BY c.created_at DESC",
        )
        .bind(workspace_id.0)
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(retrieval_eval_case_from_row).collect()
    }

    pub(super) async fn get_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let row = sqlx::query(
            "SELECT c.id, c.name, c.query, c.top_k, c.expected_chunk_ids,
                    c.expected_document_ids, c.notes, c.source_trace_id,
                    c.source_ingestion_source, c.source_privacy_mode, c.created_at
             FROM retrieval_eval_cases c
             INNER JOIN retrieval_eval_datasets d ON d.id = c.dataset_id
             WHERE d.workspace_id = $1 AND c.id = $2",
        )
        .bind(workspace_id.0)
        .bind(case_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        retrieval_eval_case_from_row(&row)
    }

    pub(super) async fn save_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
        eval_run: &RetrievalEvalRun,
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        let case_ids = eval_run
            .results
            .iter()
            .map(|result| result.case_id.0)
            .collect::<Vec<_>>();
        let owned_case_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM retrieval_eval_cases c
             INNER JOIN retrieval_eval_datasets d ON d.id = c.dataset_id
             WHERE d.workspace_id = $1 AND c.id = ANY($2)",
        )
        .bind(workspace_id.0)
        .bind(&case_ids)
        .fetch_one(&mut *transaction)
        .await?;
        if owned_case_count != case_ids.len() as i64 {
            return Err(StorageError::NotFound);
        }

        sqlx::query(
            "INSERT INTO retrieval_eval_runs (
                id, retrieval_mode, case_count, passed_count,
                average_recall_at_k, average_precision_at_k, created_at, workspace_id
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(eval_run.id.0)
        .bind(retrieval_mode_to_str(eval_run.retrieval_mode))
        .bind(eval_run.case_count as i32)
        .bind(eval_run.passed_count as i32)
        .bind(eval_run.average_recall_at_k)
        .bind(eval_run.average_precision_at_k)
        .bind(eval_run.created_at)
        .bind(workspace_id.0)
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

    pub(super) async fn latest_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalRun>, StorageError> {
        let Some(row) = sqlx::query(
            "SELECT id, retrieval_mode, case_count, passed_count,
                    average_recall_at_k, average_precision_at_k, created_at
             FROM retrieval_eval_runs
             WHERE workspace_id = $1
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(workspace_id.0)
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

    pub(super) async fn create_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset: RetrievalEvalDataset,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        sqlx::query(
            "INSERT INTO retrieval_eval_datasets (
                 id, workspace_id, is_default, name, description, created_at, updated_at
             )
             SELECT $1, $2, FALSE, $3, $4, $5, $6
             WHERE EXISTS (SELECT 1 FROM workspaces WHERE id = $2)",
        )
        .bind(dataset.id.0)
        .bind(workspace_id.0)
        .bind(&dataset.name)
        .bind(&dataset.description)
        .bind(dataset.created_at)
        .bind(dataset.updated_at)
        .execute(&self.pool)
        .await?
        .rows_affected()
        .eq(&1)
        .then_some(())
        .ok_or(StorageError::NotFound)?;

        Ok(dataset)
    }

    pub(super) async fn list_retrieval_eval_datasets(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalDatasetSummary>, StorageError> {
        ensure_default_eval_dataset(&self.pool, workspace_id).await?;
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
             WHERE d.workspace_id = $1
             GROUP BY d.id, e.experiment_json
             ORDER BY d.updated_at DESC",
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(eval_dataset_summary_from_row).collect()
    }

    pub(super) async fn get_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        ensure_default_eval_dataset(&self.pool, workspace_id).await?;
        let row = sqlx::query(
            "SELECT id, name, description, created_at, updated_at
             FROM retrieval_eval_datasets
             WHERE workspace_id = $1 AND id = $2",
        )
        .bind(workspace_id.0)
        .bind(dataset_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        let case_rows = sqlx::query(
            "SELECT id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes,
                    source_trace_id, source_ingestion_source, source_privacy_mode, created_at
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

    pub(super) async fn create_retrieval_eval_case_in_dataset(
        &self,
        workspace_id: WorkspaceId,
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
        let owns_dataset = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1 FROM retrieval_eval_datasets
                 WHERE id = $1 AND workspace_id = $2
             )",
        )
        .bind(dataset_id.0)
        .bind(workspace_id.0)
        .fetch_one(&mut *transaction)
        .await?;
        if !owns_dataset {
            return Err(StorageError::NotFound);
        }
        validate_expected_evidence(
            &mut transaction,
            workspace_id,
            &expected_document_ids,
            &expected_chunk_ids,
        )
        .await?;
        validate_eval_case_provenance(
            &mut transaction,
            workspace_id,
            &eval_case.provenance,
            &eval_case.query,
        )
        .await?;

        sqlx::query(
            "INSERT INTO retrieval_eval_cases (
                id, dataset_id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes,
                source_trace_id, source_ingestion_source, source_privacy_mode, created_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(eval_case.id.0)
        .bind(dataset_id.0)
        .bind(&eval_case.name)
        .bind(&eval_case.query)
        .bind(eval_case.top_k as i32)
        .bind(expected_chunk_ids)
        .bind(expected_document_ids)
        .bind(&eval_case.notes)
        .bind(eval_case.provenance.as_ref().map(|value| value.source_trace_id.0))
        .bind(eval_case.provenance.as_ref().map(|value| value.source.as_str()))
        .bind(eval_case.provenance.as_ref().map(|value| value.privacy_mode.as_str()))
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

    pub(super) async fn update_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
        submitted_evidence: SubmittedExpectedEvidence,
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
        let submitted_document_ids = submitted_evidence
            .document_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| id.0)
            .collect::<Vec<_>>();
        let submitted_chunk_ids = submitted_evidence
            .chunk_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| id.0)
            .collect::<Vec<_>>();
        let mut transaction = self.pool.begin().await?;
        validate_expected_evidence(
            &mut transaction,
            workspace_id,
            &submitted_document_ids,
            &submitted_chunk_ids,
        )
        .await?;
        validate_eval_case_provenance(
            &mut transaction,
            workspace_id,
            &eval_case.provenance,
            &eval_case.query,
        )
        .await?;
        let row = sqlx::query(
            "UPDATE retrieval_eval_cases
             SET name = $2, query = $3, top_k = $4, expected_chunk_ids = $5,
                 expected_document_ids = $6, notes = $7
             WHERE id = $1
               AND EXISTS (
                   SELECT 1
                   FROM retrieval_eval_datasets d
                   WHERE d.id = retrieval_eval_cases.dataset_id
                     AND d.workspace_id = $8
               )
               AND source_trace_id IS NOT DISTINCT FROM $9
               AND source_ingestion_source IS NOT DISTINCT FROM $10
               AND source_privacy_mode IS NOT DISTINCT FROM $11
             RETURNING dataset_id",
        )
        .bind(eval_case.id.0)
        .bind(&eval_case.name)
        .bind(&eval_case.query)
        .bind(eval_case.top_k as i32)
        .bind(expected_chunk_ids)
        .bind(expected_document_ids)
        .bind(&eval_case.notes)
        .bind(workspace_id.0)
        .bind(
            eval_case
                .provenance
                .as_ref()
                .map(|value| value.source_trace_id.0),
        )
        .bind(
            eval_case
                .provenance
                .as_ref()
                .map(|value| value.source.as_str()),
        )
        .bind(
            eval_case
                .provenance
                .as_ref()
                .map(|value| value.privacy_mode.as_str()),
        )
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(StorageError::NotFound)?;

        if let Some(dataset_id) = row.try_get::<Option<Uuid>, _>("dataset_id")? {
            sqlx::query("UPDATE retrieval_eval_datasets SET updated_at = $1 WHERE id = $2")
                .bind(OffsetDateTime::now_utc())
                .bind(dataset_id)
                .execute(&mut *transaction)
                .await?;
        }

        transaction.commit().await?;
        Ok(eval_case)
    }

    pub(super) async fn delete_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<(), StorageError> {
        let row = sqlx::query(
            "DELETE FROM retrieval_eval_cases
                 WHERE id = $1
                   AND EXISTS (
                       SELECT 1
                       FROM retrieval_eval_datasets d
                       WHERE d.id = retrieval_eval_cases.dataset_id
                         AND d.workspace_id = $2
                   )
                 RETURNING dataset_id",
        )
        .bind(case_id.0)
        .bind(workspace_id.0)
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

    pub(super) async fn save_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
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
             SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
             WHERE EXISTS (
                 SELECT 1
                 FROM retrieval_eval_datasets
                 WHERE id = $2 AND workspace_id = $13
             )",
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
        .bind(workspace_id.0)
        .execute(&self.pool)
        .await?
        .rows_affected()
        .eq(&1)
        .then_some(())
        .ok_or(StorageError::NotFound)?;

        sqlx::query("UPDATE retrieval_eval_datasets SET updated_at = $1 WHERE id = $2")
            .bind(experiment.created_at)
            .bind(experiment.dataset_id.0)
            .execute(&self.pool)
            .await?;

        Ok(experiment)
    }

    pub(super) async fn list_retrieval_eval_experiments(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        let rows = sqlx::query(
            "SELECT experiment_json
             FROM retrieval_eval_experiments e
             INNER JOIN retrieval_eval_datasets d ON d.id = e.dataset_id
             WHERE d.workspace_id = $1
             ORDER BY e.created_at DESC
             LIMIT 100",
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(eval_experiment_from_row).collect()
    }

    pub(super) async fn list_retrieval_eval_experiments_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        let owns_dataset = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1
                 FROM retrieval_eval_datasets
                 WHERE workspace_id = $1 AND id = $2
             )",
        )
        .bind(workspace_id.0)
        .bind(dataset_id.0)
        .fetch_one(&self.pool)
        .await?;
        if !owns_dataset {
            return Err(StorageError::NotFound);
        }

        let rows = sqlx::query(
            "SELECT experiment_json
             FROM retrieval_eval_experiments e
             INNER JOIN retrieval_eval_datasets d ON d.id = e.dataset_id
             WHERE d.workspace_id = $1 AND e.dataset_id = $2
             ORDER BY e.created_at DESC
             LIMIT 100",
        )
        .bind(workspace_id.0)
        .bind(dataset_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(eval_experiment_from_row).collect()
    }

    pub(super) async fn get_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
        experiment_id: RetrievalEvalExperimentId,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        let row = sqlx::query(
            "SELECT experiment_json
             FROM retrieval_eval_experiments e
             INNER JOIN retrieval_eval_datasets d ON d.id = e.dataset_id
             WHERE d.workspace_id = $1 AND e.id = $2",
        )
        .bind(workspace_id.0)
        .bind(experiment_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        eval_experiment_from_row(&row)
    }

    pub(super) async fn latest_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalExperiment>, StorageError> {
        let Some(row) = sqlx::query(
            "SELECT experiment_json
             FROM retrieval_eval_experiments e
             INNER JOIN retrieval_eval_datasets d ON d.id = e.dataset_id
             WHERE d.workspace_id = $1
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(workspace_id.0)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        Ok(Some(eval_experiment_from_row(&row)?))
    }
}

async fn validate_expected_evidence(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: WorkspaceId,
    document_ids: &[Uuid],
    chunk_ids: &[Uuid],
) -> Result<(), StorageError> {
    let expected_document_count = document_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>()
        .len() as i64;
    let expected_chunk_count = chunk_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>()
        .len() as i64;
    let document_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT d.id)
         FROM documents d
         INNER JOIN sources s ON s.id = d.source_id
         INNER JOIN projects p ON p.id = s.project_id
         WHERE p.workspace_id = $1 AND d.id = ANY($2)",
    )
    .bind(workspace_id.0)
    .bind(document_ids)
    .fetch_one(&mut **transaction)
    .await?;
    let chunk_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT c.id)
         FROM chunks c
         INNER JOIN sources s ON s.id = c.source_id
         INNER JOIN projects p ON p.id = s.project_id
         WHERE p.workspace_id = $1 AND c.id = ANY($2)",
    )
    .bind(workspace_id.0)
    .bind(chunk_ids)
    .fetch_one(&mut **transaction)
    .await?;

    if document_count == expected_document_count && chunk_count == expected_chunk_count {
        Ok(())
    } else {
        Err(StorageError::UnavailableEvidence)
    }
}

async fn validate_eval_case_provenance(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: WorkspaceId,
    provenance: &Option<RetrievalEvalCaseProvenance>,
    query: &str,
) -> Result<(), StorageError> {
    let Some(provenance) = provenance else {
        return Ok(());
    };
    if provenance.privacy_mode != TraceIngestionPrivacyMode::FullLocalOnly {
        return Err(StorageError::NotFound);
    }
    let matches = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
             SELECT 1 FROM debug_traces
             WHERE id = $1 AND workspace_id = $2
               AND ingestion_source = $3 AND ingestion_privacy_mode = $4
               AND trace_json ->> 'input' = $5
         )",
    )
    .bind(provenance.source_trace_id.0)
    .bind(workspace_id.0)
    .bind(provenance.source.as_str())
    .bind(provenance.privacy_mode.as_str())
    .bind(query)
    .fetch_one(&mut **transaction)
    .await?;
    matches.then_some(()).ok_or(StorageError::NotFound)
}
