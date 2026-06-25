use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    retrieval::{RetrievalCitation, RetrievalMode, RetrievalQueryRunId},
    source::{DocumentId, SourceId},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalReport {
    pub id: RetrievalReportId,
    pub run_id: Option<RetrievalQueryRunId>,
    pub title: String,
    pub diagnosis: RetrievalDiagnosis,
    pub evidence: Vec<RetrievalCitation>,
    pub issues: Vec<EvidenceIssue>,
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
