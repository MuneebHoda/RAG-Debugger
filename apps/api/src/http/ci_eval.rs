use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use rag_debugger_core::{
    ApiKeyScope, CiEvalPrincipal, CiEvalRegressionSummary, CiEvalReport, CiEvalRun, CiEvalRunId,
    CiEvalRunReportResponse, CiEvalRunStatus, RetrievalEvalConfigSnapshot, RetrievalEvalExperiment,
    RetrievalEvalExperimentId, RetrievalEvalGateStatus, RetrievalEvalModeResult, RetrievalEvalRun,
    RetrievalEvalRunId, RetrievalMode, RetrievalQueryRequest, RunCiEvalRequest,
};
use rag_debugger_rag::{
    embedding::LocalHashEmbeddingProvider,
    evals::{
        compare_mode_results, evaluate_gate, evaluate_retrieval_eval_case, summarize_mode_result,
    },
    retrieval::LocalHybridRetriever,
    RagError,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{auth, error::ApiError, state::AppState};

pub async fn run_ci_eval(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RunCiEvalRequest>,
) -> Result<(StatusCode, Json<CiEvalRun>), ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let api_key =
        auth::authenticate_api_key(repository.as_ref(), &headers, ApiKeyScope::CiEvalRuns).await?;
    let dataset = repository
        .get_retrieval_eval_dataset(request.dataset_id)
        .await
        .map_err(not_found_to_api("eval dataset"))?;
    if dataset.cases.is_empty() {
        return Err(ApiError::BadRequest(
            "eval dataset needs at least one case".to_owned(),
        ));
    }

    let modes = normalized_modes(request.modes);
    let top_k = normalized_top_k(
        request
            .top_k
            .unwrap_or(state.config().product.retrieval.default_top_k),
        state.config().product.retrieval.default_top_k,
        state.config().product.retrieval.max_top_k,
    );
    let config_label = request
        .config_label
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| "default".to_owned());
    let baseline = repository
        .latest_ci_eval_run_for_dataset(dataset.id, &config_label)
        .await?;
    let experiment =
        run_experiment_for_dataset(&state, dataset.clone(), modes, top_k, request.name).await?;
    let saved_experiment = repository
        .save_retrieval_eval_experiment(experiment)
        .await?;
    save_legacy_best_run(repository.as_ref(), &saved_experiment).await?;

    let report = build_report(&saved_experiment);
    let regression = baseline
        .as_ref()
        .and_then(|baseline| regression_summary(baseline, &saved_experiment));
    let status = if saved_experiment.gate.status == RetrievalEvalGateStatus::Passed {
        CiEvalRunStatus::Passed
    } else {
        CiEvalRunStatus::Failed
    };
    let run = CiEvalRun {
        id: CiEvalRunId(Uuid::now_v7()),
        workspace_id: api_key.workspace_id,
        dataset_id: dataset.id,
        dataset_name: dataset.name,
        experiment_id: saved_experiment.id,
        status,
        gate_status: saved_experiment.gate.status,
        branch: request.branch,
        commit_sha: request.commit_sha,
        base_ref: request.base_ref,
        head_ref: request.head_ref,
        config_label,
        regression,
        report,
        created_at: OffsetDateTime::now_utc(),
    };
    let saved = repository.save_ci_eval_run(run).await?;
    let response_status = if request.fail_on_gate && saved.status == CiEvalRunStatus::Failed {
        StatusCode::UNPROCESSABLE_ENTITY
    } else {
        StatusCode::CREATED
    };
    Ok((response_status, Json(saved)))
}

pub async fn list_ci_eval_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CiEvalRun>>, ApiError> {
    authorize_ci_read(&state, &headers).await?;
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(repository.list_ci_eval_runs().await?))
}

pub async fn get_ci_eval_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(run_id): Path<Uuid>,
) -> Result<Json<CiEvalRun>, ApiError> {
    authorize_ci_read(&state, &headers).await?;
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository
            .get_ci_eval_run(CiEvalRunId(run_id))
            .await
            .map_err(not_found_to_api("CI eval run"))?,
    ))
}

pub async fn get_ci_eval_report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(run_id): Path<Uuid>,
) -> Result<Json<CiEvalRunReportResponse>, ApiError> {
    authorize_ci_read(&state, &headers).await?;
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let run = repository
        .get_ci_eval_run(CiEvalRunId(run_id))
        .await
        .map_err(not_found_to_api("CI eval run"))?;
    Ok(Json(CiEvalRunReportResponse {
        report: run.report.clone(),
        run,
    }))
}

async fn authorize_ci_read(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<CiEvalPrincipal, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    if let Ok(user) =
        auth::authenticate_session(repository.as_ref(), headers, &state.config().auth).await
    {
        return Ok(CiEvalPrincipal {
            workspace_id: user.workspace.id,
            role: Some(user.role),
        });
    }
    let api_key =
        auth::authenticate_api_key(repository.as_ref(), headers, ApiKeyScope::CiEvalRuns).await?;
    Ok(CiEvalPrincipal {
        workspace_id: api_key.workspace_id,
        role: None,
    })
}

async fn run_experiment_for_dataset(
    state: &AppState,
    dataset: rag_debugger_core::RetrievalEvalDataset,
    modes: Vec<RetrievalMode>,
    top_k: u32,
    name: Option<String>,
) -> Result<RetrievalEvalExperiment, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let provider = LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone());
    let retriever = LocalHybridRetriever::new(provider, state.config().product.retrieval.clone());
    let mut mode_results = Vec::with_capacity(modes.len());

    for mode in &modes {
        let mut case_results = Vec::with_capacity(dataset.cases.len());
        for eval_case in &dataset.cases {
            let case = rag_debugger_core::RetrievalEvalCase {
                top_k,
                ..eval_case.clone()
            };
            let query_request = RetrievalQueryRequest {
                query: case.query.clone(),
                top_k,
                retrieval_mode: *mode,
                source_ids: Vec::new(),
                document_ids: Vec::new(),
            };
            let candidates = repository.list_searchable_chunks(&query_request).await?;
            let response = retriever
                .retrieve(query_request, candidates)
                .map_err(rag_error_to_api_error)?;
            case_results.push(evaluate_retrieval_eval_case(&case, &response));
        }
        mode_results.push(summarize_mode_result(*mode, case_results));
    }

    let comparison = compare_mode_results(&mode_results);
    let gate = evaluate_gate(&mode_results);
    let failures = mode_results
        .iter()
        .flat_map(|result| result.case_results.iter())
        .flat_map(|result| result.failures.iter().cloned())
        .collect::<Vec<_>>();
    Ok(RetrievalEvalExperiment {
        id: RetrievalEvalExperimentId(Uuid::now_v7()),
        dataset_id: dataset.id,
        dataset_name: dataset.name.clone(),
        name: name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("{} CI gate", dataset.name)),
        modes,
        top_k,
        config_snapshot: RetrievalEvalConfigSnapshot {
            top_k,
            scoring_weights: state.config().product.retrieval.weights.clone(),
            embedding_model: state.config().product.embedding.model.clone(),
            dataset_case_count: dataset.cases.len() as u32,
        },
        mode_results,
        comparison,
        gate,
        failures,
        created_at: OffsetDateTime::now_utc(),
    })
}

async fn save_legacy_best_run(
    repository: &dyn rag_debugger_storage::repository::AppRepository,
    experiment: &RetrievalEvalExperiment,
) -> Result<(), ApiError> {
    if let Some(best_result) = best_mode_result(experiment) {
        repository
            .save_retrieval_eval_run(&RetrievalEvalRun {
                id: RetrievalEvalRunId(Uuid::now_v7()),
                retrieval_mode: best_result.retrieval_mode,
                case_count: best_result.case_count,
                passed_count: best_result.passed_count,
                average_recall_at_k: best_result.average_recall_at_k,
                average_precision_at_k: best_result.average_precision_at_k,
                created_at: experiment.created_at,
                results: best_result
                    .case_results
                    .iter()
                    .map(|result| rag_debugger_core::RetrievalEvalResult {
                        case_id: result.case_id,
                        query: result.query.clone(),
                        top_k: result.top_k,
                        recall_at_k: result.recall_at_k,
                        precision_at_k: result.precision_at_k,
                        top_hit_rank: result.top_hit_rank,
                        passed: result.passed,
                        expected_chunk_ids: result.expected_chunk_ids.clone(),
                        expected_document_ids: result.expected_document_ids.clone(),
                        retrieved_chunk_ids: result.retrieved_chunk_ids.clone(),
                        latency_ms: result.latency_ms,
                    })
                    .collect(),
            })
            .await?;
    }
    Ok(())
}

fn build_report(experiment: &RetrievalEvalExperiment) -> CiEvalReport {
    let failed_cases = experiment.failures.clone();
    let summary = if experiment.gate.status == RetrievalEvalGateStatus::Passed {
        "CI retrieval gate passed.".to_owned()
    } else {
        format!(
            "CI retrieval gate failed with {} failure signals.",
            failed_cases.len()
        )
    };
    CiEvalReport {
        title: format!("{} CI eval report", experiment.dataset_name),
        summary,
        gate: experiment.gate.clone(),
        experiment: experiment.clone(),
        failed_cases,
    }
}

fn regression_summary(
    baseline: &CiEvalRun,
    current: &RetrievalEvalExperiment,
) -> Option<CiEvalRegressionSummary> {
    let baseline_best = best_mode_result(&baseline.report.experiment)?;
    let current_best = best_mode_result(current)?;
    let baseline_failed = baseline
        .report
        .failed_cases
        .iter()
        .map(|failure| failure.case_id)
        .collect::<HashSet<_>>();
    let newly_failed_case_count = current
        .failures
        .iter()
        .filter(|failure| !baseline_failed.contains(&failure.case_id))
        .map(|failure| failure.case_id)
        .collect::<HashSet<_>>()
        .len() as u32;
    let recall_delta = current_best.average_recall_at_k - baseline_best.average_recall_at_k;
    let precision_delta =
        current_best.average_precision_at_k - baseline_best.average_precision_at_k;
    let mrr_delta = current_best.mean_reciprocal_rank - baseline_best.mean_reciprocal_rank;
    let latency_delta_ms = current_best.latency_p95_ms as i64 - baseline_best.latency_p95_ms as i64;
    Some(CiEvalRegressionSummary {
        baseline_run_id: baseline.id,
        recall_delta,
        precision_delta,
        mrr_delta,
        latency_delta_ms,
        newly_failed_case_count,
        summary: format!(
            "Recall {recall_delta:+.2}, precision {precision_delta:+.2}, MRR {mrr_delta:+.2}, p95 latency {latency_delta_ms:+} ms."
        ),
    })
}

fn best_mode_result(experiment: &RetrievalEvalExperiment) -> Option<&RetrievalEvalModeResult> {
    experiment.mode_results.iter().max_by(|left, right| {
        left.average_recall_at_k
            .partial_cmp(&right.average_recall_at_k)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn normalized_top_k(top_k: u32, default_top_k: u32, max_top_k: u32) -> u32 {
    if top_k == 0 {
        default_top_k
    } else {
        top_k.min(max_top_k)
    }
}

fn normalized_modes(modes: Vec<RetrievalMode>) -> Vec<RetrievalMode> {
    let mut normalized = if modes.is_empty() {
        vec![
            RetrievalMode::Hybrid,
            RetrievalMode::Vector,
            RetrievalMode::Lexical,
        ]
    } else {
        modes
    };
    normalized.sort_by_key(|mode| match mode {
        RetrievalMode::Hybrid => 0,
        RetrievalMode::Vector => 1,
        RetrievalMode::Lexical => 2,
    });
    normalized.dedup();
    normalized
}

fn rag_error_to_api_error(error: RagError) -> ApiError {
    match error {
        RagError::InvalidConfig(message) => ApiError::BadRequest(message.to_owned()),
        RagError::NotImplemented(_) => ApiError::Internal,
    }
}

fn not_found_to_api(
    label: &'static str,
) -> impl FnOnce(rag_debugger_storage::StorageError) -> ApiError {
    move |error| match error {
        rag_debugger_storage::StorageError::NotFound => {
            ApiError::NotFound(format!("{label} not found"))
        }
        other => ApiError::Storage(other),
    }
}
