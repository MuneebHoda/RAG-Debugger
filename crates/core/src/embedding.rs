use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    chunk::{ChunkId, ChunkingStrategy},
    source::{DocumentId, SourceId},
};

pub const DEFAULT_EMBEDDING_PROVIDER: &str = "local";
pub const DEFAULT_EMBEDDING_MODEL_NAME: &str = "local-hash-v1";
pub const DEFAULT_EMBEDDING_DIMENSION: u32 = 384;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EmbeddingModelInfo {
    pub provider: String,
    pub model_name: String,
    pub dimension: u32,
}

impl Default for EmbeddingModelInfo {
    fn default() -> Self {
        Self {
            provider: DEFAULT_EMBEDDING_PROVIDER.to_owned(),
            model_name: DEFAULT_EMBEDDING_MODEL_NAME.to_owned(),
            dimension: DEFAULT_EMBEDDING_DIMENSION,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkEmbedding {
    pub chunk_id: ChunkId,
    pub chunk_checksum: String,
    pub model: EmbeddingModelInfo,
    pub vector: Vec<f32>,
    pub indexed_at: OffsetDateTime,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EmbeddingIndexRequest {
    #[serde(default)]
    pub source_ids: Vec<SourceId>,
    #[serde(default)]
    pub document_ids: Vec<DocumentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EmbeddingStatus {
    pub model: EmbeddingModelInfo,
    pub total_chunks: u32,
    pub indexed_chunks: u32,
    pub missing_chunks: u32,
    pub stale_chunks: u32,
    pub last_indexed_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EmbeddingIndexResponse {
    pub status: EmbeddingStatus,
    pub indexed_chunks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EmbeddingIndexCandidate {
    pub chunk_id: ChunkId,
    pub source_id: SourceId,
    pub document_id: DocumentId,
    pub text: String,
    pub checksum: String,
    pub chunking_strategy: ChunkingStrategy,
}
