use rag_debugger_core::{
    DebugReport, DebugReportId, DebugReportPrivacyMode, DebugReportSource, WorkspaceId,
};
use sqlx::Row;

use super::PostgresStore;
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn save_debug_report(
        &self,
        report: DebugReport,
    ) -> Result<DebugReport, StorageError> {
        let (source_type, source_id) = report_source_columns(&report.source);
        let report_json = serde_json::to_value(&report)
            .map_err(|error| StorageError::InvalidData(error.to_string()))?;
        let result = sqlx::query(
            "INSERT INTO debug_reports (
                id, workspace_id, project_id, source_type, source_id, privacy_mode,
                title, subject, report_json, created_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(report.id.0)
        .bind(report.workspace_id.0)
        .bind(report.project_id.0)
        .bind(source_type)
        .bind(source_id)
        .bind(privacy_mode_label(report.privacy_mode))
        .bind(&report.title)
        .bind(&report.subject)
        .bind(report_json)
        .bind(report.created_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(report),
            Err(error) if is_unique_violation(&error) => Err(StorageError::Conflict(format!(
                "debug report {}",
                report.id.0
            ))),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) async fn list_debug_reports(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<DebugReport>, StorageError> {
        let rows = sqlx::query(
            "SELECT report_json
             FROM debug_reports
             WHERE workspace_id = $1
             ORDER BY created_at DESC, id DESC",
        )
        .bind(workspace_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(report_from_row).collect()
    }

    pub(super) async fn get_debug_report(
        &self,
        workspace_id: WorkspaceId,
        report_id: DebugReportId,
    ) -> Result<DebugReport, StorageError> {
        let row = sqlx::query(
            "SELECT report_json
             FROM debug_reports
             WHERE workspace_id = $1 AND id = $2",
        )
        .bind(workspace_id.0)
        .bind(report_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        report_from_row(&row)
    }
}

fn report_from_row(row: &sqlx::postgres::PgRow) -> Result<DebugReport, StorageError> {
    serde_json::from_value(row.try_get("report_json")?)
        .map_err(|error| StorageError::InvalidData(error.to_string()))
}

fn report_source_columns(source: &DebugReportSource) -> (&'static str, Option<uuid::Uuid>) {
    match source {
        DebugReportSource::Trace { trace_id } => ("trace", Some(trace_id.0)),
        DebugReportSource::EvalExperiment { experiment_id } => {
            ("eval_experiment", Some(experiment_id.0))
        }
        DebugReportSource::CiEvalRun { run_id } => ("ci_eval_run", Some(run_id.0)),
        DebugReportSource::Manual { .. } => ("manual", None),
    }
}

fn privacy_mode_label(mode: DebugReportPrivacyMode) -> &'static str {
    match mode {
        DebugReportPrivacyMode::MetadataOnly => "metadata_only",
        DebugReportPrivacyMode::SnippetsAllowed => "snippets_allowed",
        DebugReportPrivacyMode::FullLocalOnly => "full_local_only",
    }
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .is_some_and(|code| code == "23505")
}
