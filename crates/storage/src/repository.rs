use async_trait::async_trait;
use rag_debugger_core::{
    ApiKey, ApiKeyId, ApiKeyRecord, AuthSessionRecord, AuthenticatedUser, Chunk, ChunkEmbedding,
    CiEvalRun, CiEvalRunId, Document, DocumentId, EmbeddingIndexCandidate, EmbeddingIndexRequest,
    EmbeddingModelInfo, EmbeddingStatus, IngestionRun, IngestionRunId, IngestionRunStatus,
    IngestionTotals, Organization, Project, ProjectId, RetrievalEvalCase, RetrievalEvalCaseId,
    RetrievalEvalDataset, RetrievalEvalDatasetId, RetrievalEvalDatasetSummary,
    RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalRun, RetrievalQueryRequest,
    RetrievalQueryResponse, RetrievalQueryRunId, SearchableChunk, Source, SourceId, SourceSummary,
    Trace, TraceId, TraceSummary, User, UserId, UserWithPassword, Workspace, WorkspaceId,
    WorkspaceRole,
};

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

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn bootstrap_identity(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError>;
    async fn create_user_workspace(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError>;
    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithPassword>, StorageError>;
    async fn get_authenticated_user(
        &self,
        user_id: UserId,
        workspace_id: WorkspaceId,
    ) -> Result<AuthenticatedUser, StorageError>;
    async fn create_auth_session(
        &self,
        session: AuthSessionRecord,
    ) -> Result<AuthSessionRecord, StorageError>;
    async fn find_auth_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<AuthSessionRecord>, StorageError>;
    async fn revoke_auth_session(&self, token_hash: &str) -> Result<(), StorageError>;
    async fn create_api_key(&self, record: ApiKeyRecord) -> Result<ApiKeyRecord, StorageError>;
    async fn list_api_keys(&self, workspace_id: WorkspaceId) -> Result<Vec<ApiKey>, StorageError>;
    async fn find_api_key(&self, secret_hash: &str) -> Result<Option<ApiKeyRecord>, StorageError>;
    async fn touch_api_key(&self, api_key_id: ApiKeyId) -> Result<(), StorageError>;
    async fn revoke_api_key(&self, api_key_id: ApiKeyId) -> Result<(), StorageError>;
}

#[async_trait]
pub trait CiEvalRepository: Send + Sync {
    async fn save_ci_eval_run(&self, run: CiEvalRun) -> Result<CiEvalRun, StorageError>;
    async fn list_ci_eval_runs(&self) -> Result<Vec<CiEvalRun>, StorageError>;
    async fn get_ci_eval_run(&self, id: CiEvalRunId) -> Result<CiEvalRun, StorageError>;
    async fn latest_ci_eval_run_for_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
        config_label: &str,
    ) -> Result<Option<CiEvalRun>, StorageError>;
}

pub trait AppRepository: IngestionRepository + AuthRepository + CiEvalRepository {}

impl<T> AppRepository for T where T: IngestionRepository + AuthRepository + CiEvalRepository {}

#[async_trait]
pub trait IngestionRepository: Send + Sync {
    async fn ping(&self) -> Result<(), StorageError>;
    async fn ensure_default_project(&self) -> Result<Project, StorageError>;
    async fn create_source(&self, source: Source) -> Result<Source, StorageError>;
    async fn create_ingestion_run(&self, run: IngestionRun) -> Result<IngestionRun, StorageError>;
    async fn complete_ingestion_run(
        &self,
        id: IngestionRunId,
        status: IngestionRunStatus,
        totals: IngestionTotals,
    ) -> Result<IngestionRun, StorageError>;
    async fn insert_document_with_chunks(
        &self,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<Document, StorageError>;
    async fn list_sources(&self) -> Result<Vec<SourceSummary>, StorageError>;
    async fn list_document_chunks(
        &self,
        document_id: DocumentId,
    ) -> Result<Vec<Chunk>, StorageError>;
    async fn list_searchable_chunks(
        &self,
        request: &RetrievalQueryRequest,
    ) -> Result<Vec<SearchableChunk>, StorageError>;
    async fn save_retrieval_query(
        &self,
        response: &RetrievalQueryResponse,
    ) -> Result<(), StorageError>;
    async fn get_retrieval_query(
        &self,
        id: RetrievalQueryRunId,
    ) -> Result<RetrievalQueryResponse, StorageError>;
    async fn latest_retrieval_query(&self) -> Result<RetrievalQueryResponse, StorageError>;
    async fn save_trace(&self, trace: Trace) -> Result<Trace, StorageError>;
    async fn list_traces(&self) -> Result<Vec<TraceSummary>, StorageError>;
    async fn get_trace_detail(&self, id: TraceId) -> Result<Trace, StorageError>;
    async fn embedding_status(
        &self,
        request: &EmbeddingIndexRequest,
        model: &EmbeddingModelInfo,
    ) -> Result<EmbeddingStatus, StorageError>;
    async fn list_embedding_candidates(
        &self,
        request: &EmbeddingIndexRequest,
    ) -> Result<Vec<EmbeddingIndexCandidate>, StorageError>;
    async fn upsert_chunk_embeddings(
        &self,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), StorageError>;
    async fn create_retrieval_eval_case(
        &self,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError>;
    async fn list_retrieval_eval_cases(&self) -> Result<Vec<RetrievalEvalCase>, StorageError>;
    async fn list_retrieval_eval_cases_by_id(
        &self,
        case_ids: &[RetrievalEvalCaseId],
    ) -> Result<Vec<RetrievalEvalCase>, StorageError>;
    async fn save_retrieval_eval_run(
        &self,
        eval_run: &RetrievalEvalRun,
    ) -> Result<(), StorageError>;
    async fn latest_retrieval_eval_run(&self) -> Result<Option<RetrievalEvalRun>, StorageError>;
    async fn create_retrieval_eval_dataset(
        &self,
        dataset: RetrievalEvalDataset,
    ) -> Result<RetrievalEvalDataset, StorageError>;
    async fn list_retrieval_eval_datasets(
        &self,
    ) -> Result<Vec<RetrievalEvalDatasetSummary>, StorageError>;
    async fn get_retrieval_eval_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<RetrievalEvalDataset, StorageError>;
    async fn create_retrieval_eval_case_in_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError>;
    async fn update_retrieval_eval_case(
        &self,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError>;
    async fn delete_retrieval_eval_case(
        &self,
        case_id: RetrievalEvalCaseId,
    ) -> Result<(), StorageError>;
    async fn save_retrieval_eval_experiment(
        &self,
        experiment: RetrievalEvalExperiment,
    ) -> Result<RetrievalEvalExperiment, StorageError>;
    async fn list_retrieval_eval_experiments(
        &self,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError>;
    async fn get_retrieval_eval_experiment(
        &self,
        experiment_id: RetrievalEvalExperimentId,
    ) -> Result<RetrievalEvalExperiment, StorageError>;
    async fn latest_retrieval_eval_experiment(
        &self,
    ) -> Result<Option<RetrievalEvalExperiment>, StorageError>;
}
