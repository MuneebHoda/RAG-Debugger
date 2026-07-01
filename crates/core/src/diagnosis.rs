use serde::{Deserialize, Serialize};

use crate::{ChunkId, RetrievalScoreBreakdown};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisOutcome {
    Strong,
    Mixed,
    Weak,
    Failing,
}

impl DiagnosisOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Strong => "strong",
            Self::Mixed => "mixed",
            Self::Weak => "weak",
            Self::Failing => "failing",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisFailureCode {
    MissingDocument,
    MissingEmbeddingIndex,
    PartialEmbeddingIndex,
    WeakEvidence,
    DuplicateEvidence,
    HeadingOnlyEvidence,
    LowScoreMargin,
    VectorLexicalDisagreement,
    CitationMissing,
    TopResultNotCited,
    MissingExpectedEvidence,
    AnswerabilityGap,
    SemanticOnlyMatch,
    MetadataOnlyMatch,
}

impl DiagnosisFailureCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingDocument => "missing_document",
            Self::MissingEmbeddingIndex => "missing_embedding_index",
            Self::PartialEmbeddingIndex => "partial_embedding_index",
            Self::WeakEvidence => "weak_evidence",
            Self::DuplicateEvidence => "duplicate_evidence",
            Self::HeadingOnlyEvidence => "heading_only_evidence",
            Self::LowScoreMargin => "low_score_margin",
            Self::VectorLexicalDisagreement => "vector_lexical_disagreement",
            Self::CitationMissing => "citation_missing",
            Self::TopResultNotCited => "top_result_not_cited",
            Self::MissingExpectedEvidence => "missing_expected_evidence",
            Self::AnswerabilityGap => "answerability_gap",
            Self::SemanticOnlyMatch => "semantic_only_match",
            Self::MetadataOnlyMatch => "metadata_only_match",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisRemediationArea {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisRecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisScoreSignal {
    Semantic,
    Lexical,
    Phrase,
    Section,
    Path,
    Metadata,
    None,
}

impl DiagnosisScoreSignal {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Semantic => "semantic similarity",
            Self::Lexical => "lexical overlap",
            Self::Phrase => "phrase matching",
            Self::Section => "section-title matching",
            Self::Path => "document-path matching",
            Self::Metadata => "metadata quality",
            Self::None => "no individual signal",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DiagnosisFailure {
    pub code: DiagnosisFailureCode,
    pub severity: DiagnosisSeverity,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceScoreExplanation {
    pub evidence_ref: String,
    pub chunk_id: ChunkId,
    pub rank: u32,
    pub final_score: f32,
    pub score_delta_from_previous: Option<f32>,
    pub score_delta_to_next: Option<f32>,
    pub dominant_signal: DiagnosisScoreSignal,
    pub score_breakdown: RetrievalScoreBreakdown,
    pub normalized_score_breakdown: RetrievalScoreBreakdown,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DiagnosisRecommendation {
    pub code: String,
    pub priority: DiagnosisRecommendationPriority,
    pub area: DiagnosisRemediationArea,
    pub title: String,
    pub rationale: String,
    pub action: String,
    #[serde(default)]
    pub failure_codes: Vec<DiagnosisFailureCode>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceDiagnosisSummary {
    pub outcome: DiagnosisOutcome,
    pub summary: String,
    pub primary_issue: Option<DiagnosisFailure>,
    #[serde(default)]
    pub failures: Vec<DiagnosisFailure>,
    #[serde(default)]
    pub score_explanations: Vec<EvidenceScoreExplanation>,
    #[serde(default)]
    pub recommendations: Vec<DiagnosisRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerunDiagnosisSummary {
    pub before_outcome: DiagnosisOutcome,
    pub after_outcome: DiagnosisOutcome,
    pub summary: String,
    #[serde(default)]
    pub resolved_failures: Vec<DiagnosisFailureCode>,
    #[serde(default)]
    pub introduced_failures: Vec<DiagnosisFailureCode>,
    #[serde(default)]
    pub gained_evidence: Vec<ChunkId>,
    #[serde(default)]
    pub lost_evidence: Vec<ChunkId>,
    #[serde(default)]
    pub gained_citations: Vec<ChunkId>,
    #[serde(default)]
    pub lost_citations: Vec<ChunkId>,
}
