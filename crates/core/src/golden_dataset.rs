use serde::{Deserialize, Serialize};

use crate::{
    ChunkId, DocumentId, RetrievalEvalDatasetId, TraceIngestionPrivacyMode, TraceIngestionSource,
};

pub const GOLDEN_DATASET_SCHEMA_VERSION: u32 = 1;
pub const GOLDEN_DATASET_CASE_KEY_MAX_CHARS: usize = 128;

pub fn is_valid_golden_dataset_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= GOLDEN_DATASET_CASE_KEY_MAX_CHARS
        && key.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (index > 0 && matches!(byte, b'-' | b'_' | b'.'))
        })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GoldenDatasetEvidenceIdentity {
    pub document_id: DocumentId,
    pub document_checksum: String,
    pub chunk_id: Option<ChunkId>,
    pub chunk_checksum: Option<String>,
    pub chunk_ordinal: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GoldenDatasetContentMode {
    Full,
    MetadataOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GoldenDataset {
    pub schema_version: u32,
    pub content_mode: GoldenDatasetContentMode,
    pub dataset: GoldenDatasetIdentity,
    pub cases: Vec<GoldenDatasetCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GoldenDatasetIdentity {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GoldenDatasetCase {
    pub case_key: String,
    pub name: String,
    pub query: Option<String>,
    pub top_k: u32,
    #[serde(default)]
    pub expected_documents: Vec<GoldenDatasetDocumentReference>,
    #[serde(default)]
    pub expected_chunks: Vec<GoldenDatasetChunkReference>,
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<GoldenDatasetCaseProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct GoldenDatasetDocumentReference {
    pub document_checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct GoldenDatasetChunkReference {
    pub document_checksum: String,
    pub chunk_checksum: String,
    pub ordinal: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GoldenDatasetCaseProvenance {
    pub source_trace_id: String,
    pub source: TraceIngestionSource,
    pub privacy_mode: TraceIngestionPrivacyMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GoldenDatasetImportMode {
    CreateNew,
    MergeByCaseKey,
    ReplaceDataset,
    ValidateOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GoldenDatasetImportAction {
    CreateDataset,
    UpdateDataset,
    ValidateOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum GoldenDatasetPrivacyField {
    Queries,
    Notes,
    EvidenceReferences,
    Provenance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum GoldenDatasetValidationCode {
    InvalidCaseKey,
    DuplicateCaseKey,
    MissingName,
    MissingQuery,
    MissingExpectedEvidence,
    InvalidTopK,
    TooManyEvidenceReferences,
    UnresolvedDocument,
    UnresolvedChunk,
    MalformedId,
    UnavailableProvenance,
    ImmutableProvenanceConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct GoldenDatasetInvalidCase {
    pub case_key: String,
    pub code: GoldenDatasetValidationCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GoldenDatasetEvidenceKind {
    Document,
    Chunk,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct GoldenDatasetUnresolvedEvidence {
    pub case_key: String,
    pub kind: GoldenDatasetEvidenceKind,
    pub document_checksum: String,
    pub chunk_checksum: Option<String>,
    pub ordinal: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct GoldenDatasetImportSummary {
    pub schema_version: u32,
    pub mode: GoldenDatasetImportMode,
    pub dry_run: bool,
    pub valid: bool,
    pub applied: bool,
    pub action: GoldenDatasetImportAction,
    pub dataset_id: Option<RetrievalEvalDatasetId>,
    pub cases_total: u32,
    pub cases_added: u32,
    pub cases_changed: u32,
    pub cases_skipped: u32,
    pub cases_removed: u32,
    pub invalid_cases: Vec<GoldenDatasetInvalidCase>,
    pub unresolved_evidence: Vec<GoldenDatasetUnresolvedEvidence>,
    pub privacy_sensitive_fields: Vec<GoldenDatasetPrivacyField>,
    pub validation_token: Option<String>,
}
