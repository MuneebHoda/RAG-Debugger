use axum::{extract::State, Json};
use rag_debugger_core::{RetrievalQueryRequest, RetrievalQueryResponse};
use rag_debugger_rag::{
    embedding::LocalHashEmbeddingProvider, retrieval::LocalHybridRetriever, RagError,
};

use crate::{error::ApiError, state::AppState};

pub async fn query_retrieval(
    State(state): State<AppState>,
    Json(request): Json<RetrievalQueryRequest>,
) -> Result<Json<RetrievalQueryResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    validate_request(&request)?;

    let candidates = repository.list_searchable_chunks(&request).await?;
    let retriever = LocalHybridRetriever::new(
        LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone()),
        state.config().product.retrieval.clone(),
    );
    let response = retriever
        .retrieve(request, candidates)
        .map_err(rag_error_to_api_error)?;
    repository.save_retrieval_query(&response).await?;

    Ok(Json(response))
}

fn validate_request(request: &RetrievalQueryRequest) -> Result<(), ApiError> {
    if request.query.trim().is_empty() {
        return Err(ApiError::BadRequest("query must not be empty".to_owned()));
    }

    Ok(())
}

fn rag_error_to_api_error(error: RagError) -> ApiError {
    match error {
        RagError::InvalidConfig(message) => ApiError::BadRequest(message.to_owned()),
        RagError::NotImplemented(_) => ApiError::Internal,
    }
}
