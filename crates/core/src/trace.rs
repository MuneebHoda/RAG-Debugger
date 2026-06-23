use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{model::ModelConfigId, project::ProjectId, retrieval::RetrievalRun};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trace {
    pub id: TraceId,
    pub project_id: ProjectId,
    pub input: String,
    pub output: Option<String>,
    pub started_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub retrieval_runs: Vec<RetrievalRun>,
    pub generation: Option<GenerationSpan>,
    pub failure_labels: Vec<FailureLabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationSpan {
    pub model_config_id: ModelConfigId,
    pub prompt_version: Option<String>,
    pub latency_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_micros_usd: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum FailureLabel {
    MissingDocument,
    BadChunking,
    BadEmbedding,
    BadRanking,
    BadPrompt,
    UnsupportedQuestion,
    HallucinatedAnswer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct TraceId(pub Uuid);
