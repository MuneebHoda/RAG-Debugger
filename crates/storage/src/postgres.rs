use std::path::Path;

use async_trait::async_trait;
use rag_debugger_core::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{repository::IngestionRepository, StorageError};

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
