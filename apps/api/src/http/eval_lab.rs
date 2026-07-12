use axum::{
    extract::{Path, Query, State},
    Json,
};
use rag_debugger_core::{
    Chunk, ChunkId, CompareRetrievalEvalExperimentRequest, CreateRetrievalEvalDatasetRequest,
    CreateRetrievalEvalLabCaseRequest, DocumentId, EvalLabEvidenceChunk, EvalLabEvidenceDocument,
    QueryEvalLabEvidenceRequest, QueryEvalLabEvidenceResponse, RetrievalEvalCase,
    RetrievalEvalCaseId, RetrievalEvalConfigSnapshot, RetrievalEvalDataset, RetrievalEvalDatasetId,
    RetrievalEvalDatasetSummary, RetrievalEvalExperiment, RetrievalEvalExperimentId,
    RetrievalEvalExperimentSummary, RetrievalEvalRegressionComparison, RetrievalEvalRun,
    RetrievalEvalRunId, RetrievalEvalTrendSummary, RetrievalMode, RetrievalQueryRequest,
    RunRetrievalEvalExperimentRequest, SourceSummary, UpdateRetrievalEvalCaseRequest,
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
use rag_debugger_storage::repository::AppRepository;
use serde::Deserialize;
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

pub async fn query_evidence(
    State(state): State<AppState>,
    Json(request): Json<QueryEvalLabEvidenceRequest>,
) -> Result<Json<QueryEvalLabEvidenceResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(
        build_evidence_response(repository.as_ref(), &request).await?,
    ))
}

pub async fn create_case(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(mut request): Json<CreateRetrievalEvalLabCaseRequest>,
) -> Result<Json<RetrievalEvalCase>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    request.expected_chunk_ids = dedupe_chunk_ids(request.expected_chunk_ids);
    request.expected_document_ids = dedupe_document_ids(request.expected_document_ids);
    validate_expected_evidence(
        repository.as_ref(),
        &request.expected_chunk_ids,
        &request.expected_document_ids,
    )
    .await?;
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
    Json(mut request): Json<UpdateRetrievalEvalCaseRequest>,
) -> Result<Json<RetrievalEvalCase>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    if let Some(chunk_ids) = request.expected_chunk_ids.take() {
        let chunk_ids = dedupe_chunk_ids(chunk_ids);
        validate_expected_evidence(repository.as_ref(), &chunk_ids, &[]).await?;
        request.expected_chunk_ids = Some(chunk_ids);
    }
    if let Some(document_ids) = request.expected_document_ids.take() {
        let document_ids = dedupe_document_ids(document_ids);
        validate_expected_evidence(repository.as_ref(), &[], &document_ids).await?;
        request.expected_document_ids = Some(document_ids);
    }
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

pub async fn list_dataset_experiments(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> Result<Json<Vec<RetrievalEvalExperimentSummary>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let dataset_id = RetrievalEvalDatasetId(dataset_id);
    let experiments = repository
        .list_retrieval_eval_experiments_for_dataset(dataset_id)
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
    Path(dataset_id): Path<Uuid>,
    Query(query): Query<TrendQuery>,
) -> Result<Json<RetrievalEvalTrendSummary>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let dataset_id = RetrievalEvalDatasetId(dataset_id);
    let experiments = repository
        .list_retrieval_eval_experiments_for_dataset(dataset_id)
        .await?;
    Ok(Json(build_trend_summary(
        dataset_id,
        &experiments,
        query.limit,
    )))
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

pub async fn experiment_regression(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Query(query): Query<RegressionQuery>,
) -> Result<Json<RetrievalEvalRegressionComparison>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let current = repository
        .get_retrieval_eval_experiment(RetrievalEvalExperimentId(experiment_id))
        .await
        .map_err(not_found_to_api("eval experiment"))?;
    let baseline = if let Some(baseline_id) = query.baseline_id {
        let baseline = repository
            .get_retrieval_eval_experiment(RetrievalEvalExperimentId(baseline_id))
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
        .list_retrieval_eval_experiments_for_dataset(current.dataset_id)
        .await?;
    let experiment_refs = dataset_experiments.iter().collect::<Vec<_>>();
    let baseline_ref = baseline
        .as_ref()
        .or_else(|| previous_comparable_experiment(&current, &experiment_refs));

    Ok(Json(compare_experiment_regression(&current, baseline_ref)))
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

async fn build_evidence_response(
    repository: &dyn AppRepository,
    request: &QueryEvalLabEvidenceRequest,
) -> Result<QueryEvalLabEvidenceResponse, ApiError> {
    let sources = repository.list_sources().await?;
    let limit = request.limit.unwrap_or(25).clamp(1, 100) as usize;
    let query = normalized_search(request.query.as_deref());
    let document_ids = dedupe_document_ids(request.document_ids.clone());
    let chunk_ids = dedupe_chunk_ids(request.chunk_ids.clone());

    let mut documents = Vec::new();
    let mut chunks = Vec::new();
    let mut found_document_ids = Vec::new();
    let mut found_chunk_ids = Vec::new();
    let requested_documents = document_ids
        .iter()
        .map(|document_id| document_id.0)
        .collect::<Vec<_>>();
    let requested_chunks = chunk_ids
        .iter()
        .map(|chunk_id| chunk_id.0)
        .collect::<Vec<_>>();

    for source in &sources {
        for summary in &source.documents {
            let document = &summary.document;
            let requested_document = requested_documents.contains(&document.id.0);
            let document_matches = requested_document
                || query
                    .as_deref()
                    .is_none_or(|needle| document_matches_query(source, summary, needle));

            let should_fetch_chunks = request.include_chunks
                || !requested_chunks.is_empty()
                || (query.is_some() && chunks.len() < limit);
            let document_chunks = if should_fetch_chunks {
                repository.list_document_chunks(document.id).await?
            } else {
                Vec::new()
            };

            if requested_document {
                found_document_ids.push(document.id);
            }

            if document_matches && documents.len() < limit {
                documents.push(evidence_document(source, summary));
            }

            for chunk in document_chunks {
                let requested_chunk = requested_chunks.contains(&chunk.id.0);
                if requested_chunk {
                    found_chunk_ids.push(chunk.id);
                }
                if requested_chunk
                    || (request.include_chunks
                        && query.as_deref().is_none_or(|needle| {
                            chunk_matches_query(&chunk, source, summary, needle)
                        }))
                {
                    chunks.push(evidence_chunk(source, summary, &chunk));
                }
            }
        }
    }

    chunks.truncate(limit);
    let unresolved_document_ids = document_ids
        .into_iter()
        .filter(|document_id| !found_document_ids.contains(document_id))
        .collect();
    let unresolved_chunk_ids = chunk_ids
        .into_iter()
        .filter(|chunk_id| !found_chunk_ids.contains(chunk_id))
        .collect();

    Ok(QueryEvalLabEvidenceResponse {
        documents,
        chunks,
        unresolved_document_ids,
        unresolved_chunk_ids,
    })
}

async fn validate_expected_evidence(
    repository: &dyn AppRepository,
    chunk_ids: &[ChunkId],
    document_ids: &[DocumentId],
) -> Result<(), ApiError> {
    if chunk_ids.is_empty() && document_ids.is_empty() {
        return Ok(());
    }

    let response = build_evidence_response(
        repository,
        &QueryEvalLabEvidenceRequest {
            query: None,
            document_ids: document_ids.to_vec(),
            chunk_ids: chunk_ids.to_vec(),
            limit: Some((chunk_ids.len() + document_ids.len()).max(1) as u32),
            include_chunks: true,
        },
    )
    .await?;

    if response.unresolved_document_ids.is_empty() && response.unresolved_chunk_ids.is_empty() {
        Ok(())
    } else {
        Err(ApiError::BadRequest(
            "expected evidence is unavailable or outside this workspace".to_owned(),
        ))
    }
}

fn evidence_document(
    source: &SourceSummary,
    summary: &rag_debugger_core::DocumentSummary,
) -> EvalLabEvidenceDocument {
    EvalLabEvidenceDocument {
        id: summary.document.id,
        source_id: source.source.id,
        source_name: source.source.name.clone(),
        path: summary.document.path.clone(),
        profile: summary.document.profile,
        extraction_quality: summary.document.extraction_quality,
        warnings: summary.document.warnings.clone(),
        chunk_count: summary.chunk_count,
    }
}

fn evidence_chunk(
    source: &SourceSummary,
    summary: &rag_debugger_core::DocumentSummary,
    chunk: &Chunk,
) -> EvalLabEvidenceChunk {
    EvalLabEvidenceChunk {
        id: chunk.id,
        document_id: chunk.document_id,
        source_id: source.source.id,
        source_name: source.source.name.clone(),
        document_path: summary.document.path.clone(),
        ordinal: chunk.ordinal,
        text: chunk.text.clone(),
        token_count: chunk.token_count,
        checksum: chunk.checksum.clone(),
        section_title: chunk.section_title.clone(),
        quality_flags: chunk.quality_flags.clone(),
        is_duplicate: chunk.is_duplicate,
        text_density: chunk.text_density,
        evidence_score_hint: chunk.evidence_score_hint,
    }
}

fn document_matches_query(
    source: &SourceSummary,
    summary: &rag_debugger_core::DocumentSummary,
    needle: &str,
) -> bool {
    source.source.name.to_lowercase().contains(needle)
        || summary.document.path.to_lowercase().contains(needle)
        || summary.document.id.0.to_string().contains(needle)
}

fn chunk_matches_query(
    chunk: &Chunk,
    source: &SourceSummary,
    summary: &rag_debugger_core::DocumentSummary,
    needle: &str,
) -> bool {
    document_matches_query(source, summary, needle)
        || chunk.text.to_lowercase().contains(needle)
        || chunk
            .section_title
            .as_ref()
            .is_some_and(|section| section.to_lowercase().contains(needle))
        || chunk.id.0.to_string().contains(needle)
}

fn normalized_search(query: Option<&str>) -> Option<String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(str::to_lowercase)
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
    let mut deduped = Vec::with_capacity(ids.len());
    for id in ids {
        if !deduped.contains(&id) {
            deduped.push(id);
        }
    }
    deduped
}

fn dedupe_document_ids(ids: Vec<DocumentId>) -> Vec<DocumentId> {
    let mut deduped = Vec::with_capacity(ids.len());
    for id in ids {
        if !deduped.contains(&id) {
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
