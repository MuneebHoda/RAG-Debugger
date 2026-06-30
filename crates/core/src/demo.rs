use serde::{Deserialize, Serialize};

use crate::{DebugReportId, ProjectId, RetrievalQueryRunId, SourceId, TraceId};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DemoStatus {
    pub version: String,
    pub project_id: Option<ProjectId>,
    pub source_id: Option<SourceId>,
    pub progress: DemoProgress,
    pub suggested_queries: Vec<DemoSuggestedQuery>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct DemoProgress {
    pub sample_corpus_loaded: bool,
    pub chunks_created: bool,
    pub embeddings_indexed: bool,
    pub document_count: u32,
    pub chunk_count: u32,
    pub indexed_chunk_count: u32,
    pub retrieval_run_id: Option<RetrievalQueryRunId>,
    pub trace_id: Option<TraceId>,
    pub report_id: Option<DebugReportId>,
}

impl DemoProgress {
    pub fn is_complete(&self) -> bool {
        self.report_id.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DemoSuggestedQuery {
    pub id: DemoQueryId,
    pub question: String,
    pub description: String,
    pub recommended: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DemoQueryId {
    AccountRecovery,
    DataRetention,
    GpuIndexing,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DemoLoadResponse {
    pub created_documents: u32,
    pub status: DemoStatus,
}
