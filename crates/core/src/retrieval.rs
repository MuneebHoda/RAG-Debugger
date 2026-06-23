use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{chunk::ChunkId, model::ModelConfigId, trace::TraceId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalRun {
    pub id: RetrievalRunId,
    pub trace_id: TraceId,
    pub query: String,
    pub rewritten_query: Option<String>,
    pub retriever: RetrieverKind,
    pub embedding_model_id: ModelConfigId,
    pub top_k: u32,
    pub hits: Vec<RetrievalHit>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalHit {
    pub chunk_id: ChunkId,
    pub rank: u32,
    pub score: f32,
    pub citation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum RetrieverKind {
    Vector,
    Keyword,
    Hybrid,
    Custom(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalRunId(pub Uuid);
