use async_trait::async_trait;
use rag_debugger_core::{RetrievalRun, TraceId};

use crate::RagError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RetrievalRequest {
    pub trace_id: TraceId,
    pub query: String,
    pub top_k: u32,
}

#[async_trait]
pub trait Retriever: Send + Sync {
    async fn retrieve(&self, request: RetrievalRequest) -> Result<RetrievalRun, RagError>;
}

#[derive(Debug, Default)]
pub struct PlaceholderRetriever;

#[async_trait]
impl Retriever for PlaceholderRetriever {
    async fn retrieve(&self, _request: RetrievalRequest) -> Result<RetrievalRun, RagError> {
        Err(RagError::NotImplemented("retrieval engine"))
    }
}
