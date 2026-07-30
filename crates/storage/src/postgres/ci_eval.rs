use rag_debugger_core::*;
use sqlx::{types::Json, Row};

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn save_ci_eval_run(&self, run: CiEvalRun) -> Result<CiEvalRun, StorageError> {
        sqlx::query(
            "INSERT INTO ci_eval_runs (
                id, workspace_id, dataset_id, dataset_name, experiment_id, status, gate_status,
                branch, commit_sha, base_ref, head_ref, config_label, run_json, created_at
             )
             SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
             WHERE EXISTS (
                 SELECT 1
                 FROM retrieval_eval_datasets
                 WHERE id = $3 AND workspace_id = $2
             )",
        )
        .bind(run.id.0)
        .bind(run.workspace_id.0)
        .bind(run.dataset_id.0)
        .bind(&run.dataset_name)
        .bind(run.experiment_id.0)
        .bind(ci_eval_run_status_to_str(run.status))
        .bind(eval_gate_status_to_str(run.gate_status))
        .bind(&run.branch)
        .bind(&run.commit_sha)
        .bind(&run.base_ref)
        .bind(&run.head_ref)
        .bind(&run.config_label)
        .bind(Json(&run))
        .bind(run.created_at)
        .execute(&self.pool)
        .await?
        .rows_affected()
        .eq(&1)
        .then_some(())
        .ok_or(StorageError::NotFound)?;
        Ok(run)
    }

    pub(super) async fn list_ci_eval_runs(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<CiEvalRun>, StorageError> {
        let rows = sqlx::query(
            "SELECT run_json
             FROM ci_eval_runs
             WHERE workspace_id = $1
             ORDER BY created_at DESC
             LIMIT 100",
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(ci_eval_run_from_row).collect()
    }

    pub(super) async fn get_ci_eval_run(
        &self,
        workspace_id: WorkspaceId,
        id: CiEvalRunId,
    ) -> Result<CiEvalRun, StorageError> {
        let row = sqlx::query(
            "SELECT run_json
             FROM ci_eval_runs
             WHERE workspace_id = $1 AND id = $2",
        )
        .bind(workspace_id.0)
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;
        ci_eval_run_from_row(&row)
    }

    pub(super) async fn latest_ci_eval_run_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
        config_label: &str,
    ) -> Result<Option<CiEvalRun>, StorageError> {
        let row = sqlx::query(
            "SELECT run_json
             FROM ci_eval_runs
             WHERE workspace_id = $1 AND dataset_id = $2 AND config_label = $3
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(workspace_id.0)
        .bind(dataset_id.0)
        .bind(config_label)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| ci_eval_run_from_row(&row)).transpose()
    }
}

fn ci_eval_run_from_row(row: &sqlx::postgres::PgRow) -> Result<CiEvalRun, StorageError> {
    Ok(row.try_get::<Json<CiEvalRun>, _>("run_json")?.0)
}
