pub mod chunking;
pub mod ingestion;
pub mod retrieval;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RagError {
    #[error("invalid RAG config: {0}")]
    InvalidConfig(&'static str),
    #[error("RAG operation is not implemented yet: {0}")]
    NotImplemented(&'static str),
}
