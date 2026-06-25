use axum::{
    extract::{Path, State},
    Json,
};
use rag_debugger_core::{
    CreateTraceFromRetrievalRunRequest, RerunTraceRequest, RetrievalQueryRequest, Trace,
    TraceRerunResponse, TraceSummary,
};
use rag_debugger_rag::{
    embedding::LocalHashEmbeddingProvider,
    retrieval::LocalHybridRetriever,
    tracing::{build_rerun_comparison, build_trace_from_retrieval},
    RagError,
};
use rag_debugger_storage::StorageError;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub async fn list_traces(
    State(state): State<AppState>,
) -> Result<Json<Vec<TraceSummary>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(repository.list_traces().await?))
}

pub async fn get_trace(
    State(state): State<AppState>,
    Path(trace_id): Path<Uuid>,
) -> Result<Json<Trace>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let trace = repository
        .get_trace_detail(rag_debugger_core::TraceId(trace_id))
        .await
        .map_err(trace_storage_error)?;

    Ok(Json(trace))
}

pub async fn create_trace_from_retrieval_run(
    State(state): State<AppState>,
    Json(request): Json<CreateTraceFromRetrievalRunRequest>,
) -> Result<Json<Trace>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let project = repository.ensure_default_project().await?;
    let response = match request.run_id {
        Some(run_id) => repository
            .get_retrieval_query(run_id)
            .await
            .map_err(trace_storage_error)?,
        None => repository.latest_retrieval_query().await.map_err(|error| {
            if matches!(error, StorageError::NotFound) {
                ApiError::BadRequest("run a retrieval query before saving a trace".to_owned())
            } else {
                ApiError::Storage(error)
            }
        })?,
    };

    let trace = build_trace_from_retrieval(project.id, response);
    Ok(Json(repository.save_trace(trace).await?))
}

pub async fn rerun_trace(
    State(state): State<AppState>,
    Path(trace_id): Path<Uuid>,
    Json(request): Json<RerunTraceRequest>,
) -> Result<Json<TraceRerunResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let mut trace = repository
        .get_trace_detail(rag_debugger_core::TraceId(trace_id))
        .await
        .map_err(trace_storage_error)?;
    let original = trace.retrieval.clone().ok_or_else(|| {
        ApiError::BadRequest("trace does not include a retrieval response".to_owned())
    })?;

    let query_request = RetrievalQueryRequest {
        query: original.run.query.clone(),
        top_k: request.top_k.unwrap_or(original.run.top_k),
        retrieval_mode: request
            .retrieval_mode
            .unwrap_or(original.run.retrieval_mode),
        source_ids: request.source_ids,
        document_ids: request.document_ids,
    };

    let candidates = repository.list_searchable_chunks(&query_request).await?;
    let retriever = LocalHybridRetriever::new(
        LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone()),
        state.config().product.retrieval.clone(),
    );
    let response = retriever
        .retrieve(query_request.clone(), candidates)
        .map_err(rag_error_to_api_error)?;
    repository.save_retrieval_query(&response).await?;

    let comparison = build_rerun_comparison(&original, query_request, response);
    trace.reruns.push(comparison.clone());
    trace.summary = format!(
        "Reran this trace {} time(s); latest run changed top score by {:+.2}.",
        trace.reruns.len(),
        comparison.score_delta
    );
    let trace = repository.save_trace(trace).await?;

    Ok(Json(TraceRerunResponse { trace, comparison }))
}

fn trace_storage_error(error: StorageError) -> ApiError {
    match error {
        StorageError::NotFound => ApiError::NotFound("trace resource was not found".to_owned()),
        other => ApiError::Storage(other),
    }
}

fn rag_error_to_api_error(error: RagError) -> ApiError {
    match error {
        RagError::InvalidConfig(message) => ApiError::BadRequest(message.to_owned()),
        RagError::NotImplemented(_) => ApiError::Internal,
    }
}
