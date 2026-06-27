use std::path::Path;

use async_trait::async_trait;
use rag_debugger_core::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{
    repository::{AuthRepository, CiEvalRepository, IngestionRepository},
    StorageError,
};

mod auth;
mod ci_eval;
mod codec;
mod embeddings;
mod eval_lab;
mod ingestion;
mod projects;
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
impl IngestionRepository for PostgresStore {
    async fn ping(&self) -> Result<(), StorageError> {
        PostgresStore::ping(self).await
    }

    async fn ensure_default_project(&self) -> Result<Project, StorageError> {
        PostgresStore::ensure_default_project(self).await
    }

    async fn create_source(&self, source: Source) -> Result<Source, StorageError> {
        PostgresStore::create_source(self, source).await
    }

    async fn create_ingestion_run(&self, run: IngestionRun) -> Result<IngestionRun, StorageError> {
        PostgresStore::create_ingestion_run(self, run).await
    }

    async fn complete_ingestion_run(
        &self,
        id: IngestionRunId,
        status: IngestionRunStatus,
        totals: IngestionTotals,
    ) -> Result<IngestionRun, StorageError> {
        PostgresStore::complete_ingestion_run(self, id, status, totals).await
    }

    async fn insert_document_with_chunks(
        &self,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<Document, StorageError> {
        PostgresStore::insert_document_with_chunks(self, document, chunks).await
    }

    async fn list_sources(&self) -> Result<Vec<SourceSummary>, StorageError> {
        PostgresStore::list_sources(self).await
    }

    async fn list_document_chunks(
        &self,
        document_id: DocumentId,
    ) -> Result<Vec<Chunk>, StorageError> {
        PostgresStore::list_document_chunks(self, document_id).await
    }

    async fn list_searchable_chunks(
        &self,
        request: &RetrievalQueryRequest,
    ) -> Result<Vec<SearchableChunk>, StorageError> {
        PostgresStore::list_searchable_chunks(self, request).await
    }

    async fn save_retrieval_query(
        &self,
        response: &RetrievalQueryResponse,
    ) -> Result<(), StorageError> {
        PostgresStore::save_retrieval_query(self, response).await
    }

    async fn get_retrieval_query(
        &self,
        id: RetrievalQueryRunId,
    ) -> Result<RetrievalQueryResponse, StorageError> {
        PostgresStore::get_retrieval_query(self, id).await
    }

    async fn latest_retrieval_query(&self) -> Result<RetrievalQueryResponse, StorageError> {
        PostgresStore::latest_retrieval_query(self).await
    }

    async fn save_trace(&self, trace: Trace) -> Result<Trace, StorageError> {
        PostgresStore::save_trace(self, trace).await
    }

    async fn list_traces(&self) -> Result<Vec<TraceSummary>, StorageError> {
        PostgresStore::list_traces(self).await
    }

    async fn get_trace_detail(&self, id: TraceId) -> Result<Trace, StorageError> {
        PostgresStore::get_trace_detail(self, id).await
    }

    async fn embedding_status(
        &self,
        request: &EmbeddingIndexRequest,
        model: &EmbeddingModelInfo,
    ) -> Result<EmbeddingStatus, StorageError> {
        PostgresStore::embedding_status(self, request, model).await
    }

    async fn list_embedding_candidates(
        &self,
        request: &EmbeddingIndexRequest,
    ) -> Result<Vec<EmbeddingIndexCandidate>, StorageError> {
        PostgresStore::list_embedding_candidates(self, request).await
    }

    async fn upsert_chunk_embeddings(
        &self,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), StorageError> {
        PostgresStore::upsert_chunk_embeddings(self, embeddings).await
    }

    async fn create_retrieval_eval_case(
        &self,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        PostgresStore::create_retrieval_eval_case(self, eval_case).await
    }

    async fn list_retrieval_eval_cases(&self) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        PostgresStore::list_retrieval_eval_cases(self).await
    }

    async fn list_retrieval_eval_cases_by_id(
        &self,
        case_ids: &[RetrievalEvalCaseId],
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        PostgresStore::list_retrieval_eval_cases_by_id(self, case_ids).await
    }

    async fn save_retrieval_eval_run(
        &self,
        eval_run: &RetrievalEvalRun,
    ) -> Result<(), StorageError> {
        PostgresStore::save_retrieval_eval_run(self, eval_run).await
    }

    async fn latest_retrieval_eval_run(&self) -> Result<Option<RetrievalEvalRun>, StorageError> {
        PostgresStore::latest_retrieval_eval_run(self).await
    }

    async fn create_retrieval_eval_dataset(
        &self,
        dataset: RetrievalEvalDataset,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        PostgresStore::create_retrieval_eval_dataset(self, dataset).await
    }

    async fn list_retrieval_eval_datasets(
        &self,
    ) -> Result<Vec<RetrievalEvalDatasetSummary>, StorageError> {
        PostgresStore::list_retrieval_eval_datasets(self).await
    }

    async fn get_retrieval_eval_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        PostgresStore::get_retrieval_eval_dataset(self, dataset_id).await
    }

    async fn create_retrieval_eval_case_in_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        PostgresStore::create_retrieval_eval_case_in_dataset(self, dataset_id, eval_case).await
    }

    async fn update_retrieval_eval_case(
        &self,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        PostgresStore::update_retrieval_eval_case(self, eval_case).await
    }

    async fn delete_retrieval_eval_case(
        &self,
        case_id: RetrievalEvalCaseId,
    ) -> Result<(), StorageError> {
        PostgresStore::delete_retrieval_eval_case(self, case_id).await
    }

    async fn save_retrieval_eval_experiment(
        &self,
        experiment: RetrievalEvalExperiment,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        PostgresStore::save_retrieval_eval_experiment(self, experiment).await
    }

    async fn list_retrieval_eval_experiments(
        &self,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        PostgresStore::list_retrieval_eval_experiments(self).await
    }

    async fn get_retrieval_eval_experiment(
        &self,
        experiment_id: RetrievalEvalExperimentId,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        PostgresStore::get_retrieval_eval_experiment(self, experiment_id).await
    }

    async fn latest_retrieval_eval_experiment(
        &self,
    ) -> Result<Option<RetrievalEvalExperiment>, StorageError> {
        PostgresStore::latest_retrieval_eval_experiment(self).await
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

    async fn revoke_api_key(&self, api_key_id: ApiKeyId) -> Result<(), StorageError> {
        PostgresStore::revoke_api_key(self, api_key_id).await
    }
}

#[async_trait]
impl CiEvalRepository for PostgresStore {
    async fn save_ci_eval_run(&self, run: CiEvalRun) -> Result<CiEvalRun, StorageError> {
        PostgresStore::save_ci_eval_run(self, run).await
    }

    async fn list_ci_eval_runs(&self) -> Result<Vec<CiEvalRun>, StorageError> {
        PostgresStore::list_ci_eval_runs(self).await
    }

    async fn get_ci_eval_run(&self, id: CiEvalRunId) -> Result<CiEvalRun, StorageError> {
        PostgresStore::get_ci_eval_run(self, id).await
    }

    async fn latest_ci_eval_run_for_dataset(
        &self,
        dataset_id: RetrievalEvalDatasetId,
        config_label: &str,
    ) -> Result<Option<CiEvalRun>, StorageError> {
        PostgresStore::latest_ci_eval_run_for_dataset(self, dataset_id, config_label).await
    }
}
