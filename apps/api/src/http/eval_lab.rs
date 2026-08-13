use std::{collections::HashSet, hash::Hash};

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use rag_debugger_core::{
    AuthenticatedUser, ChunkId, CompareRetrievalEvalExperimentRequest,
    CreateRetrievalEvalDatasetRequest, CreateRetrievalEvalLabCaseRequest, DocumentId,
    EvalLabEvidenceSearchQuery, EvalLabEvidenceSearchRequest, QueryEvalLabEvidenceRequest,
    QueryEvalLabEvidenceResponse, RetrievalEvalCase, RetrievalEvalCaseId,
    RetrievalEvalConfigSnapshot, RetrievalEvalDataset, RetrievalEvalDatasetId,
    RetrievalEvalDatasetSummary, RetrievalEvalExperiment, RetrievalEvalExperimentId,
    RetrievalEvalExperimentSummary, RetrievalEvalRegressionComparison, RetrievalEvalRun,
    RetrievalEvalRunId, RetrievalEvalTrendSummary, RetrievalMode, RetrievalQueryRequest,
    RunRetrievalEvalExperimentRequest, TraceIngestionPrivacyMode, UpdateRetrievalEvalCaseRequest,
    WorkspaceId, EVAL_LAB_EVIDENCE_DEFAULT_CANDIDATE_LIMIT, EVAL_LAB_EVIDENCE_MAX_CANDIDATE_LIMIT,
    EVAL_LAB_EVIDENCE_MAX_REQUESTED_CHUNKS, EVAL_LAB_EVIDENCE_MAX_REQUESTED_DOCUMENTS,
    EVAL_LAB_EVIDENCE_MAX_REQUESTED_IDS, EVAL_LAB_EVIDENCE_MIN_TEXT_QUERY_CHARS,
};
use rag_debugger_rag::{
    embedding::LocalHashEmbeddingProvider,
    evals::{
        build_trend_summary, compare_experiment_regression, compare_mode_results, evaluate_gate,
        evaluate_retrieval_eval_case_with_context, expected_chunk_parent_document_ids,
        previous_comparable_experiment, summarize_experiment, summarize_mode_result,
    },
    retrieval::LocalHybridRetriever,
    RagError,
};
use rag_debugger_storage::repository::{EvidenceRepository, SubmittedExpectedEvidence};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub async fn list_datasets(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<RetrievalEvalDatasetSummary>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository
            .list_retrieval_eval_datasets(user.workspace.id)
            .await?,
    ))
}

pub async fn create_dataset(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateRetrievalEvalDatasetRequest>,
) -> Result<Json<RetrievalEvalDataset>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest(
            "dataset name must not be empty".to_owned(),
        ));
    }
    let now = OffsetDateTime::now_utc();
    let dataset = RetrievalEvalDataset {
        id: RetrievalEvalDatasetId(Uuid::now_v7()),
        name: name.to_owned(),
        description: request
            .description
            .filter(|description| !description.trim().is_empty()),
        cases: Vec::new(),
        created_at: now,
        updated_at: now,
    };

    Ok(Json(
        repository
            .create_retrieval_eval_dataset(user.workspace.id, dataset)
            .await?,
    ))
}

pub async fn get_dataset(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(dataset_id): Path<Uuid>,
) -> Result<Json<RetrievalEvalDataset>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository
            .get_retrieval_eval_dataset(user.workspace.id, RetrievalEvalDatasetId(dataset_id))
            .await
            .map_err(not_found_to_api("eval dataset"))?,
    ))
}

pub async fn query_evidence(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<QueryEvalLabEvidenceRequest>,
) -> Result<Json<QueryEvalLabEvidenceResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        build_evidence_response(repository.as_ref(), user.workspace.id, &request).await?,
    ))
}

pub async fn create_case(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(dataset_id): Path<Uuid>,
    Json(mut request): Json<CreateRetrievalEvalLabCaseRequest>,
) -> Result<Json<RetrievalEvalCase>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let workspace_id = user.workspace.id;
    repository
        .get_retrieval_eval_dataset(workspace_id, RetrievalEvalDatasetId(dataset_id))
        .await
        .map_err(not_found_to_api("eval dataset"))?;
    if let Some(trace_id) = request.source_trace_id {
        let trace = repository
            .get_trace_detail(workspace_id, trace_id)
            .await
            .map_err(not_found_to_api("trace"))?;
        if trace.ingestion.as_ref().is_some_and(|metadata| {
            metadata.privacy_mode == TraceIngestionPrivacyMode::FullLocalOnly
        }) {
            return Err(ApiError::Coded {
                status: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                code: "full_local_eval_not_permitted",
                message: "full-local imported traces cannot create Eval Lab cases",
            });
        }
        if trace.input.trim() != request.query.trim() {
            return Err(ApiError::Coded {
                status: axum::http::StatusCode::BAD_REQUEST,
                code: "trace_query_mismatch",
                message: "eval query does not match the source trace",
            });
        }
    }
    validate_evidence_request_counts(
        Some(&request.expected_document_ids),
        Some(&request.expected_chunk_ids),
    )?;
    request.expected_chunk_ids = dedupe_chunk_ids(request.expected_chunk_ids);
    request.expected_document_ids = dedupe_document_ids(request.expected_document_ids);
    let eval_case = eval_case_from_request(
        request,
        state.config().product.retrieval.default_top_k,
        state.config().product.retrieval.max_top_k,
    )?;
    Ok(Json(
        repository
            .create_retrieval_eval_case_in_dataset(
                workspace_id,
                RetrievalEvalDatasetId(dataset_id),
                eval_case,
            )
            .await
            .map_err(eval_case_write_error)?,
    ))
}

pub async fn update_case(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(case_id): Path<Uuid>,
    Json(mut request): Json<UpdateRetrievalEvalCaseRequest>,
) -> Result<Json<RetrievalEvalCase>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    validate_evidence_request_counts(
        request.expected_document_ids.as_deref(),
        request.expected_chunk_ids.as_deref(),
    )?;
    if let Some(chunk_ids) = request.expected_chunk_ids.take() {
        request.expected_chunk_ids = Some(dedupe_chunk_ids(chunk_ids));
    }
    if let Some(document_ids) = request.expected_document_ids.take() {
        request.expected_document_ids = Some(dedupe_document_ids(document_ids));
    }
    let workspace_id = user.workspace.id;
    let case_id = RetrievalEvalCaseId(case_id);
    let current = repository
        .get_retrieval_eval_case(workspace_id, case_id)
        .await
        .map_err(not_found_to_api("eval case"))?;
    let submitted_evidence = SubmittedExpectedEvidence {
        document_ids: request.expected_document_ids.clone(),
        chunk_ids: request.expected_chunk_ids.clone(),
    };
    let updated = merge_case_update(
        current,
        request,
        state.config().product.retrieval.default_top_k,
        state.config().product.retrieval.max_top_k,
    )?;

    Ok(Json(
        repository
            .update_retrieval_eval_case(workspace_id, updated, submitted_evidence)
            .await
            .map_err(eval_case_write_error)?,
    ))
}

pub async fn delete_case(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(case_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    repository
        .delete_retrieval_eval_case(user.workspace.id, RetrievalEvalCaseId(case_id))
        .await
        .map_err(not_found_to_api("eval case"))?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn run_experiment(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<RunRetrievalEvalExperimentRequest>,
) -> Result<Json<RetrievalEvalExperiment>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let workspace_id = user.workspace.id;
    let dataset = repository
        .get_retrieval_eval_dataset(workspace_id, request.dataset_id)
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
    let provider = LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone());
    let retriever = LocalHybridRetriever::new(provider, state.config().product.retrieval.clone())
        .with_debugger_config(state.config().product.debugger.clone());
    let mut mode_results = Vec::with_capacity(modes.len());

    for mode in &modes {
        let mut case_results = Vec::with_capacity(dataset.cases.len());
        for eval_case in &dataset.cases {
            let case = RetrievalEvalCase {
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
            let candidates = repository
                .list_searchable_chunks(workspace_id, &query_request)
                .await?;
            let expected_chunk_document_ids =
                expected_chunk_parent_document_ids(&case, &candidates);
            let response = retriever
                .retrieve(query_request, candidates)
                .map_err(rag_error_to_api_error)?;
            case_results.push(evaluate_retrieval_eval_case_with_context(
                &case,
                &response,
                &state.config().product.debugger,
                &expected_chunk_document_ids,
            ));
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
    let experiment = RetrievalEvalExperiment {
        id: RetrievalEvalExperimentId(Uuid::now_v7()),
        dataset_id: dataset.id,
        dataset_name: dataset.name.clone(),
        name: request
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("{} comparison", dataset.name)),
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
    };

    let saved = repository
        .save_retrieval_eval_experiment(workspace_id, experiment)
        .await?;
    if let Some(best_result) = saved.mode_results.iter().max_by(|left, right| {
        left.average_recall_at_k
            .partial_cmp(&right.average_recall_at_k)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        repository
            .save_retrieval_eval_run(
                workspace_id,
                &RetrievalEvalRun {
                    id: RetrievalEvalRunId(Uuid::now_v7()),
                    retrieval_mode: best_result.retrieval_mode,
                    case_count: best_result.case_count,
                    passed_count: best_result.passed_count,
                    average_recall_at_k: best_result.average_recall_at_k,
                    average_precision_at_k: best_result.average_precision_at_k,
                    created_at: saved.created_at,
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
                },
            )
            .await?;
    }

    Ok(Json(saved))
}

pub async fn list_experiments(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<RetrievalEvalExperiment>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository
            .list_retrieval_eval_experiments(user.workspace.id)
            .await?,
    ))
}

pub async fn list_dataset_experiments(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(dataset_id): Path<Uuid>,
) -> Result<Json<Vec<RetrievalEvalExperimentSummary>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let dataset_id = RetrievalEvalDatasetId(dataset_id);
    let experiments = repository
        .list_retrieval_eval_experiments_for_dataset(user.workspace.id, dataset_id)
        .await?;
    Ok(Json(
        experiments
            .iter()
            .map(summarize_experiment)
            .collect::<Vec<_>>(),
    ))
}

pub async fn dataset_trends(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(dataset_id): Path<Uuid>,
    Query(query): Query<TrendQuery>,
) -> Result<Json<RetrievalEvalTrendSummary>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let dataset_id = RetrievalEvalDatasetId(dataset_id);
    let experiments = repository
        .list_retrieval_eval_experiments_for_dataset(user.workspace.id, dataset_id)
        .await?;
    Ok(Json(build_trend_summary(
        dataset_id,
        &experiments,
        query.limit,
    )))
}

pub async fn get_experiment(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(experiment_id): Path<Uuid>,
) -> Result<Json<RetrievalEvalExperiment>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository
            .get_retrieval_eval_experiment(
                user.workspace.id,
                RetrievalEvalExperimentId(experiment_id),
            )
            .await
            .map_err(not_found_to_api("eval experiment"))?,
    ))
}

pub async fn experiment_regression(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(experiment_id): Path<Uuid>,
    Query(query): Query<RegressionQuery>,
) -> Result<Json<RetrievalEvalRegressionComparison>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let workspace_id = user.workspace.id;
    let current = repository
        .get_retrieval_eval_experiment(workspace_id, RetrievalEvalExperimentId(experiment_id))
        .await
        .map_err(not_found_to_api("eval experiment"))?;
    let baseline = if let Some(baseline_id) = query.baseline_id {
        let baseline = repository
            .get_retrieval_eval_experiment(workspace_id, RetrievalEvalExperimentId(baseline_id))
            .await
            .map_err(not_found_to_api("baseline eval experiment"))?;
        if baseline.dataset_id != current.dataset_id {
            return Err(ApiError::BadRequest(
                "baseline experiment must belong to the same dataset".to_owned(),
            ));
        }
        Some(baseline)
    } else {
        None
    };
    let dataset_experiments = repository
        .list_retrieval_eval_experiments_for_dataset(workspace_id, current.dataset_id)
        .await?;
    let experiment_refs = dataset_experiments.iter().collect::<Vec<_>>();
    let baseline_ref = baseline
        .as_ref()
        .or_else(|| previous_comparable_experiment(&current, &experiment_refs));

    Ok(Json(compare_experiment_regression(&current, baseline_ref)))
}

pub async fn compare_experiment(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(experiment_id): Path<Uuid>,
    Json(request): Json<CompareRetrievalEvalExperimentRequest>,
) -> Result<Json<rag_debugger_core::RetrievalEvalComparison>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let experiment = repository
        .get_retrieval_eval_experiment(user.workspace.id, RetrievalEvalExperimentId(experiment_id))
        .await
        .map_err(not_found_to_api("eval experiment"))?;
    let results = if request.modes.is_empty() {
        experiment.mode_results
    } else {
        experiment
            .mode_results
            .into_iter()
            .filter(|result| request.modes.contains(&result.retrieval_mode))
            .collect()
    };
    Ok(Json(compare_mode_results(&results)))
}

async fn build_evidence_response(
    repository: &(impl EvidenceRepository + ?Sized),
    workspace_id: WorkspaceId,
    request: &QueryEvalLabEvidenceRequest,
) -> Result<QueryEvalLabEvidenceResponse, ApiError> {
    validate_evidence_request_counts(Some(&request.document_ids), Some(&request.chunk_ids))?;
    let query = evidence_search_query(request.query.as_deref())?;
    let document_ids = dedupe_document_ids(request.document_ids.clone());
    let chunk_ids = dedupe_chunk_ids(request.chunk_ids.clone());
    let mut documents = repository
        .resolve_evidence_documents(workspace_id, &document_ids)
        .await?;
    let mut chunks = repository
        .resolve_evidence_chunks(workspace_id, &chunk_ids)
        .await?;
    let found_document_ids = documents
        .iter()
        .map(|document| document.id)
        .collect::<HashSet<_>>();
    let found_chunk_ids = chunks.iter().map(|chunk| chunk.id).collect::<HashSet<_>>();
    let unresolved_document_ids = document_ids
        .iter()
        .copied()
        .filter(|document_id| !found_document_ids.contains(document_id))
        .collect();
    let unresolved_chunk_ids = chunk_ids
        .iter()
        .copied()
        .filter(|chunk_id| !found_chunk_ids.contains(chunk_id))
        .collect();

    let document_limit = candidate_limit(request.document_limit, request.limit);
    let chunk_limit = if request.include_chunks {
        candidate_limit(request.chunk_limit, request.limit)
    } else {
        0
    };
    if document_limit > 0 || chunk_limit > 0 {
        let candidates = repository
            .search_evidence(
                workspace_id,
                &EvalLabEvidenceSearchRequest {
                    query,
                    excluded_document_ids: document_ids,
                    excluded_chunk_ids: chunk_ids,
                    document_limit,
                    chunk_limit,
                },
            )
            .await?;
        let mut seen_document_ids = found_document_ids;
        for candidate in candidates.documents {
            if seen_document_ids.insert(candidate.id) {
                documents.push(candidate);
            }
        }
        let mut seen_chunk_ids = found_chunk_ids;
        for candidate in candidates.chunks {
            if seen_chunk_ids.insert(candidate.id) {
                chunks.push(candidate);
            }
        }
    }

    Ok(QueryEvalLabEvidenceResponse {
        documents,
        chunks,
        unresolved_document_ids,
        unresolved_chunk_ids,
    })
}

fn evidence_search_query(query: Option<&str>) -> Result<EvalLabEvidenceSearchQuery, ApiError> {
    let query = query.map(str::trim).unwrap_or_default();
    if query.is_empty() {
        return Ok(EvalLabEvidenceSearchQuery::Browse);
    }
    if let Ok(id) = Uuid::parse_str(query) {
        return Ok(EvalLabEvidenceSearchQuery::ExactId(id));
    }
    if query.chars().count() < EVAL_LAB_EVIDENCE_MIN_TEXT_QUERY_CHARS {
        return Err(ApiError::BadRequest(
            "Enter at least 3 characters, paste an exact UUID, or leave blank to browse."
                .to_owned(),
        ));
    }
    Ok(EvalLabEvidenceSearchQuery::Text(query.to_lowercase()))
}

fn validate_evidence_request_counts(
    document_ids: Option<&[DocumentId]>,
    chunk_ids: Option<&[ChunkId]>,
) -> Result<(), ApiError> {
    let document_count = document_ids.map_or(0, <[DocumentId]>::len);
    let chunk_count = chunk_ids.map_or(0, <[ChunkId]>::len);
    if document_count > EVAL_LAB_EVIDENCE_MAX_REQUESTED_DOCUMENTS {
        return Err(ApiError::BadRequest(format!(
            "Too many requested document IDs; maximum is {EVAL_LAB_EVIDENCE_MAX_REQUESTED_DOCUMENTS}."
        )));
    }
    if chunk_count > EVAL_LAB_EVIDENCE_MAX_REQUESTED_CHUNKS {
        return Err(ApiError::BadRequest(format!(
            "Too many requested chunk IDs; maximum is {EVAL_LAB_EVIDENCE_MAX_REQUESTED_CHUNKS}."
        )));
    }
    if document_count + chunk_count > EVAL_LAB_EVIDENCE_MAX_REQUESTED_IDS {
        return Err(ApiError::BadRequest(format!(
            "Too many requested evidence IDs; combined maximum is {EVAL_LAB_EVIDENCE_MAX_REQUESTED_IDS}."
        )));
    }
    Ok(())
}

fn candidate_limit(specific: Option<u32>, legacy: Option<u32>) -> u32 {
    specific
        .map(|limit| limit.min(EVAL_LAB_EVIDENCE_MAX_CANDIDATE_LIMIT))
        .unwrap_or_else(|| {
            legacy
                .unwrap_or(EVAL_LAB_EVIDENCE_DEFAULT_CANDIDATE_LIMIT)
                .clamp(1, EVAL_LAB_EVIDENCE_MAX_CANDIDATE_LIMIT)
        })
}

fn eval_case_from_request(
    request: CreateRetrievalEvalLabCaseRequest,
    default_top_k: u32,
    max_top_k: u32,
) -> Result<RetrievalEvalCase, ApiError> {
    let query = request.query.trim().to_owned();
    if query.is_empty() {
        return Err(ApiError::BadRequest(
            "eval query must not be empty".to_owned(),
        ));
    }
    if request.expected_chunk_ids.is_empty() && request.expected_document_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "eval case needs at least one expected chunk or document".to_owned(),
        ));
    }

    Ok(RetrievalEvalCase {
        id: RetrievalEvalCaseId(Uuid::now_v7()),
        name: request
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| query.clone()),
        query,
        top_k: normalized_top_k(request.top_k, default_top_k, max_top_k),
        expected_chunk_ids: dedupe_chunk_ids(request.expected_chunk_ids),
        expected_document_ids: dedupe_document_ids(request.expected_document_ids),
        notes: request.notes,
        created_at: OffsetDateTime::now_utc(),
    })
}

fn dedupe_chunk_ids(ids: Vec<ChunkId>) -> Vec<ChunkId> {
    dedupe_ids(ids)
}

fn dedupe_document_ids(ids: Vec<DocumentId>) -> Vec<DocumentId> {
    dedupe_ids(ids)
}

fn dedupe_ids<T>(ids: Vec<T>) -> Vec<T>
where
    T: Copy + Eq + Hash,
{
    let mut seen = HashSet::with_capacity(ids.len());
    let mut deduped = Vec::with_capacity(ids.len());
    for id in ids {
        if seen.insert(id) {
            deduped.push(id);
        }
    }
    deduped
}

#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RegressionQuery {
    pub baseline_id: Option<Uuid>,
}

fn merge_case_update(
    current: RetrievalEvalCase,
    request: UpdateRetrievalEvalCaseRequest,
    default_top_k: u32,
    max_top_k: u32,
) -> Result<RetrievalEvalCase, ApiError> {
    let query = request
        .query
        .map(|query| query.trim().to_owned())
        .unwrap_or_else(|| current.query.clone());
    if query.is_empty() {
        return Err(ApiError::BadRequest(
            "eval query must not be empty".to_owned(),
        ));
    }
    let expected_chunk_ids = request
        .expected_chunk_ids
        .unwrap_or_else(|| current.expected_chunk_ids.clone());
    let expected_document_ids = request
        .expected_document_ids
        .unwrap_or_else(|| current.expected_document_ids.clone());

    Ok(RetrievalEvalCase {
        name: request
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(current.name),
        query,
        top_k: request.top_k.map_or(current.top_k, |top_k| {
            normalized_top_k(top_k, default_top_k, max_top_k)
        }),
        expected_chunk_ids,
        expected_document_ids,
        notes: request.notes.unwrap_or(current.notes),
        ..current
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

fn eval_case_write_error(error: rag_debugger_storage::StorageError) -> ApiError {
    match error {
        rag_debugger_storage::StorageError::UnavailableEvidence => ApiError::BadRequest(
            "Some selected evidence is unavailable. Remove or replace stale evidence before saving."
                .to_owned(),
        ),
        rag_debugger_storage::StorageError::NotFound => {
            ApiError::NotFound("eval case not found".to_owned())
        }
        other => ApiError::Storage(other),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };

    use async_trait::async_trait;
    use rag_debugger_core::{
        EvalLabEvidenceChunk, EvalLabEvidenceDocument, EvalLabEvidenceSearchResult,
    };
    use rag_debugger_storage::StorageError;

    use super::*;

    #[derive(Default)]
    struct CountingEvidenceRepository {
        document_resolutions: AtomicUsize,
        chunk_resolutions: AtomicUsize,
        searches: AtomicUsize,
        search_requests: Mutex<Vec<EvalLabEvidenceSearchRequest>>,
    }

    #[async_trait]
    impl EvidenceRepository for CountingEvidenceRepository {
        async fn resolve_evidence_documents(
            &self,
            _workspace_id: WorkspaceId,
            _document_ids: &[DocumentId],
        ) -> Result<Vec<EvalLabEvidenceDocument>, StorageError> {
            self.document_resolutions.fetch_add(1, Ordering::Relaxed);
            Ok(Vec::new())
        }

        async fn resolve_evidence_chunks(
            &self,
            _workspace_id: WorkspaceId,
            _chunk_ids: &[ChunkId],
        ) -> Result<Vec<EvalLabEvidenceChunk>, StorageError> {
            self.chunk_resolutions.fetch_add(1, Ordering::Relaxed);
            Ok(Vec::new())
        }

        async fn search_evidence(
            &self,
            _workspace_id: WorkspaceId,
            request: &EvalLabEvidenceSearchRequest,
        ) -> Result<EvalLabEvidenceSearchResult, StorageError> {
            self.searches.fetch_add(1, Ordering::Relaxed);
            self.search_requests
                .lock()
                .expect("search requests lock")
                .push(request.clone());
            Ok(EvalLabEvidenceSearchResult::default())
        }
    }

    #[tokio::test]
    async fn evidence_response_uses_three_fixed_repository_calls() {
        let repository = CountingEvidenceRepository::default();
        let document_id = DocumentId(Uuid::now_v7());
        let chunk_id = ChunkId(Uuid::now_v7());

        let response = build_evidence_response(
            &repository,
            test_workspace_id(),
            &QueryEvalLabEvidenceRequest {
                query: Some("account recovery".to_owned()),
                document_ids: vec![document_id],
                chunk_ids: vec![chunk_id],
                limit: None,
                document_limit: Some(10),
                chunk_limit: Some(10),
                include_chunks: true,
            },
        )
        .await
        .expect("bounded evidence response");

        assert_eq!(response.unresolved_document_ids, vec![document_id]);
        assert_eq!(response.unresolved_chunk_ids, vec![chunk_id]);
        assert_eq!(repository.document_resolutions.load(Ordering::Relaxed), 1);
        assert_eq!(repository.chunk_resolutions.load(Ordering::Relaxed), 1);
        assert_eq!(repository.searches.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn evidence_response_rejects_oversized_work_before_repository_access() {
        let repository = CountingEvidenceRepository::default();
        let repeated_document = DocumentId(Uuid::now_v7());
        let request = QueryEvalLabEvidenceRequest {
            query: None,
            document_ids: vec![repeated_document; EVAL_LAB_EVIDENCE_MAX_REQUESTED_DOCUMENTS + 1],
            chunk_ids: Vec::new(),
            limit: None,
            document_limit: Some(0),
            chunk_limit: Some(0),
            include_chunks: true,
        };

        let error = build_evidence_response(&repository, test_workspace_id(), &request)
            .await
            .expect_err("duplicates count toward request work");

        assert!(matches!(error, ApiError::BadRequest(_)));
        assert_eq!(repository.document_resolutions.load(Ordering::Relaxed), 0);
        assert_eq!(repository.chunk_resolutions.load(Ordering::Relaxed), 0);
        assert_eq!(repository.searches.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn evidence_response_accepts_exact_combined_request_limit() {
        let repository = CountingEvidenceRepository::default();
        let document_ids = (0..EVAL_LAB_EVIDENCE_MAX_REQUESTED_DOCUMENTS)
            .map(|value| DocumentId(Uuid::from_u128(value as u128 + 1)))
            .collect::<Vec<_>>();
        let chunk_ids = (0..(EVAL_LAB_EVIDENCE_MAX_REQUESTED_IDS - document_ids.len()))
            .map(|value| ChunkId(Uuid::from_u128(value as u128 + 1_000)))
            .collect::<Vec<_>>();

        let response = build_evidence_response(
            &repository,
            test_workspace_id(),
            &QueryEvalLabEvidenceRequest {
                query: None,
                document_ids: document_ids.clone(),
                chunk_ids: chunk_ids.clone(),
                limit: None,
                document_limit: Some(0),
                chunk_limit: Some(0),
                include_chunks: true,
            },
        )
        .await
        .expect("exact request-work limit is accepted");

        assert_eq!(response.unresolved_document_ids, document_ids);
        assert_eq!(response.unresolved_chunk_ids, chunk_ids);
        assert_eq!(repository.document_resolutions.load(Ordering::Relaxed), 1);
        assert_eq!(repository.chunk_resolutions.load(Ordering::Relaxed), 1);
        assert_eq!(repository.searches.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn evidence_response_rejects_short_text_before_repository_access() {
        let repository = CountingEvidenceRepository::default();
        let error = build_evidence_response(
            &repository,
            test_workspace_id(),
            &QueryEvalLabEvidenceRequest {
                query: Some("éa".to_owned()),
                document_ids: Vec::new(),
                chunk_ids: Vec::new(),
                limit: None,
                document_limit: Some(1),
                chunk_limit: Some(1),
                include_chunks: true,
            },
        )
        .await
        .expect_err("two Unicode scalars are rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
        assert_eq!(repository.document_resolutions.load(Ordering::Relaxed), 0);
        assert_eq!(repository.chunk_resolutions.load(Ordering::Relaxed), 0);
        assert_eq!(repository.searches.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn evidence_response_routes_exact_uuid_searches_without_substrings() {
        let repository = CountingEvidenceRepository::default();
        let id = Uuid::now_v7();

        build_evidence_response(
            &repository,
            test_workspace_id(),
            &QueryEvalLabEvidenceRequest {
                query: Some(id.to_string()),
                document_ids: Vec::new(),
                chunk_ids: Vec::new(),
                limit: None,
                document_limit: Some(1),
                chunk_limit: Some(1),
                include_chunks: true,
            },
        )
        .await
        .expect("exact UUID search succeeds");

        let requests = repository
            .search_requests
            .lock()
            .expect("search requests lock");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].query, EvalLabEvidenceSearchQuery::ExactId(id));
    }

    fn test_workspace_id() -> WorkspaceId {
        WorkspaceId(Uuid::from_u128(1))
    }

    #[test]
    fn evidence_request_limits_are_independent_and_combined() {
        let documents = vec![DocumentId(Uuid::nil()); 100];
        let chunks = vec![ChunkId(Uuid::nil()); 150];
        assert!(validate_evidence_request_counts(Some(&documents), Some(&chunks)).is_ok());

        let maximum_chunks = vec![ChunkId(Uuid::nil()); 250];
        assert!(validate_evidence_request_counts(None, Some(&maximum_chunks)).is_ok());

        let too_many_chunks = vec![ChunkId(Uuid::nil()); 251];
        assert!(validate_evidence_request_counts(None, Some(&too_many_chunks)).is_err());

        let combined_over_limit = vec![ChunkId(Uuid::nil()); 151];
        assert!(
            validate_evidence_request_counts(Some(&documents), Some(&combined_over_limit)).is_err()
        );
    }
}
