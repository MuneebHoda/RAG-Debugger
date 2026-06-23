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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct SourceId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct DocumentId(pub Uuid);
