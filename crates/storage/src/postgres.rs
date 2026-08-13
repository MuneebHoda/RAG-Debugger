use std::path::Path;

use async_trait::async_trait;
use rag_debugger_core::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{
    repository::{
        AuthRepository, CiEvalRepository, DemoRepository, DocumentRepository, EmbeddingRepository,
        EvalRepository, EvidenceRepository, HealthRepository, ProjectRepository, ReportRepository,
        RetrievalRepository, SourceRepository, SubmittedExpectedEvidence, TraceRepository,
    },
    StorageError,
};

mod auth;
mod ci_eval;
mod codec;
mod demo;
mod embeddings;
mod eval_lab;
mod evidence;
mod ingestion;
mod projects;
mod reports;
mod retrieval;
mod traces;

#[derive(Debug, Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn run_migrations(&self, migrations_path: &Path) -> Result<(), StorageError> {
        let migrator = sqlx::migrate::Migrator::new(migrations_path).await?;
        migrator.run(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl HealthRepository for PostgresStore {
    async fn ping(&self) -> Result<(), StorageError> {
        PostgresStore::ping(self).await
    }
}

#[async_trait]
impl ProjectRepository for PostgresStore {
    async fn ensure_default_project(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Project, StorageError> {
        PostgresStore::ensure_default_project(self, workspace_id).await
    }

    async fn get_project(
        &self,
        workspace_id: WorkspaceId,
        project_id: ProjectId,
    ) -> Result<Project, StorageError> {
        PostgresStore::get_project(self, workspace_id, project_id).await
    }
}

#[async_trait]
impl SourceRepository for PostgresStore {
    async fn create_source(
        &self,
        workspace_id: WorkspaceId,
        source: Source,
    ) -> Result<Source, StorageError> {
        PostgresStore::create_source(self, workspace_id, source).await
    }

    async fn create_ingestion_run(
        &self,
        workspace_id: WorkspaceId,
        run: IngestionRun,
    ) -> Result<IngestionRun, StorageError> {
        PostgresStore::create_ingestion_run(self, workspace_id, run).await
    }

    async fn complete_ingestion_run(
        &self,
        workspace_id: WorkspaceId,
        id: IngestionRunId,
        status: IngestionRunStatus,
        totals: IngestionTotals,
    ) -> Result<IngestionRun, StorageError> {
        PostgresStore::complete_ingestion_run(self, workspace_id, id, status, totals).await
    }

    async fn list_sources(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<SourceSummary>, StorageError> {
        PostgresStore::list_sources(self, workspace_id).await
    }
}

#[async_trait]
impl DocumentRepository for PostgresStore {
    async fn insert_document_with_chunks(
        &self,
        workspace_id: WorkspaceId,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<Document, StorageError> {
        PostgresStore::insert_document_with_chunks(self, workspace_id, document, chunks).await
    }

    async fn list_document_chunks(
        &self,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
    ) -> Result<Vec<Chunk>, StorageError> {
        PostgresStore::list_document_chunks(self, workspace_id, document_id).await
    }
}

#[async_trait]
impl EvidenceRepository for PostgresStore {
    async fn resolve_evidence_documents(
        &self,
        workspace_id: WorkspaceId,
        document_ids: &[DocumentId],
    ) -> Result<Vec<EvalLabEvidenceDocument>, StorageError> {
        PostgresStore::resolve_evidence_documents(self, workspace_id, document_ids).await
    }

    async fn resolve_evidence_chunks(
        &self,
        workspace_id: WorkspaceId,
        chunk_ids: &[ChunkId],
    ) -> Result<Vec<EvalLabEvidenceChunk>, StorageError> {
        PostgresStore::resolve_evidence_chunks(self, workspace_id, chunk_ids).await
    }

    async fn search_evidence(
        &self,
        workspace_id: WorkspaceId,
        request: &EvalLabEvidenceSearchRequest,
    ) -> Result<EvalLabEvidenceSearchResult, StorageError> {
        PostgresStore::search_evidence(self, workspace_id, request).await
    }
}

#[async_trait]
impl DemoRepository for PostgresStore {
    async fn ensure_demo_project(
        &self,
        workspace_id: WorkspaceId,
        project: Project,
    ) -> Result<Project, StorageError> {
        PostgresStore::ensure_demo_project(self, workspace_id, project).await
    }

    async fn ensure_demo_source(&self, source: Source) -> Result<Source, StorageError> {
        PostgresStore::ensure_demo_source(self, source).await
    }

    async fn upsert_demo_document_with_chunks(
        &self,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<bool, StorageError> {
        PostgresStore::upsert_demo_document_with_chunks(self, document, chunks).await
    }

    async fn get_demo_source(
        &self,
        workspace_id: WorkspaceId,
        version_marker: &str,
    ) -> Result<Option<SourceSummary>, StorageError> {
        PostgresStore::get_demo_source(self, workspace_id, version_marker).await
    }

    async fn latest_retrieval_query_for_source(
        &self,
        workspace_id: WorkspaceId,
        source_id: SourceId,
    ) -> Result<Option<RetrievalQueryResponse>, StorageError> {
        PostgresStore::latest_retrieval_query_for_source(self, workspace_id, source_id).await
    }

    async fn latest_trace_for_source(
        &self,
        workspace_id: WorkspaceId,
        source_id: SourceId,
    ) -> Result<Option<Trace>, StorageError> {
        PostgresStore::latest_trace_for_source(self, workspace_id, source_id).await
    }
}

#[async_trait]
impl RetrievalRepository for PostgresStore {
    async fn list_searchable_chunks(
        &self,
        workspace_id: WorkspaceId,
        request: &RetrievalQueryRequest,
    ) -> Result<Vec<SearchableChunk>, StorageError> {
        PostgresStore::list_searchable_chunks(self, workspace_id, request).await
    }

    async fn save_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
        response: &RetrievalQueryResponse,
    ) -> Result<(), StorageError> {
        PostgresStore::save_retrieval_query(self, workspace_id, response).await
    }

    async fn get_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
        id: RetrievalQueryRunId,
    ) -> Result<RetrievalQueryResponse, StorageError> {
        PostgresStore::get_retrieval_query(self, workspace_id, id).await
    }

    async fn latest_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<RetrievalQueryResponse, StorageError> {
        PostgresStore::latest_retrieval_query(self, workspace_id).await
    }
}

#[async_trait]
impl TraceRepository for PostgresStore {
    async fn save_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<Trace, StorageError> {
        PostgresStore::save_trace(self, workspace_id, trace).await
    }

    async fn upsert_imported_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<ImportedTraceUpsertResult, StorageError> {
        PostgresStore::upsert_imported_trace(self, workspace_id, trace).await
    }

    async fn list_traces(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<TraceSummary>, StorageError> {
        PostgresStore::list_traces(self, workspace_id).await
    }

    async fn get_trace_detail(
        &self,
        workspace_id: WorkspaceId,
        id: TraceId,
    ) -> Result<Trace, StorageError> {
        PostgresStore::get_trace_detail(self, workspace_id, id).await
    }
}

#[async_trait]
impl EmbeddingRepository for PostgresStore {
    async fn embedding_status(
        &self,
        workspace_id: WorkspaceId,
        request: &EmbeddingIndexRequest,
        model: &EmbeddingModelInfo,
    ) -> Result<EmbeddingStatus, StorageError> {
        PostgresStore::embedding_status(self, workspace_id, request, model).await
    }

    async fn list_embedding_candidates(
        &self,
        workspace_id: WorkspaceId,
        request: &EmbeddingIndexRequest,
    ) -> Result<Vec<EmbeddingIndexCandidate>, StorageError> {
        PostgresStore::list_embedding_candidates(self, workspace_id, request).await
    }

    async fn upsert_chunk_embeddings(
        &self,
        workspace_id: WorkspaceId,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), StorageError> {
        PostgresStore::upsert_chunk_embeddings(self, workspace_id, embeddings).await
    }
}

#[async_trait]
impl EvalRepository for PostgresStore {
    async fn create_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        PostgresStore::create_retrieval_eval_case(self, workspace_id, eval_case).await
    }

    async fn list_retrieval_eval_cases(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        PostgresStore::list_retrieval_eval_cases(self, workspace_id).await
    }

    async fn list_retrieval_eval_cases_by_id(
        &self,
        workspace_id: WorkspaceId,
        case_ids: &[RetrievalEvalCaseId],
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        PostgresStore::list_retrieval_eval_cases_by_id(self, workspace_id, case_ids).await
    }

    async fn get_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<RetrievalEvalCase, StorageError> {
        PostgresStore::get_retrieval_eval_case(self, workspace_id, case_id).await
    }

    async fn save_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
        eval_run: &RetrievalEvalRun,
    ) -> Result<(), StorageError> {
        PostgresStore::save_retrieval_eval_run(self, workspace_id, eval_run).await
    }

    async fn latest_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalRun>, StorageError> {
        PostgresStore::latest_retrieval_eval_run(self, workspace_id).await
    }

    async fn create_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset: RetrievalEvalDataset,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        PostgresStore::create_retrieval_eval_dataset(self, workspace_id, dataset).await
    }

    async fn list_retrieval_eval_datasets(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalDatasetSummary>, StorageError> {
        PostgresStore::list_retrieval_eval_datasets(self, workspace_id).await
    }

    async fn get_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        PostgresStore::get_retrieval_eval_dataset(self, workspace_id, dataset_id).await
    }

    async fn create_retrieval_eval_case_in_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        PostgresStore::create_retrieval_eval_case_in_dataset(
            self,
            workspace_id,
            dataset_id,
            eval_case,
        )
        .await
    }

    async fn update_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
        submitted_evidence: SubmittedExpectedEvidence,
    ) -> Result<RetrievalEvalCase, StorageError> {
        PostgresStore::update_retrieval_eval_case(self, workspace_id, eval_case, submitted_evidence)
            .await
    }

    async fn delete_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<(), StorageError> {
        PostgresStore::delete_retrieval_eval_case(self, workspace_id, case_id).await
    }

    async fn save_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
        experiment: RetrievalEvalExperiment,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        PostgresStore::save_retrieval_eval_experiment(self, workspace_id, experiment).await
    }

    async fn list_retrieval_eval_experiments(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        PostgresStore::list_retrieval_eval_experiments(self, workspace_id).await
    }

    async fn list_retrieval_eval_experiments_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        PostgresStore::list_retrieval_eval_experiments_for_dataset(self, workspace_id, dataset_id)
            .await
    }

    async fn get_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
        experiment_id: RetrievalEvalExperimentId,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        PostgresStore::get_retrieval_eval_experiment(self, workspace_id, experiment_id).await
    }

    async fn latest_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalExperiment>, StorageError> {
        PostgresStore::latest_retrieval_eval_experiment(self, workspace_id).await
    }
}

#[async_trait]
impl AuthRepository for PostgresStore {
    async fn bootstrap_identity(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError> {
        PostgresStore::bootstrap_identity(self, organization, workspace, user, role, password_hash)
            .await
    }

    async fn create_user_workspace(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError> {
        PostgresStore::create_user_workspace(
            self,
            organization,
            workspace,
            user,
            role,
            password_hash,
        )
        .await
    }

    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithPassword>, StorageError> {
        PostgresStore::find_user_by_email(self, email).await
    }

    async fn get_authenticated_user(
        &self,
        user_id: UserId,
        workspace_id: WorkspaceId,
    ) -> Result<AuthenticatedUser, StorageError> {
        PostgresStore::get_authenticated_user(self, user_id, workspace_id).await
    }

    async fn create_auth_session(
        &self,
        session: AuthSessionRecord,
    ) -> Result<AuthSessionRecord, StorageError> {
        PostgresStore::create_auth_session(self, session).await
    }

    async fn find_auth_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<AuthSessionRecord>, StorageError> {
        PostgresStore::find_auth_session(self, token_hash).await
    }

    async fn revoke_auth_session(&self, token_hash: &str) -> Result<(), StorageError> {
        PostgresStore::revoke_auth_session(self, token_hash).await
    }

    async fn create_api_key(&self, record: ApiKeyRecord) -> Result<ApiKeyRecord, StorageError> {
        PostgresStore::create_api_key(self, record).await
    }

    async fn list_api_keys(&self, workspace_id: WorkspaceId) -> Result<Vec<ApiKey>, StorageError> {
        PostgresStore::list_api_keys(self, workspace_id).await
    }

    async fn find_api_key(&self, secret_hash: &str) -> Result<Option<ApiKeyRecord>, StorageError> {
        PostgresStore::find_api_key(self, secret_hash).await
    }

    async fn touch_api_key(&self, api_key_id: ApiKeyId) -> Result<(), StorageError> {
        PostgresStore::touch_api_key(self, api_key_id).await
    }

    async fn revoke_api_key(
        &self,
        workspace_id: WorkspaceId,
        api_key_id: ApiKeyId,
    ) -> Result<(), StorageError> {
        PostgresStore::revoke_api_key(self, workspace_id, api_key_id).await
    }
}

#[async_trait]
impl CiEvalRepository for PostgresStore {
    async fn save_ci_eval_run(&self, run: CiEvalRun) -> Result<CiEvalRun, StorageError> {
        PostgresStore::save_ci_eval_run(self, run).await
    }

    async fn list_ci_eval_runs(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<CiEvalRun>, StorageError> {
        PostgresStore::list_ci_eval_runs(self, workspace_id).await
    }

    async fn get_ci_eval_run(
        &self,
        workspace_id: WorkspaceId,
        id: CiEvalRunId,
    ) -> Result<CiEvalRun, StorageError> {
        PostgresStore::get_ci_eval_run(self, workspace_id, id).await
    }

    async fn latest_ci_eval_run_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
        config_label: &str,
    ) -> Result<Option<CiEvalRun>, StorageError> {
        PostgresStore::latest_ci_eval_run_for_dataset(self, workspace_id, dataset_id, config_label)
            .await
    }
}

#[async_trait]
impl ReportRepository for PostgresStore {
    async fn save_debug_report(&self, report: DebugReport) -> Result<DebugReport, StorageError> {
        PostgresStore::save_debug_report(self, report).await
    }

    async fn list_debug_reports(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<DebugReport>, StorageError> {
        PostgresStore::list_debug_reports(self, workspace_id).await
    }

    async fn get_debug_report(
        &self,
        workspace_id: WorkspaceId,
        report_id: DebugReportId,
    ) -> Result<DebugReport, StorageError> {
        PostgresStore::get_debug_report(self, workspace_id, report_id).await
    }
}
