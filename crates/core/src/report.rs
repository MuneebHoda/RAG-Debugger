use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::WorkspaceId,
    chunk::{ChunkId, ChunkQualityFlag},
    ci_eval::CiEvalRunId,
    eval::RetrievalEvalExperimentId,
    project::ProjectId,
    retrieval::{
        EvidenceStrength, RetrievalCitation, RetrievalMode, RetrievalQualityFlag,
        RetrievalQueryRunId,
    },
    source::{DocumentId, SourceId},
    trace::TraceId,
};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DebugReport {
    pub id: DebugReportId,
    pub workspace_id: WorkspaceId,
    pub project_id: ProjectId,
    pub title: String,
    pub subject: String,
    pub source: DebugReportSource,
    pub privacy_mode: DebugReportPrivacyMode,
    pub executive_summary: String,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    #[serde(default)]
    pub findings: Vec<DebugReportFinding>,
    #[serde(default)]
    pub recommendations: Vec<DebugReportRecommendation>,
    #[serde(default)]
    pub evidence: Vec<DebugReportEvidenceRef>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct DebugReportId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DebugReportSource {
    Trace {
        trace_id: TraceId,
    },
    EvalExperiment {
        experiment_id: RetrievalEvalExperimentId,
    },
    CiEvalRun {
        run_id: CiEvalRunId,
    },
    Manual {
        label: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DebugReportPrivacyMode {
    MetadataOnly,
    SnippetsAllowed,
    FullLocalOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DebugReportFinding {
    pub code: String,
    pub severity: DebugReportSeverity,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub failure_labels: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DebugReportSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DebugReportRecommendation {
    pub code: String,
    pub priority: DebugReportRecommendationPriority,
    pub area: DebugReportRecommendationArea,
    pub title: String,
    pub rationale: String,
    pub action: String,
    #[serde(default)]
    pub finding_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DebugReportRecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DebugReportRecommendationArea {
    Chunking,
    Embeddings,
    TopK,
    RetrievalMode,
    Reranking,
    MetadataFilters,
    Citations,
    CorpusCoverage,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DebugReportEvidenceRef {
    pub label: String,
    pub role: DebugReportEvidenceRole,
    pub source_id: Option<SourceId>,
    pub document_id: Option<DocumentId>,
    pub chunk_id: Option<ChunkId>,
    pub rank: Option<u32>,
    pub document_path: Option<String>,
    pub section_title: Option<String>,
    pub checksum_prefix: Option<String>,
    pub citation_label: Option<String>,
    pub snippet: Option<String>,
    pub evidence_strength: Option<EvidenceStrength>,
    #[serde(default)]
    pub chunk_quality_flags: Vec<ChunkQualityFlag>,
    #[serde(default)]
    pub retrieval_quality_flags: Vec<RetrievalQualityFlag>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DebugReportEvidenceRole {
    Retrieved,
    Expected,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalReport {
    pub id: RetrievalReportId,
    pub run_id: Option<RetrievalQueryRunId>,
    pub title: String,
    pub diagnosis: RetrievalDiagnosis,
    pub evidence: Vec<RetrievalCitation>,
    pub issues: Vec<EvidenceIssue>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalDiagnosis {
    pub query: String,
    pub retrieval_mode: RetrievalMode,
    pub summary: String,
    pub confidence: DiagnosisConfidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EvidenceIssue {
    pub severity: EvidenceIssueSeverity,
    pub code: String,
    pub message: String,
    pub source_id: Option<SourceId>,
    pub document_id: Option<DocumentId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceIssueSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalReportId(pub Uuid);
