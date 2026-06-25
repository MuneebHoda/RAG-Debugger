use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{chunk::ChunkingConfig, project::ProjectId};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Source {
    pub id: SourceId,
    pub project_id: ProjectId,
    pub name: String,
    pub kind: SourceKind,
    pub sync_policy: SourceSyncPolicy,
    pub chunking: ChunkingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum SourceKind {
    FileSet { root_hint: String },
    GitHubRepository { owner: String, repo: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum SourceSyncPolicy {
    Manual,
    OnDemand,
    Scheduled { cron: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Document {
    pub id: DocumentId,
    pub source_id: SourceId,
    pub path: String,
    pub mime_type: Option<String>,
    pub checksum: String,
    pub byte_size: u64,
    #[serde(default)]
    pub profile: DocumentProfile,
    #[serde(default)]
    pub extraction_quality: ExtractionQuality,
    #[serde(default)]
    pub warnings: Vec<DocumentWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct SourceSummary {
    pub source: Source,
    pub document_count: u32,
    pub chunk_count: u32,
    pub documents: Vec<DocumentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DocumentSummary {
    pub document: Document,
    pub chunk_count: u32,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentProfile {
    #[default]
    General,
    TechnicalDocs,
    PolicyOrLegal,
    SupportKb,
    ResearchPaper,
    CodeDocs,
    Resume,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionQuality {
    High,
    Medium,
    Low,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DocumentWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct SourceId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct DocumentId(pub Uuid);
