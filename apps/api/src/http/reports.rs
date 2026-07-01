use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use rag_debugger_core::{
    CreateDebugReportFromCiRunRequest, CreateDebugReportFromExperimentRequest,
    CreateDebugReportFromTraceRequest, DebugReport, DebugReportId,
};
use rag_debugger_rag::reports::{
    build_ci_eval_debug_report, build_eval_experiment_debug_report, build_trace_debug_report,
    render_debug_report_markdown, DebugReportBuildContext, ReportBuildError, ReportExportError,
};
use rag_debugger_storage::StorageError;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{auth, error::ApiError, state::AppState};

pub async fn list_reports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DebugReport>>, ApiError> {
    let (repository, user) = authenticated_repository(&state, &headers).await?;
    Ok(Json(
        repository.list_debug_reports(user.workspace.id).await?,
    ))
}

pub async fn get_report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(report_id): Path<Uuid>,
) -> Result<Json<DebugReport>, ApiError> {
    let (repository, user) = authenticated_repository(&state, &headers).await?;
    Ok(Json(
        repository
            .get_debug_report(user.workspace.id, DebugReportId(report_id))
            .await
            .map_err(report_storage_error)?,
    ))
}

pub async fn create_report_from_trace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDebugReportFromTraceRequest>,
) -> Result<(StatusCode, Json<DebugReport>), ApiError> {
    let (repository, user) = authenticated_repository(&state, &headers).await?;
    let trace = repository
        .get_trace_detail(request.trace_id)
        .await
        .map_err(source_storage_error("trace"))?;
    let trace = rag_debugger_rag::tracing::ensure_trace_diagnosis(
        trace,
        &state.config().product.retrieval,
        &state.config().product.debugger,
    );
    let report = build_trace_debug_report(
        build_context(user.workspace.id, trace.project_id, request.privacy_mode),
        &trace,
    )
    .map_err(report_build_error)?;
    Ok((
        StatusCode::CREATED,
        Json(repository.save_debug_report(report).await?),
    ))
}

pub async fn create_report_from_experiment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDebugReportFromExperimentRequest>,
) -> Result<(StatusCode, Json<DebugReport>), ApiError> {
    let (repository, user) = authenticated_repository(&state, &headers).await?;
    let experiment = repository
        .get_retrieval_eval_experiment(request.experiment_id)
        .await
        .map_err(source_storage_error("eval experiment"))?;
    let project = repository.ensure_default_project().await?;
    let report = build_eval_experiment_debug_report(
        build_context(user.workspace.id, project.id, request.privacy_mode),
        &experiment,
    );
    Ok((
        StatusCode::CREATED,
        Json(repository.save_debug_report(report).await?),
    ))
}

pub async fn create_report_from_ci_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDebugReportFromCiRunRequest>,
) -> Result<(StatusCode, Json<DebugReport>), ApiError> {
    let (repository, user) = authenticated_repository(&state, &headers).await?;
    let run = repository
        .get_ci_eval_run(request.run_id)
        .await
        .map_err(source_storage_error("CI eval run"))?;
    if run.workspace_id != user.workspace.id {
        return Err(ApiError::NotFound("CI eval run not found".to_owned()));
    }
    let project = repository.ensure_default_project().await?;
    let report = build_ci_eval_debug_report(
        build_context(user.workspace.id, project.id, request.privacy_mode),
        &run,
    );
    Ok((
        StatusCode::CREATED,
        Json(repository.save_debug_report(report).await?),
    ))
}

pub async fn export_report_markdown(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(report_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let (repository, user) = authenticated_repository(&state, &headers).await?;
    let report = repository
        .get_debug_report(user.workspace.id, DebugReportId(report_id))
        .await
        .map_err(report_storage_error)?;
    let markdown = render_debug_report_markdown(&report).map_err(report_export_error)?;
    let disposition = HeaderValue::from_str(&format!(
        "attachment; filename=\"corpuslab-report-{}.md\"",
        report.id.0
    ))
    .map_err(|_| ApiError::Internal)?;

    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            ),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        markdown,
    ))
}

async fn authenticated_repository(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<
    (
        std::sync::Arc<dyn rag_debugger_storage::repository::AppRepository>,
        rag_debugger_core::AuthenticatedUser,
    ),
    ApiError,
> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let user =
        auth::authenticate_session(repository.as_ref(), headers, &state.config().auth).await?;
    Ok((repository, user))
}

fn build_context(
    workspace_id: rag_debugger_core::WorkspaceId,
    project_id: rag_debugger_core::ProjectId,
    privacy_mode: rag_debugger_core::DebugReportPrivacyMode,
) -> DebugReportBuildContext {
    DebugReportBuildContext {
        report_id: DebugReportId(Uuid::now_v7()),
        workspace_id,
        project_id,
        privacy_mode,
        created_at: OffsetDateTime::now_utc(),
    }
}

fn source_storage_error(label: &'static str) -> impl FnOnce(StorageError) -> ApiError {
    move |error| match error {
        StorageError::NotFound => ApiError::NotFound(format!("{label} not found")),
        other => ApiError::Storage(other),
    }
}

fn report_storage_error(error: StorageError) -> ApiError {
    match error {
        StorageError::NotFound => ApiError::NotFound("audit report not found".to_owned()),
        other => ApiError::Storage(other),
    }
}

fn report_build_error(error: ReportBuildError) -> ApiError {
    match error {
        ReportBuildError::InvalidSource(message) => ApiError::Unprocessable(message.to_owned()),
    }
}

fn report_export_error(error: ReportExportError) -> ApiError {
    match error {
        ReportExportError::FullLocalOnly => ApiError::Unprocessable(error.to_string()),
    }
}
