use axum::{
    extract::{Extension, State},
    Json,
};
use rag_debugger_core::{
    AuthenticatedUser, ChunkEmbedding, EmbeddingIndexRequest, EmbeddingIndexResponse,
    EmbeddingStatus,
};
use rag_debugger_rag::{
    embedding::{EmbeddingProvider, LocalHashEmbeddingProvider},
    RagError,
};
use time::OffsetDateTime;

use crate::{error::ApiError, state::AppState};

pub async fn embedding_status(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<EmbeddingStatus>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let provider = LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone());
    let status = repository
        .embedding_status(
            user.workspace.id,
            &EmbeddingIndexRequest::default(),
            &provider.model(),
        )
        .await?;

    Ok(Json(status))
}

pub async fn index_embeddings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<EmbeddingIndexRequest>,
) -> Result<Json<EmbeddingIndexResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let provider = LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone());
    let model = provider.model();
    let candidates = repository
        .list_embedding_candidates(user.workspace.id, &request)
        .await?;
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let vectors = provider.embed(&texts).map_err(rag_error_to_api_error)?;
    let indexed_at = OffsetDateTime::now_utc();
    let embeddings = candidates
        .into_iter()
        .zip(vectors)
        .map(|(candidate, vector)| ChunkEmbedding {
            chunk_id: candidate.chunk_id,
            chunk_checksum: candidate.checksum,
            model: model.clone(),
            vector,
            indexed_at,
        })
        .collect::<Vec<_>>();
    let indexed_chunks = embeddings.len() as u32;

    repository
        .upsert_chunk_embeddings(user.workspace.id, embeddings)
        .await?;
    let status = repository
        .embedding_status(user.workspace.id, &request, &model)
        .await?;

    Ok(Json(EmbeddingIndexResponse {
        status,
        indexed_chunks,
    }))
}

fn rag_error_to_api_error(error: RagError) -> ApiError {
    match error {
        RagError::InvalidConfig(message) => ApiError::BadRequest(message.to_owned()),
        RagError::NotImplemented(_) => ApiError::Internal,
    }
}
