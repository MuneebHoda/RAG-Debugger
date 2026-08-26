use async_trait::async_trait;
use rag_debugger_core::{
    ApiKey, ApiKeyId, ApiKeyRecord, AuthSessionRecord, AuthenticatedUser, Chunk, ChunkEmbedding,
    ChunkId, CiEvalRun, CiEvalRunId, DebugReport, DebugReportId, Document, DocumentId,
    EmbeddingIndexCandidate, EmbeddingIndexRequest, EmbeddingModelInfo, EmbeddingStatus,
    EvalLabEvidenceChunk, EvalLabEvidenceDocument, EvalLabEvidenceSearchRequest,
    EvalLabEvidenceSearchResult, ImportedTraceUpsertResult, IngestionRun, IngestionRunId,
    IngestionRunStatus, IngestionTotals, Organization, Project, ProjectId, RetrievalEvalCase,
    RetrievalEvalCaseId, RetrievalEvalDataset, RetrievalEvalDatasetId, RetrievalEvalDatasetSummary,
    RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalRun, RetrievalQueryRequest,
    RetrievalQueryResponse, RetrievalQueryRunId, SearchableChunk, Source, SourceSummary, Trace,
    TraceId, TraceSummary, User, UserId, UserWithPassword, Workspace, WorkspaceId, WorkspaceRole,
};

use crate::StorageError;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SubmittedExpectedEvidence {
    pub document_ids: Option<Vec<DocumentId>>,
    pub chunk_ids: Option<Vec<ChunkId>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RetrievalEvalCorpusSnapshot {
    pub sources: Vec<SourceSummary>,
    pub candidates: Vec<SearchableChunk>,
}

#[async_trait]
pub trait HealthRepository: Send + Sync {
    async fn ping(&self) -> Result<(), StorageError>;
}

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn ensure_default_project(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Project, StorageError>;
    async fn get_project(
        &self,
        workspace_id: WorkspaceId,
        project_id: ProjectId,
    ) -> Result<Project, StorageError>;
}

#[async_trait]
pub trait SourceRepository: Send + Sync {
    async fn create_source(
        &self,
        workspace_id: WorkspaceId,
        source: Source,
    ) -> Result<Source, StorageError>;
    async fn create_ingestion_run(
        &self,
        workspace_id: WorkspaceId,
        run: IngestionRun,
    ) -> Result<IngestionRun, StorageError>;
    async fn complete_ingestion_run(
        &self,
        workspace_id: WorkspaceId,
        id: IngestionRunId,
        status: IngestionRunStatus,
        totals: IngestionTotals,
    ) -> Result<IngestionRun, StorageError>;
    async fn list_sources(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<SourceSummary>, StorageError>;
}

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn insert_document_with_chunks(
        &self,
        workspace_id: WorkspaceId,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<Document, StorageError>;
    async fn list_document_chunks(
        &self,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
    ) -> Result<Vec<Chunk>, StorageError>;
}

#[async_trait]
pub trait EvidenceRepository: Send + Sync {
    async fn resolve_evidence_documents(
        &self,
        workspace_id: WorkspaceId,
        document_ids: &[DocumentId],
    ) -> Result<Vec<EvalLabEvidenceDocument>, StorageError>;
    async fn resolve_evidence_chunks(
        &self,
        workspace_id: WorkspaceId,
        chunk_ids: &[ChunkId],
    ) -> Result<Vec<EvalLabEvidenceChunk>, StorageError>;
    async fn search_evidence(
        &self,
        workspace_id: WorkspaceId,
        request: &EvalLabEvidenceSearchRequest,
    ) -> Result<EvalLabEvidenceSearchResult, StorageError>;
}

#[async_trait]
pub trait EmbeddingRepository: Send + Sync {
    async fn embedding_status(
        &self,
        workspace_id: WorkspaceId,
        request: &EmbeddingIndexRequest,
        model: &EmbeddingModelInfo,
    ) -> Result<EmbeddingStatus, StorageError>;
    async fn list_embedding_candidates(
        &self,
        workspace_id: WorkspaceId,
        request: &EmbeddingIndexRequest,
    ) -> Result<Vec<EmbeddingIndexCandidate>, StorageError>;
    async fn upsert_chunk_embeddings(
        &self,
        workspace_id: WorkspaceId,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), StorageError>;
}

#[async_trait]
pub trait RetrievalRepository: Send + Sync {
    async fn list_searchable_chunks(
        &self,
        workspace_id: WorkspaceId,
        request: &RetrievalQueryRequest,
    ) -> Result<Vec<SearchableChunk>, StorageError>;
    async fn save_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
        response: &RetrievalQueryResponse,
    ) -> Result<(), StorageError>;
    async fn get_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
        id: RetrievalQueryRunId,
    ) -> Result<RetrievalQueryResponse, StorageError>;
    async fn latest_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<RetrievalQueryResponse, StorageError>;
}

#[async_trait]
pub trait TraceRepository: Send + Sync {
    async fn save_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<Trace, StorageError>;
    async fn upsert_imported_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<ImportedTraceUpsertResult, StorageError>;
    async fn list_traces(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<TraceSummary>, StorageError>;
    async fn get_trace_detail(
        &self,
        workspace_id: WorkspaceId,
        id: TraceId,
    ) -> Result<Trace, StorageError>;
}

#[async_trait]
pub trait EvalRepository: Send + Sync {
    async fn retrieval_eval_corpus_snapshot(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<RetrievalEvalCorpusSnapshot, StorageError>;
    async fn create_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError>;
    async fn list_retrieval_eval_cases(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalCase>, StorageError>;
    async fn list_retrieval_eval_cases_by_id(
        &self,
        workspace_id: WorkspaceId,
        case_ids: &[RetrievalEvalCaseId],
    ) -> Result<Vec<RetrievalEvalCase>, StorageError>;
    async fn get_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<RetrievalEvalCase, StorageError>;
    async fn save_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
        eval_run: &RetrievalEvalRun,
    ) -> Result<(), StorageError>;
    async fn latest_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalRun>, StorageError>;
    async fn create_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset: RetrievalEvalDataset,
    ) -> Result<RetrievalEvalDataset, StorageError>;
    async fn list_retrieval_eval_datasets(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalDatasetSummary>, StorageError>;
    async fn get_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<RetrievalEvalDataset, StorageError>;
    async fn create_retrieval_eval_case_in_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError>;
    async fn update_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
        submitted_evidence: SubmittedExpectedEvidence,
    ) -> Result<RetrievalEvalCase, StorageError>;
    async fn delete_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<(), StorageError>;
    async fn save_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
        experiment: RetrievalEvalExperiment,
    ) -> Result<RetrievalEvalExperiment, StorageError>;
    async fn list_retrieval_eval_experiments(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError>;
    async fn list_retrieval_eval_experiments_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError>;
    async fn get_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
        experiment_id: RetrievalEvalExperimentId,
    ) -> Result<RetrievalEvalExperiment, StorageError>;
    async fn latest_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalExperiment>, StorageError>;
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
    async fn revoke_api_key(
        &self,
        workspace_id: WorkspaceId,
        api_key_id: ApiKeyId,
    ) -> Result<(), StorageError>;
}

#[async_trait]
pub trait CiEvalRepository: Send + Sync {
    async fn save_ci_eval_run(&self, run: CiEvalRun) -> Result<CiEvalRun, StorageError>;
    async fn list_ci_eval_runs(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<CiEvalRun>, StorageError>;
    async fn get_ci_eval_run(
        &self,
        workspace_id: WorkspaceId,
        id: CiEvalRunId,
    ) -> Result<CiEvalRun, StorageError>;
    async fn latest_ci_eval_run_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
        config_label: &str,
    ) -> Result<Option<CiEvalRun>, StorageError>;
}

#[async_trait]
pub trait ReportRepository: Send + Sync {
    async fn save_debug_report(&self, report: DebugReport) -> Result<DebugReport, StorageError>;
    async fn list_debug_reports(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<DebugReport>, StorageError>;
    async fn get_debug_report(
        &self,
        workspace_id: WorkspaceId,
        report_id: DebugReportId,
    ) -> Result<DebugReport, StorageError>;
}

#[async_trait]
pub trait DemoRepository: Send + Sync {
    async fn ensure_demo_project(
        &self,
        workspace_id: WorkspaceId,
        project: Project,
    ) -> Result<Project, StorageError>;
    async fn ensure_demo_source(&self, source: Source) -> Result<Source, StorageError>;
    async fn upsert_demo_document_with_chunks(
        &self,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<bool, StorageError>;
    async fn get_demo_source(
        &self,
        workspace_id: WorkspaceId,
        version_marker: &str,
    ) -> Result<Option<SourceSummary>, StorageError>;
    async fn latest_retrieval_query_for_source(
        &self,
        workspace_id: WorkspaceId,
        source_id: rag_debugger_core::SourceId,
    ) -> Result<Option<RetrievalQueryResponse>, StorageError>;
    async fn latest_trace_for_source(
        &self,
        workspace_id: WorkspaceId,
        source_id: rag_debugger_core::SourceId,
    ) -> Result<Option<Trace>, StorageError>;
}

/// Compatibility boundary for the synchronous upload workflow.
pub trait IngestionRepository:
    ProjectRepository + SourceRepository + DocumentRepository + Send + Sync
{
}

impl<T> IngestionRepository for T where
    T: ProjectRepository + SourceRepository + DocumentRepository + Send + Sync
{
}

pub trait AppRepository:
    HealthRepository
    + IngestionRepository
    + EvidenceRepository
    + EmbeddingRepository
    + RetrievalRepository
    + TraceRepository
    + EvalRepository
    + AuthRepository
    + CiEvalRepository
    + ReportRepository
    + DemoRepository
{
}

impl<T> AppRepository for T where
    T: HealthRepository
        + IngestionRepository
        + EvidenceRepository
        + EmbeddingRepository
        + RetrievalRepository
        + TraceRepository
        + EvalRepository
        + AuthRepository
        + CiEvalRepository
        + ReportRepository
        + DemoRepository
{
}
