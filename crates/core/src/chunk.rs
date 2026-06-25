use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::source::{DocumentId, SourceId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chunk {
    pub id: ChunkId,
    pub source_id: SourceId,
    pub document_id: DocumentId,
    pub ordinal: u32,
    pub text: String,
    pub token_count: u32,
    pub byte_range: ByteRange,
    pub checksum: String,
    pub strategy: ChunkingStrategy,
    pub section_title: Option<String>,
    pub split_reason: ChunkSplitReason,
    #[serde(default)]
    pub quality_flags: Vec<ChunkQualityFlag>,
    #[serde(default)]
    pub is_duplicate: bool,
    #[serde(default)]
    pub text_density: f32,
    #[serde(default)]
    pub evidence_score_hint: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ChunkId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChunkingStrategy {
    #[default]
    Structured,
    SmartSections,
    Whitespace,
}

impl ChunkingStrategy {
    pub fn normalized(self) -> Self {
        match self {
            Self::SmartSections => Self::Structured,
            strategy => strategy,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChunkSplitReason {
    SectionBoundary,
    TokenLimit,
    DocumentEnd,
    FallbackWhitespace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub struct ChunkingConfig {
    pub target_tokens: u32,
    pub overlap_tokens: u32,
    pub strategy: ChunkingStrategy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ChunkQualityFlag {
    HeadingOnly,
    TooShort,
    TooLong,
    Duplicate,
    LowTextDensity,
    ExtractionWarning,
    GoodEvidenceCandidate,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            target_tokens: 512,
            overlap_tokens: 64,
            strategy: ChunkingStrategy::Structured,
        }
    }
}
