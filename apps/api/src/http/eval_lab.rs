use axum::{
    extract::{Path, State},
    Json,
};
use rag_debugger_core::{
    CompareRetrievalEvalExperimentRequest, CreateRetrievalEvalDatasetRequest,
    CreateRetrievalEvalLabCaseRequest, RetrievalEvalCase, RetrievalEvalCaseId,
    RetrievalEvalConfigSnapshot, RetrievalEvalDataset, RetrievalEvalDatasetId,
    RetrievalEvalDatasetSummary, RetrievalEvalExperiment, RetrievalEvalExperimentId,
    RetrievalEvalRun, RetrievalEvalRunId, RetrievalMode, RetrievalQueryRequest,
    RunRetrievalEvalExperimentRequest, UpdateRetrievalEvalCaseRequest,
};
use rag_debugger_rag::{
    embedding::LocalHashEmbeddingProvider,
    evals::{
        compare_mode_results, evaluate_gate, evaluate_retrieval_eval_case_with_config,
        summarize_mode_result,
    },
    retrieval::LocalHybridRetriever,
    RagError,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub async fn list_datasets(
    State(state): State<AppState>,
) -> Result<Json<Vec<RetrievalEvalDatasetSummary>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(repository.list_retrieval_eval_datasets().await?))
}

pub async fn create_dataset(
    State(state): State<AppState>,
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
        repository.create_retrieval_eval_dataset(dataset).await?,
    ))
}

pub async fn get_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> Result<Json<RetrievalEvalDataset>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository
            .get_retrieval_eval_dataset(RetrievalEvalDatasetId(dataset_id))
            .await
            .map_err(not_found_to_api("eval dataset"))?,
    ))
}

pub async fn create_case(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(request): Json<CreateRetrievalEvalLabCaseRequest>,
) -> Result<Json<RetrievalEvalCase>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let eval_case = eval_case_from_request(
        request,
        state.config().product.retrieval.default_top_k,
        state.config().product.retrieval.max_top_k,
    )?;
    Ok(Json(
        repository
            .create_retrieval_eval_case_in_dataset(RetrievalEvalDatasetId(dataset_id), eval_case)
            .await
            .map_err(not_found_to_api("eval dataset"))?,
    ))
}

pub async fn update_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(request): Json<UpdateRetrievalEvalCaseRequest>,
) -> Result<Json<RetrievalEvalCase>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let current = repository
        .list_retrieval_eval_cases()
        .await?
        .into_iter()
        .find(|eval_case| eval_case.id == RetrievalEvalCaseId(case_id))
        .ok_or_else(|| ApiError::NotFound("eval case not found".to_owned()))?;
    let updated = merge_case_update(
        current,
        request,
        state.config().product.retrieval.default_top_k,
        state.config().product.retrieval.max_top_k,
    )?;

    Ok(Json(
        repository
            .update_retrieval_eval_case(updated)
            .await
            .map_err(not_found_to_api("eval case"))?,
    ))
}

pub async fn delete_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    repository
        .delete_retrieval_eval_case(RetrievalEvalCaseId(case_id))
        .await
        .map_err(not_found_to_api("eval case"))?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn run_experiment(
    State(state): State<AppState>,
    Json(request): Json<RunRetrievalEvalExperimentRequest>,
) -> Result<Json<RetrievalEvalExperiment>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
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
            let candidates = repository.list_searchable_chunks(&query_request).await?;
            let response = retriever
                .retrieve(query_request, candidates)
                .map_err(rag_error_to_api_error)?;
            case_results.push(evaluate_retrieval_eval_case_with_config(
                &case,
                &response,
                &state.config().product.debugger,
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
        .save_retrieval_eval_experiment(experiment)
        .await?;
    if let Some(best_result) = saved.mode_results.iter().max_by(|left, right| {
        left.average_recall_at_k
            .partial_cmp(&right.average_recall_at_k)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        repository
            .save_retrieval_eval_run(&RetrievalEvalRun {
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
            })
            .await?;
    }

    Ok(Json(saved))
}

pub async fn list_experiments(
    State(state): State<AppState>,
) -> Result<Json<Vec<RetrievalEvalExperiment>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(repository.list_retrieval_eval_experiments().await?))
}

pub async fn get_experiment(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> Result<Json<RetrievalEvalExperiment>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        repository
            .get_retrieval_eval_experiment(RetrievalEvalExperimentId(experiment_id))
            .await
            .map_err(not_found_to_api("eval experiment"))?,
    ))
}

pub async fn compare_experiment(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Json(request): Json<CompareRetrievalEvalExperimentRequest>,
) -> Result<Json<rag_debugger_core::RetrievalEvalComparison>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let experiment = repository
        .get_retrieval_eval_experiment(RetrievalEvalExperimentId(experiment_id))
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
        expected_chunk_ids: request.expected_chunk_ids,
        expected_document_ids: request.expected_document_ids,
        notes: request.notes,
        created_at: OffsetDateTime::now_utc(),
    })
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
    if expected_chunk_ids.is_empty() && expected_document_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "eval case needs at least one expected chunk or document".to_owned(),
        ));
    }

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
