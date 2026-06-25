use rag_debugger_core::*;
use sqlx::{types::Json, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn create_retrieval_eval_case(
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

    pub(super) async fn list_retrieval_eval_cases(
        &self,
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, name, query, top_k, expected_chunk_ids, expected_document_ids, notes, created_at
             FROM retrieval_eval_cases
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(retrieval_eval_case_from_row).collect()
    }

    pub(super) async fn list_retrieval_eval_cases_by_id(
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

    pub(super) async fn save_retrieval_eval_run(
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

    pub(super) async fn latest_retrieval_eval_run(
        &self,
    ) -> Result<Option<RetrievalEvalRun>, StorageError> {
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

    pub(super) async fn create_retrieval_eval_dataset(
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

    pub(super) async fn list_retrieval_eval_datasets(
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

    pub(super) async fn get_retrieval_eval_dataset(
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

    pub(super) async fn create_retrieval_eval_case_in_dataset(
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

    pub(super) async fn update_retrieval_eval_case(
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

    pub(super) async fn delete_retrieval_eval_case(
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

    pub(super) async fn save_retrieval_eval_experiment(
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

    pub(super) async fn list_retrieval_eval_experiments(
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

    pub(super) async fn get_retrieval_eval_experiment(
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

    pub(super) async fn latest_retrieval_eval_experiment(
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
