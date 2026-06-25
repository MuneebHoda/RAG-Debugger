use axum::{extract::State, Json};
use rag_debugger_core::{
    CreateRetrievalEvalCaseRequest, RetrievalEvalCase, RetrievalEvalRun, RetrievalEvalRunId,
    RetrievalQueryRequest, RunRetrievalEvalRequest,
};
use rag_debugger_rag::{
    embedding::LocalHashEmbeddingProvider, evals::score_retrieval_eval_case,
    retrieval::LocalHybridRetriever, RagError,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub async fn list_retrieval_eval_cases(
    State(state): State<AppState>,
) -> Result<Json<Vec<RetrievalEvalCase>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(repository.list_retrieval_eval_cases().await?))
}

pub async fn create_retrieval_eval_case(
    State(state): State<AppState>,
    Json(request): Json<CreateRetrievalEvalCaseRequest>,
) -> Result<Json<RetrievalEvalCase>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
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

    let top_k = normalized_top_k(
        request.top_k,
        state.config().product.retrieval.default_top_k,
        state.config().product.retrieval.max_top_k,
    );
    let eval_case = RetrievalEvalCase {
        id: rag_debugger_core::RetrievalEvalCaseId(Uuid::now_v7()),
        name: request
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| query.clone()),
        query,
        top_k,
        expected_chunk_ids: request.expected_chunk_ids,
        expected_document_ids: request.expected_document_ids,
        notes: request.notes,
        created_at: OffsetDateTime::now_utc(),
    };

    Ok(Json(
        repository.create_retrieval_eval_case(eval_case).await?,
    ))
}

pub async fn run_retrieval_evals(
    State(state): State<AppState>,
    Json(request): Json<RunRetrievalEvalRequest>,
) -> Result<Json<RetrievalEvalRun>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let cases = if request.case_ids.is_empty() {
        repository.list_retrieval_eval_cases().await?
    } else {
        repository
            .list_retrieval_eval_cases_by_id(&request.case_ids)
            .await?
    };

    if cases.is_empty() {
        return Err(ApiError::BadRequest("no eval cases found".to_owned()));
    }

    let retriever = LocalHybridRetriever::new(
        LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone()),
        state.config().product.retrieval.clone(),
    );
    let mut results = Vec::with_capacity(cases.len());

    for eval_case in &cases {
        let query_request = RetrievalQueryRequest {
            query: eval_case.query.clone(),
            top_k: normalized_top_k(
                eval_case.top_k,
                state.config().product.retrieval.default_top_k,
                state.config().product.retrieval.max_top_k,
            ),
            retrieval_mode: request.retrieval_mode,
            source_ids: Vec::new(),
            document_ids: Vec::new(),
        };
        let candidates = repository.list_searchable_chunks(&query_request).await?;
        let response = retriever
            .retrieve(query_request, candidates)
            .map_err(rag_error_to_api_error)?;
        results.push(score_retrieval_eval_case(eval_case, &response));
    }

    let case_count = results.len() as u32;
    let passed_count = results.iter().filter(|result| result.passed).count() as u32;
    let average_recall_at_k = average(results.iter().map(|result| result.recall_at_k));
    let average_precision_at_k = average(results.iter().map(|result| result.precision_at_k));
    let eval_run = RetrievalEvalRun {
        id: RetrievalEvalRunId(Uuid::now_v7()),
        retrieval_mode: request.retrieval_mode,
        case_count,
        passed_count,
        average_recall_at_k,
        average_precision_at_k,
        created_at: OffsetDateTime::now_utc(),
        results,
    };

    repository.save_retrieval_eval_run(&eval_run).await?;
    Ok(Json(eval_run))
}

fn normalized_top_k(top_k: u32, default_top_k: u32, max_top_k: u32) -> u32 {
    if top_k == 0 {
        default_top_k
    } else {
        top_k.min(max_top_k)
    }
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut total = 0.0;
    let mut count = 0u32;
    for value in values {
        total += value;
        count += 1;
    }

    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn rag_error_to_api_error(error: RagError) -> ApiError {
    match error {
        RagError::InvalidConfig(message) => ApiError::BadRequest(message.to_owned()),
        RagError::NotImplemented(_) => ApiError::Internal,
    }
}
