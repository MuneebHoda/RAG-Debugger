pub mod chunking;
pub mod embedding;
pub mod evals;
pub mod extraction;
pub mod ingestion;
pub mod intelligence;
pub mod reports;
pub mod retrieval;
pub mod tracing;

pub use extraction::{ExtractedText, ExtractionError, SupportedFileKind, TextExtractor};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RagError {
    #[error("invalid RAG config: {0}")]
    InvalidConfig(&'static str),
    #[error("RAG operation is not implemented yet: {0}")]
    NotImplemented(&'static str),
}
