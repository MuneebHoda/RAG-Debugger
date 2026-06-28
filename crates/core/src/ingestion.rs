use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{chunk::Chunk, source::SourceId, ChunkQualityFlag, ChunkSplitReason, ChunkingStrategy};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct IngestionRun {
    pub id: IngestionRunId,
    pub source_id: SourceId,
    pub status: IngestionRunStatus,
    pub totals: IngestionTotals,
    #[serde(with = "crate::wire_time")]
    pub started_at: OffsetDateTime,
    #[serde(with = "crate::wire_time::option")]
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum IngestionRunStatus {
    Running,
    Completed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct IngestionTotals {
    pub files_received: u32,
    pub documents_created: u32,
    pub chunks_created: u32,
    pub failed_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkPreview {
    pub id: crate::chunk::ChunkId,
    pub document_id: crate::source::DocumentId,
    pub ordinal: u32,
    pub text: String,
    pub token_count: u32,
    pub byte_range: crate::chunk::ByteRange,
    pub checksum: String,
    pub strategy: ChunkingStrategy,
    pub section_title: Option<String>,
    pub split_reason: ChunkSplitReason,
    pub quality_flags: Vec<ChunkQualityFlag>,
    pub is_duplicate: bool,
    pub text_density: f32,
    pub evidence_score_hint: f32,
}

impl From<Chunk> for ChunkPreview {
    fn from(chunk: Chunk) -> Self {
        Self {
            id: chunk.id,
            document_id: chunk.document_id,
            ordinal: chunk.ordinal,
            text: chunk.text,
            token_count: chunk.token_count,
            byte_range: chunk.byte_range,
            checksum: chunk.checksum,
            strategy: chunk.strategy,
            section_title: chunk.section_title,
            split_reason: chunk.split_reason,
            quality_flags: chunk.quality_flags,
            is_duplicate: chunk.is_duplicate,
            text_density: chunk.text_density,
            evidence_score_hint: chunk.evidence_score_hint,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct IngestionRunId(pub Uuid);
