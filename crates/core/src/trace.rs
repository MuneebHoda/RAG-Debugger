use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    diagnosis::{EvidenceDiagnosisSummary, RerunDiagnosisSummary},
    model::ModelConfigId,
    project::ProjectId,
    retrieval::{
        EvidenceStrength, RetrievalEmbeddingReadiness, RetrievalMode, RetrievalQueryRequest,
        RetrievalQueryResponse, RetrievalQueryRunId, RetrievalRun,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trace {
    pub id: TraceId,
    pub project_id: ProjectId,
    pub input: String,
    pub output: Option<String>,
    #[serde(with = "crate::wire_time")]
    pub started_at: OffsetDateTime,
    #[serde(with = "crate::wire_time::option")]
    pub completed_at: Option<OffsetDateTime>,
    #[serde(default)]
    pub retrieval_runs: Vec<RetrievalRun>,
    pub generation: Option<GenerationSpan>,
    #[serde(default)]
    pub failure_labels: Vec<FailureLabel>,
    #[serde(default)]
    pub source_run_id: Option<RetrievalQueryRunId>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub status: TraceStatus,
    #[serde(default)]
    pub evidence_strength: Option<EvidenceStrength>,
    #[serde(default)]
    pub spans: Vec<TraceSpan>,
    #[serde(default)]
    pub retrieval: Option<RetrievalQueryResponse>,
    #[serde(default)]
    pub reruns: Vec<TraceRerunComparison>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<EvidenceDiagnosisSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceSummary {
    pub id: TraceId,
    pub query: String,
    pub retrieval_mode: RetrievalMode,
    pub latency_ms: u64,
    pub evidence_strength: EvidenceStrength,
    pub failure_labels: Vec<FailureLabel>,
    pub span_count: u32,
    pub rerun_count: u32,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    #[default]
    Completed,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceSpan {
    pub id: TraceSpanId,
    pub kind: TraceSpanKind,
    pub title: String,
    pub description: String,
    #[serde(with = "crate::wire_time")]
    pub started_at: OffsetDateTime,
    #[serde(with = "crate::wire_time::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub latency_ms: u64,
    pub status: TraceSpanStatus,
    pub detail: TraceSpanDetail,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct TraceSpanId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TraceSpanKind {
    QueryInput,
    Retrieval,
    EvidenceSummary,
    EvalCheck,
    Generation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TraceSpanStatus {
    Succeeded,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TraceSpanDetail {
    QueryInput {
        top_k: u32,
        retrieval_mode: RetrievalMode,
        source_filter_count: u32,
        document_filter_count: u32,
    },
    Retrieval {
        hit_count: u32,
        top_score: f32,
        embedding_readiness: RetrievalEmbeddingReadiness,
    },
    EvidenceSummary {
        answer_status: String,
        citation_count: u32,
        strongest_evidence: EvidenceStrength,
    },
    EvalCheck {
        checked: bool,
        passed: Option<bool>,
        message: String,
    },
    Generation {
        model: Option<String>,
        prompt_version: Option<String>,
        input_tokens: u32,
        output_tokens: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceRerunComparison {
    pub id: TraceRerunId,
    pub request: RetrievalQueryRequest,
    pub response: RetrievalQueryResponse,
    pub score_delta: f32,
    pub latency_delta_ms: i64,
    pub overlap_count: u32,
    pub changed_rank_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<RerunDiagnosisSummary>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct TraceRerunId(pub Uuid);

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateTraceFromRetrievalRunRequest {
    pub run_id: Option<RetrievalQueryRunId>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RerunTraceRequest {
    pub retrieval_mode: Option<RetrievalMode>,
    pub top_k: Option<u32>,
    #[serde(default)]
    pub source_ids: Vec<crate::source::SourceId>,
    #[serde(default)]
    pub document_ids: Vec<crate::source::DocumentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceRerunResponse {
    pub trace: Trace,
    pub comparison: TraceRerunComparison,
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
#[serde(rename_all = "snake_case")]
pub enum FailureLabel {
    MissingDocument,
    BadChunking,
    BadEmbedding,
    BadRanking,
    BadPrompt,
    UnsupportedQuestion,
    HallucinatedAnswer,
    WeakEvidence,
    MissingEmbeddingIndex,
    DuplicateEvidence,
    HeadingOnlyEvidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct TraceId(pub Uuid);
