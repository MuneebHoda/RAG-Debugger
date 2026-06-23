use async_trait::async_trait;
use rag_debugger_core::{Document, Source};

use crate::RagError;

#[async_trait]
pub trait SourceIngestor: Send + Sync {
    async fn discover(&self, source: &Source) -> Result<Vec<Document>, RagError>;
}

#[derive(Debug, Default)]
pub struct PlaceholderIngestor;

#[async_trait]
impl SourceIngestor for PlaceholderIngestor {
    async fn discover(&self, _source: &Source) -> Result<Vec<Document>, RagError> {
        Ok(Vec::new())
    }
}
