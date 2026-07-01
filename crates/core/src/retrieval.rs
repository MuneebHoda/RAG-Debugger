use serde::{Deserialize, Serialize};
use uuid::Uuid;

use time::OffsetDateTime;

use crate::{
    chunk::ChunkId,
    diagnosis::EvidenceDiagnosisSummary,
    embedding::{ChunkEmbedding, EmbeddingModelInfo},
    ingestion::ChunkPreview,
    model::ModelConfigId,
    source::{Document, DocumentId, Source, SourceId},
    trace::TraceId,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalRun {
    pub id: RetrievalRunId,
    pub trace_id: TraceId,
    pub query: String,
    pub rewritten_query: Option<String>,
    pub retriever: RetrieverKind,
    pub embedding_model_id: ModelConfigId,
    pub top_k: u32,
    pub hits: Vec<RetrievalHit>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalHit {
    pub chunk_id: ChunkId,
    pub rank: u32,
    pub score: f32,
    pub citation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum RetrieverKind {
    Vector,
    Keyword,
    Hybrid,
    Custom(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalRunId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RetrievalQueryRequest {
    pub query: String,
    #[serde(default = "default_retrieval_top_k")]
    pub top_k: u32,
    #[serde(default)]
    pub retrieval_mode: RetrievalMode,
    #[serde(default)]
    pub source_ids: Vec<SourceId>,
    #[serde(default)]
    pub document_ids: Vec<DocumentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalQueryResponse {
    pub run: RetrievalQueryRun,
    pub answer: ExtractiveAnswer,
    pub hits: Vec<RetrievalQueryHit>,
    pub embedding_status: RetrievalEmbeddingStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<EvidenceDiagnosisSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalQueryRun {
    pub id: RetrievalQueryRunId,
    pub query: String,
    pub top_k: u32,
    pub retrieval_mode: RetrievalMode,
    pub latency_ms: u64,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalQueryHit {
    pub rank: u32,
    pub score: f32,
    pub chunk: ChunkPreview,
    pub document: Document,
    pub source: Source,
    pub matched_terms: Vec<RetrievalMatchedTerm>,
    pub score_breakdown: RetrievalScoreBreakdown,
    pub normalized_score_breakdown: RetrievalScoreBreakdown,
    pub snippet: String,
    pub citation: RetrievalCitation,
    #[serde(default)]
    pub quality_flags: Vec<RetrievalQualityFlag>,
    pub evidence_strength: EvidenceStrength,
    pub duplicate_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RetrievalMatchedTerm {
    pub term: String,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RetrievalScoreBreakdown {
    pub semantic: f32,
    pub lexical: f32,
    pub phrase: f32,
    pub section: f32,
    pub path: f32,
    pub metadata: f32,
}

impl RetrievalScoreBreakdown {
    pub fn zero() -> Self {
        Self {
            semantic: 0.0,
            lexical: 0.0,
            phrase: 0.0,
            section: 0.0,
            path: 0.0,
            metadata: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalQualityFlag {
    Duplicate,
    HeadingOnly,
    TooShort,
    WeakEvidence,
    SemanticMatch,
    ExactTermMatch,
    SectionOnlyMatch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    Strong,
    Medium,
    Weak,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RetrievalCitation {
    pub label: String,
    pub chunk_id: ChunkId,
    pub document_id: DocumentId,
    pub document_path: String,
    pub chunk_ordinal: u32,
    pub section_title: Option<String>,
    pub checksum_prefix: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ExtractiveAnswer {
    pub status: ExtractiveAnswerStatus,
    pub text: String,
    pub citations: Vec<RetrievalCitation>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractiveAnswerStatus {
    Answered,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchableChunk {
    pub source: Source,
    pub document: Document,
    pub chunk: crate::chunk::Chunk,
    pub embedding: Option<ChunkEmbedding>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalQueryRunId(pub Uuid);

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    Lexical,
    Vector,
    #[default]
    Hybrid,
}

impl RetrievalMode {
    pub fn requires_embeddings(self) -> bool {
        matches!(self, Self::Vector | Self::Hybrid)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RetrievalEmbeddingStatus {
    pub readiness: RetrievalEmbeddingReadiness,
    pub required: bool,
    pub model: EmbeddingModelInfo,
    pub total_chunks: u32,
    pub indexed_chunks: u32,
    pub missing_chunks: u32,
    pub stale_chunks: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEmbeddingReadiness {
    NotRequired,
    Ready,
    Partial,
    Missing,
}

pub const DEFAULT_RETRIEVAL_TOP_K: u32 = 5;

fn default_retrieval_top_k() -> u32 {
    DEFAULT_RETRIEVAL_TOP_K
}
