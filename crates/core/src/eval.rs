use serde::{Deserialize, Serialize};
use uuid::Uuid;

use time::OffsetDateTime;

use crate::{
    chunk::ChunkId,
    config::RetrievalWeights,
    diagnosis::EvidenceDiagnosisSummary,
    embedding::EmbeddingModelInfo,
    model::ModelConfigId,
    project::ProjectId,
    retrieval::{RetrievalMode, DEFAULT_RETRIEVAL_TOP_K},
    source::DocumentId,
    trace::TraceId,
};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EvalDataset {
    pub id: EvalDatasetId,
    pub project_id: ProjectId,
    pub name: String,
    pub cases: Vec<EvalCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EvalCase {
    pub id: EvalCaseId,
    pub question: String,
    pub expected_answer: Option<String>,
    pub required_source_refs: Vec<String>,
    pub rubric: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalRun {
    pub id: EvalRunId,
    pub dataset_id: EvalDatasetId,
    pub model_config_id: ModelConfigId,
    pub status: EvalRunStatus,
    pub results: Vec<EvalCaseResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalCaseResult {
    pub case_id: EvalCaseId,
    pub trace_id: Option<TraceId>,
    pub retrieval_recall_at_k: Option<f32>,
    pub mean_reciprocal_rank: Option<f32>,
    pub faithfulness_score: Option<f32>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum EvalRunStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EvalDatasetId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EvalCaseId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EvalRunId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RetrievalEvalCase {
    pub id: RetrievalEvalCaseId,
    pub name: String,
    pub query: String,
    pub top_k: u32,
    pub expected_chunk_ids: Vec<ChunkId>,
    pub expected_document_ids: Vec<DocumentId>,
    pub notes: Option<String>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalDataset {
    pub id: RetrievalEvalDatasetId,
    pub name: String,
    pub description: Option<String>,
    pub cases: Vec<RetrievalEvalCase>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::wire_time")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalDatasetSummary {
    pub id: RetrievalEvalDatasetId,
    pub name: String,
    pub description: Option<String>,
    pub case_count: u32,
    pub latest_experiment_id: Option<RetrievalEvalExperimentId>,
    pub latest_gate: Option<RetrievalEvalGate>,
    pub latest_average_recall_at_k: Option<f32>,
    pub latest_average_precision_at_k: Option<f32>,
    #[serde(with = "crate::wire_time")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateRetrievalEvalDatasetRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateRetrievalEvalLabCaseRequest {
    pub name: Option<String>,
    pub query: String,
    #[serde(default = "default_eval_top_k")]
    pub top_k: u32,
    #[serde(default)]
    pub expected_chunk_ids: Vec<ChunkId>,
    #[serde(default)]
    pub expected_document_ids: Vec<DocumentId>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct UpdateRetrievalEvalCaseRequest {
    pub name: Option<String>,
    pub query: Option<String>,
    pub top_k: Option<u32>,
    pub expected_chunk_ids: Option<Vec<ChunkId>>,
    pub expected_document_ids: Option<Vec<DocumentId>>,
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RunRetrievalEvalExperimentRequest {
    pub dataset_id: RetrievalEvalDatasetId,
    pub name: Option<String>,
    #[serde(default)]
    pub modes: Vec<RetrievalMode>,
    pub top_k: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct CompareRetrievalEvalExperimentRequest {
    #[serde(default)]
    pub modes: Vec<RetrievalMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalExperiment {
    pub id: RetrievalEvalExperimentId,
    pub dataset_id: RetrievalEvalDatasetId,
    pub dataset_name: String,
    pub name: String,
    pub modes: Vec<RetrievalMode>,
    pub top_k: u32,
    pub config_snapshot: RetrievalEvalConfigSnapshot,
    pub mode_results: Vec<RetrievalEvalModeResult>,
    pub comparison: RetrievalEvalComparison,
    pub gate: RetrievalEvalGate,
    pub failures: Vec<RetrievalEvalFailure>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalConfigSnapshot {
    pub top_k: u32,
    pub scoring_weights: RetrievalWeights,
    pub embedding_model: EmbeddingModelInfo,
    pub dataset_case_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalModeResult {
    pub retrieval_mode: RetrievalMode,
    pub case_count: u32,
    pub passed_count: u32,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    pub mean_reciprocal_rank: f32,
    pub citation_coverage: f32,
    pub weak_evidence_count: u32,
    pub missing_embedding_failures: u32,
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub case_results: Vec<RetrievalEvalCaseEvaluation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalCaseEvaluation {
    pub case_id: RetrievalEvalCaseId,
    pub query: String,
    pub top_k: u32,
    pub recall_at_k: f32,
    pub precision_at_k: f32,
    pub mrr: f32,
    pub top_hit_rank: Option<u32>,
    pub citation_coverage: f32,
    pub weak_evidence_count: u32,
    pub missing_embedding_failures: u32,
    pub passed: bool,
    pub expected_chunk_ids: Vec<ChunkId>,
    pub expected_document_ids: Vec<DocumentId>,
    pub retrieved_chunk_ids: Vec<ChunkId>,
    pub latency_ms: u64,
    pub failures: Vec<RetrievalEvalFailure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<EvidenceDiagnosisSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalComparison {
    pub best_mode: Option<RetrievalMode>,
    pub mode_count: u32,
    pub recall_delta: f32,
    pub precision_delta: f32,
    pub latency_delta_ms: i64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalGate {
    pub status: RetrievalEvalGateStatus,
    pub average_recall_at_k: f32,
    pub weak_evidence_rate: f32,
    pub critical_failure_count: u32,
    pub recall_threshold: f32,
    pub weak_evidence_limit: f32,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalGateStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalFailure {
    pub case_id: RetrievalEvalCaseId,
    pub query: String,
    pub retrieval_mode: RetrievalMode,
    pub label: RetrievalEvalFailureLabel,
    pub severity: RetrievalEvalFailureSeverity,
    pub message: String,
    pub top_hit_rank: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalFailureLabel {
    ExpectedEvidenceMissing,
    CorrectDocumentWrongChunk,
    LowPrecision,
    WeakEvidence,
    MissingEmbeddings,
    HeadingOnlyEvidence,
    DuplicateEvidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalFailureSeverity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateRetrievalEvalCaseRequest {
    pub name: Option<String>,
    pub query: String,
    #[serde(default = "default_eval_top_k")]
    pub top_k: u32,
    #[serde(default)]
    pub expected_chunk_ids: Vec<ChunkId>,
    #[serde(default)]
    pub expected_document_ids: Vec<DocumentId>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RunRetrievalEvalRequest {
    #[serde(default)]
    pub case_ids: Vec<RetrievalEvalCaseId>,
    #[serde(default)]
    pub retrieval_mode: RetrievalMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalRun {
    pub id: RetrievalEvalRunId,
    pub retrieval_mode: RetrievalMode,
    pub case_count: u32,
    pub passed_count: u32,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
    pub results: Vec<RetrievalEvalResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalResult {
    pub case_id: RetrievalEvalCaseId,
    pub query: String,
    pub top_k: u32,
    pub recall_at_k: f32,
    pub precision_at_k: f32,
    pub top_hit_rank: Option<u32>,
    pub passed: bool,
    pub expected_chunk_ids: Vec<ChunkId>,
    pub expected_document_ids: Vec<DocumentId>,
    pub retrieved_chunk_ids: Vec<ChunkId>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalCaseId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalRunId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalDatasetId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalExperimentId(pub Uuid);

fn default_eval_top_k() -> u32 {
    DEFAULT_RETRIEVAL_TOP_K
}
