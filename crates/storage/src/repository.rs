use async_trait::async_trait;
use rag_debugger_core::{Project, ProjectId, Source, SourceId, Trace, TraceId};

use crate::StorageError;

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn get_project(&self, id: ProjectId) -> Result<Project, StorageError>;
    async fn upsert_project(&self, project: Project) -> Result<(), StorageError>;
}

#[async_trait]
pub trait SourceRepository: Send + Sync {
    async fn get_source(&self, id: SourceId) -> Result<Source, StorageError>;
    async fn list_sources_for_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<Source>, StorageError>;
}

#[async_trait]
pub trait TraceRepository: Send + Sync {
    async fn get_trace(&self, id: TraceId) -> Result<Trace, StorageError>;
    async fn append_trace(&self, trace: Trace) -> Result<(), StorageError>;
}
