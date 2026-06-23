use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::source::{DocumentId, SourceId};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Chunk {
    pub id: ChunkId,
    pub source_id: SourceId,
    pub document_id: DocumentId,
    pub ordinal: u32,
    pub text: String,
    pub token_count: u32,
    pub byte_range: ByteRange,
    pub checksum: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ChunkId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub struct ChunkingConfig {
    pub target_tokens: u32,
    pub overlap_tokens: u32,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            target_tokens: 512,
            overlap_tokens: 64,
        }
    }
}
