use std::{
    cmp::Reverse,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use rag_debugger_core::{
    ApiKey, ApiKeyId, ApiKeyRecord, AuthSessionRecord, AuthenticatedUser, Chunk, ChunkEmbedding,
    ChunkId, CiEvalRun, CiEvalRunId, DebugReport, DebugReportId, Document, DocumentId,
    DocumentSummary, EmbeddingIndexCandidate, EmbeddingIndexRequest, EmbeddingModelInfo,
    EmbeddingStatus, ImportedTraceUpsertResult, IngestionRun, IngestionRunId, IngestionRunStatus,
    IngestionTotals, Organization, PrivacyMode, Project, ProjectId, RetrievalEvalCase,
    RetrievalEvalCaseId, RetrievalEvalDataset, RetrievalEvalDatasetId, RetrievalEvalDatasetSummary,
    RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalRun, RetrievalQueryRequest,
    RetrievalQueryResponse, RetrievalQueryRunId, SearchableChunk, Source, SourceSummary, Trace,
    TraceId, TraceSummary, User, UserId, UserWithPassword, Workspace, WorkspaceId, WorkspaceRole,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    repository::{
        AuthRepository, CiEvalRepository, DemoRepository, DocumentRepository, EmbeddingRepository,
        EvalRepository, HealthRepository, ProjectRepository, ReportRepository, RetrievalRepository,
        SourceRepository, SubmittedExpectedEvidence, TraceRepository,
    },
    StorageError,
};

mod evidence;
mod evidence_index;

#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    inner: Arc<Mutex<MemoryStoreInner>>,
}

#[derive(Debug, Default)]
struct MemoryStoreInner {
    projects: HashMap<ProjectId, Project>,
    project_workspaces: HashMap<ProjectId, WorkspaceId>,
    sources: HashMap<rag_debugger_core::SourceId, Source>,
    runs: HashMap<IngestionRunId, IngestionRun>,
    documents: HashMap<DocumentId, Document>,
    chunks: HashMap<DocumentId, Vec<Chunk>>,
    chunks_by_id: HashMap<ChunkId, Chunk>,
    evidence_browse_indexes: evidence_index::EvidenceBrowseIndexes,
    embeddings: HashMap<ChunkId, ChunkEmbedding>,
    retrieval_runs: HashMap<rag_debugger_core::RetrievalQueryRunId, RetrievalQueryResponse>,
    retrieval_run_workspaces: HashMap<rag_debugger_core::RetrievalQueryRunId, WorkspaceId>,
    retrieval_eval_cases: HashMap<RetrievalEvalCaseId, RetrievalEvalCase>,
    retrieval_eval_case_datasets: HashMap<RetrievalEvalCaseId, RetrievalEvalDatasetId>,
    retrieval_eval_datasets: HashMap<RetrievalEvalDatasetId, RetrievalEvalDataset>,
    retrieval_eval_dataset_workspaces: HashMap<RetrievalEvalDatasetId, WorkspaceId>,
    default_eval_datasets: HashMap<WorkspaceId, RetrievalEvalDatasetId>,
    retrieval_eval_experiments: HashMap<RetrievalEvalExperimentId, RetrievalEvalExperiment>,
    retrieval_eval_runs: HashMap<rag_debugger_core::RetrievalEvalRunId, RetrievalEvalRun>,
    retrieval_eval_run_workspaces: HashMap<rag_debugger_core::RetrievalEvalRunId, WorkspaceId>,
    traces: HashMap<TraceId, Trace>,
    trace_workspaces: HashMap<TraceId, WorkspaceId>,
    organizations: HashMap<rag_debugger_core::OrganizationId, Organization>,
    workspaces: HashMap<WorkspaceId, Workspace>,
    users: HashMap<UserId, User>,
    user_password_hashes: HashMap<UserId, String>,
    memberships: HashMap<(UserId, WorkspaceId), WorkspaceRole>,
    auth_sessions: HashMap<String, AuthSessionRecord>,
    api_keys: HashMap<ApiKeyId, ApiKeyRecord>,
    ci_eval_runs: HashMap<CiEvalRunId, CiEvalRun>,
    debug_reports: HashMap<DebugReportId, DebugReport>,
}

#[async_trait]
impl HealthRepository for MemoryStore {
    async fn ping(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait]
impl ProjectRepository for MemoryStore {
    async fn ensure_default_project(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Project, StorageError> {
        let mut inner = self.lock()?;
        if !inner.workspaces.contains_key(&workspace_id) {
            return Err(StorageError::NotFound);
        }
        if let Some(project) = inner
            .projects
            .values()
            .find(|project| inner.project_workspaces.get(&project.id) == Some(&workspace_id))
        {
            return Ok(project.clone());
        }

        let now = OffsetDateTime::now_utc();
        let project = Project {
            id: ProjectId(Uuid::now_v7()),
            name: "Corpus Workspace".to_owned(),
            privacy_mode: PrivacyMode::LocalOnly,
            created_at: now,
            updated_at: now,
        };
        inner.projects.insert(project.id, project.clone());
        inner.project_workspaces.insert(project.id, workspace_id);
        Ok(project)
    }

    async fn get_project(
        &self,
        workspace_id: WorkspaceId,
        project_id: ProjectId,
    ) -> Result<Project, StorageError> {
        let inner = self.lock()?;
        if inner.project_workspaces.get(&project_id) != Some(&workspace_id) {
            return Err(StorageError::NotFound);
        }
        inner
            .projects
            .get(&project_id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }
}

#[async_trait]
impl SourceRepository for MemoryStore {
    async fn create_source(
        &self,
        workspace_id: WorkspaceId,
        source: Source,
    ) -> Result<Source, StorageError> {
        let mut inner = self.lock()?;
        if inner.project_workspaces.get(&source.project_id) != Some(&workspace_id) {
            return Err(StorageError::NotFound);
        }
        inner.sources.insert(source.id, source.clone());
        Ok(source)
    }

    async fn create_ingestion_run(
        &self,
        workspace_id: WorkspaceId,
        run: IngestionRun,
    ) -> Result<IngestionRun, StorageError> {
        let mut inner = self.lock()?;
        if !workspace_owns_source(&inner, workspace_id, run.source_id) {
            return Err(StorageError::NotFound);
        }
        inner.runs.insert(run.id, run.clone());
        Ok(run)
    }

    async fn complete_ingestion_run(
        &self,
        workspace_id: WorkspaceId,
        id: IngestionRunId,
        status: IngestionRunStatus,
        totals: IngestionTotals,
    ) -> Result<IngestionRun, StorageError> {
        let mut inner = self.lock()?;
        let source_id = inner
            .runs
            .get(&id)
            .map(|run| run.source_id)
            .ok_or(StorageError::NotFound)?;
        if !workspace_owns_source(&inner, workspace_id, source_id) {
            return Err(StorageError::NotFound);
        }
        let run = inner.runs.get_mut(&id).ok_or(StorageError::NotFound)?;
        run.status = status;
        run.totals = totals;
        run.completed_at = Some(OffsetDateTime::now_utc());
        Ok(run.clone())
    }

    async fn list_sources(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<SourceSummary>, StorageError> {
        let inner = self.lock()?;
        let mut summaries = Vec::new();

        for source in inner
            .sources
            .values()
            .filter(|source| workspace_owns_source(&inner, workspace_id, source.id))
        {
            let documents = inner
                .documents
                .values()
                .filter(|document| document.source_id == source.id)
                .map(|document| {
                    let chunk_count = inner
                        .chunks
                        .get(&document.id)
                        .map_or(0, |chunks| chunks.len() as u32);
                    DocumentSummary {
                        document: document.clone(),
                        chunk_count,
                    }
                })
                .collect::<Vec<_>>();

            summaries.push(SourceSummary {
                source: source.clone(),
                document_count: documents.len() as u32,
                chunk_count: documents.iter().map(|document| document.chunk_count).sum(),
                documents,
            });
        }

        Ok(summaries)
    }
}

#[async_trait]
impl DocumentRepository for MemoryStore {
    async fn insert_document_with_chunks(
        &self,
        workspace_id: WorkspaceId,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<Document, StorageError> {
        let mut inner = self.lock()?;
        if !workspace_owns_source(&inner, workspace_id, document.source_id) {
            return Err(StorageError::NotFound);
        }
        inner.replace_document_with_chunks(document.clone(), chunks);
        Ok(document)
    }

    async fn list_document_chunks(
        &self,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
    ) -> Result<Vec<Chunk>, StorageError> {
        let inner = self.lock()?;
        if !workspace_owns_document(&inner, workspace_id, document_id) {
            return Err(StorageError::NotFound);
        }
        let mut chunks = inner.chunks.get(&document_id).cloned().unwrap_or_default();
        chunks.sort_by_key(|chunk| chunk.ordinal);
        Ok(chunks)
    }
}

#[async_trait]
impl RetrievalRepository for MemoryStore {
    async fn list_searchable_chunks(
        &self,
        workspace_id: WorkspaceId,
        request: &RetrievalQueryRequest,
    ) -> Result<Vec<SearchableChunk>, StorageError> {
        let inner = self.lock()?;
        let mut candidates = Vec::new();

        for chunks in inner.chunks.values() {
            for chunk in chunks {
                if !workspace_owns_source(&inner, workspace_id, chunk.source_id) {
                    continue;
                }
                if !request.source_ids.is_empty() && !request.source_ids.contains(&chunk.source_id)
                {
                    continue;
                }

                if !request.document_ids.is_empty()
                    && !request.document_ids.contains(&chunk.document_id)
                {
                    continue;
                }

                let Some(document) = inner.documents.get(&chunk.document_id) else {
                    continue;
                };
                let Some(source) = inner.sources.get(&chunk.source_id) else {
                    continue;
                };

                candidates.push(SearchableChunk {
                    source: source.clone(),
                    document: document.clone(),
                    chunk: chunk.clone(),
                    embedding: inner.embeddings.get(&chunk.id).cloned(),
                });
            }
        }

        candidates.sort_by(|left, right| {
            left.document
                .path
                .cmp(&right.document.path)
                .then_with(|| left.chunk.ordinal.cmp(&right.chunk.ordinal))
        });
        Ok(candidates)
    }

    async fn save_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
        response: &RetrievalQueryResponse,
    ) -> Result<(), StorageError> {
        let mut inner = self.lock()?;
        if response.hits.iter().any(|hit| {
            !workspace_owns_chunk(&inner, workspace_id, hit.chunk.id)
                || !workspace_owns_project(&inner, workspace_id, hit.source.project_id)
        }) || inner
            .retrieval_run_workspaces
            .get(&response.run.id)
            .is_some_and(|owner| *owner != workspace_id)
        {
            return Err(StorageError::NotFound);
        }
        inner
            .retrieval_runs
            .insert(response.run.id, response.clone());
        inner
            .retrieval_run_workspaces
            .insert(response.run.id, workspace_id);
        Ok(())
    }

    async fn get_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
        id: RetrievalQueryRunId,
    ) -> Result<RetrievalQueryResponse, StorageError> {
        let inner = self.lock()?;
        if inner.retrieval_run_workspaces.get(&id) != Some(&workspace_id) {
            return Err(StorageError::NotFound);
        }
        inner
            .retrieval_runs
            .get(&id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    async fn latest_retrieval_query(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<RetrievalQueryResponse, StorageError> {
        let inner = self.lock()?;
        inner
            .retrieval_runs
            .values()
            .filter(|response| {
                inner.retrieval_run_workspaces.get(&response.run.id) == Some(&workspace_id)
            })
            .max_by_key(|response| response.run.created_at)
            .cloned()
            .ok_or(StorageError::NotFound)
    }
}

#[async_trait]
impl TraceRepository for MemoryStore {
    async fn save_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<Trace, StorageError> {
        let mut inner = self.lock()?;
        if !workspace_owns_project(&inner, workspace_id, trace.project_id)
            || trace.source_run_id.is_some_and(|run_id| {
                inner.retrieval_run_workspaces.get(&run_id) != Some(&workspace_id)
            })
            || inner
                .trace_workspaces
                .get(&trace.id)
                .is_some_and(|owner| *owner != workspace_id)
        {
            return Err(StorageError::NotFound);
        }
        inner.traces.insert(trace.id, trace.clone());
        inner.trace_workspaces.insert(trace.id, workspace_id);
        Ok(trace)
    }

    async fn upsert_imported_trace(
        &self,
        workspace_id: WorkspaceId,
        trace: Trace,
    ) -> Result<ImportedTraceUpsertResult, StorageError> {
        let mut inner = self.lock()?;
        if !workspace_owns_project(&inner, workspace_id, trace.project_id) {
            return Err(StorageError::NotFound);
        }
        let identity = trace.ingestion.as_ref().ok_or_else(|| {
            StorageError::InvalidData("imported trace metadata is required".to_owned())
        })?;
        let existing = inner
            .traces
            .values()
            .find(|candidate| {
                inner.trace_workspaces.get(&candidate.id) == Some(&workspace_id)
                    && candidate.project_id == trace.project_id
                    && candidate.ingestion.as_ref().is_some_and(|metadata| {
                        metadata.source == identity.source
                            && metadata.external_trace_id == identity.external_trace_id
                    })
            })
            .cloned();
        let (saved, disposition) = if let Some(existing) = existing {
            let merged = rag_debugger_core::merge_imported_trace(&existing, trace)
                .map_err(|message| StorageError::Conflict(message.to_owned()))?;
            let disposition = if merged == existing {
                rag_debugger_core::TraceIngestionDisposition::Unchanged
            } else {
                rag_debugger_core::TraceIngestionDisposition::Updated
            };
            (merged, disposition)
        } else {
            (trace, rag_debugger_core::TraceIngestionDisposition::Created)
        };
        inner.trace_workspaces.insert(saved.id, workspace_id);
        inner.traces.insert(saved.id, saved.clone());
        Ok(ImportedTraceUpsertResult {
            trace: saved,
            disposition,
        })
    }

    async fn list_traces(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<TraceSummary>, StorageError> {
        let inner = self.lock()?;
        let mut traces = inner
            .traces
            .values()
            .filter(|trace| inner.trace_workspaces.get(&trace.id) == Some(&workspace_id))
            .map(trace_summary_from_trace)
            .collect::<Vec<_>>();
        traces.sort_by_key(|trace| Reverse(trace.created_at));
        Ok(traces)
    }

    async fn get_trace_detail(
        &self,
        workspace_id: WorkspaceId,
        id: TraceId,
    ) -> Result<Trace, StorageError> {
        let inner = self.lock()?;
        if inner.trace_workspaces.get(&id) != Some(&workspace_id) {
            return Err(StorageError::NotFound);
        }
        inner.traces.get(&id).cloned().ok_or(StorageError::NotFound)
    }
}

#[async_trait]
impl EmbeddingRepository for MemoryStore {
    async fn embedding_status(
        &self,
        workspace_id: WorkspaceId,
        request: &EmbeddingIndexRequest,
        model: &EmbeddingModelInfo,
    ) -> Result<EmbeddingStatus, StorageError> {
        let inner = self.lock()?;
        let chunks = filtered_chunks(&inner, workspace_id, request);
        let mut indexed_chunks = 0u32;
        let mut missing_chunks = 0u32;
        let mut stale_chunks = 0u32;
        let mut last_indexed_at: Option<OffsetDateTime> = None;

        for chunk in &chunks {
            match inner.embeddings.get(&chunk.id) {
                Some(embedding)
                    if embedding.model == *model
                        && embedding.chunk_checksum == chunk.checksum
                        && embedding.vector.len() == model.dimension as usize =>
                {
                    indexed_chunks += 1;
                    last_indexed_at = Some(
                        last_indexed_at
                            .map(|current| current.max(embedding.indexed_at))
                            .unwrap_or(embedding.indexed_at),
                    );
                }
                Some(_) => stale_chunks += 1,
                None => missing_chunks += 1,
            }
        }

        Ok(EmbeddingStatus {
            model: model.clone(),
            total_chunks: chunks.len() as u32,
            indexed_chunks,
            missing_chunks,
            stale_chunks,
            last_indexed_at,
        })
    }

    async fn list_embedding_candidates(
        &self,
        workspace_id: WorkspaceId,
        request: &EmbeddingIndexRequest,
    ) -> Result<Vec<EmbeddingIndexCandidate>, StorageError> {
        let inner = self.lock()?;
        Ok(filtered_chunks(&inner, workspace_id, request)
            .into_iter()
            .map(|chunk| EmbeddingIndexCandidate {
                chunk_id: chunk.id,
                source_id: chunk.source_id,
                document_id: chunk.document_id,
                text: chunk.text,
                checksum: chunk.checksum,
                chunking_strategy: chunk.strategy,
            })
            .collect())
    }

    async fn upsert_chunk_embeddings(
        &self,
        workspace_id: WorkspaceId,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), StorageError> {
        let mut inner = self.lock()?;
        if embeddings
            .iter()
            .any(|embedding| !workspace_owns_chunk(&inner, workspace_id, embedding.chunk_id))
        {
            return Err(StorageError::NotFound);
        }
        for embedding in embeddings {
            inner.embeddings.insert(embedding.chunk_id, embedding);
        }
        Ok(())
    }
}

#[async_trait]
impl EvalRepository for MemoryStore {
    async fn create_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let mut inner = self.lock()?;
        validate_expected_evidence_owned(
            &inner,
            workspace_id,
            &eval_case.expected_document_ids,
            &eval_case.expected_chunk_ids,
        )?;
        let dataset_id = ensure_default_eval_dataset(&mut inner, workspace_id)?.id;
        inner
            .retrieval_eval_case_datasets
            .insert(eval_case.id, dataset_id);
        inner
            .retrieval_eval_cases
            .insert(eval_case.id, eval_case.clone());
        Ok(eval_case)
    }

    async fn list_retrieval_eval_cases(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        let inner = self.lock()?;
        let mut cases = inner
            .retrieval_eval_cases
            .values()
            .filter(|eval_case| eval_case_owned_by(&inner, workspace_id, eval_case.id))
            .cloned()
            .collect::<Vec<_>>();
        cases.sort_by_key(|eval_case| Reverse(eval_case.created_at));
        Ok(cases)
    }

    async fn list_retrieval_eval_cases_by_id(
        &self,
        workspace_id: WorkspaceId,
        case_ids: &[RetrievalEvalCaseId],
    ) -> Result<Vec<RetrievalEvalCase>, StorageError> {
        let inner = self.lock()?;
        let mut cases = case_ids
            .iter()
            .filter(|case_id| eval_case_owned_by(&inner, workspace_id, **case_id))
            .filter_map(|case_id| inner.retrieval_eval_cases.get(case_id).cloned())
            .collect::<Vec<_>>();
        cases.sort_by_key(|eval_case| Reverse(eval_case.created_at));
        Ok(cases)
    }

    async fn get_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let inner = self.lock()?;
        if !eval_case_owned_by(&inner, workspace_id, case_id) {
            return Err(StorageError::NotFound);
        }
        inner
            .retrieval_eval_cases
            .get(&case_id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    async fn save_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
        eval_run: &RetrievalEvalRun,
    ) -> Result<(), StorageError> {
        let mut inner = self.lock()?;
        if eval_run
            .results
            .iter()
            .any(|result| !eval_case_owned_by(&inner, workspace_id, result.case_id))
        {
            return Err(StorageError::NotFound);
        }
        inner
            .retrieval_eval_runs
            .insert(eval_run.id, eval_run.clone());
        inner
            .retrieval_eval_run_workspaces
            .insert(eval_run.id, workspace_id);
        Ok(())
    }

    async fn latest_retrieval_eval_run(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalRun>, StorageError> {
        let inner = self.lock()?;
        Ok(inner
            .retrieval_eval_runs
            .values()
            .filter(|run| inner.retrieval_eval_run_workspaces.get(&run.id) == Some(&workspace_id))
            .max_by_key(|run| run.created_at)
            .cloned())
    }

    async fn create_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset: RetrievalEvalDataset,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        let mut inner = self.lock()?;
        inner
            .retrieval_eval_datasets
            .insert(dataset.id, dataset.clone());
        inner
            .retrieval_eval_dataset_workspaces
            .insert(dataset.id, workspace_id);
        Ok(dataset)
    }

    async fn list_retrieval_eval_datasets(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalDatasetSummary>, StorageError> {
        let mut inner = self.lock()?;
        ensure_default_eval_dataset(&mut inner, workspace_id)?;
        let mut summaries = inner
            .retrieval_eval_datasets
            .values()
            .filter(|dataset| dataset_owned_by(&inner, workspace_id, dataset.id))
            .map(|dataset| eval_dataset_summary(&inner, dataset))
            .collect::<Vec<_>>();
        summaries.sort_by_key(|summary| Reverse(summary.updated_at));
        Ok(summaries)
    }

    async fn get_retrieval_eval_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<RetrievalEvalDataset, StorageError> {
        let mut inner = self.lock()?;
        ensure_default_eval_dataset(&mut inner, workspace_id)?;
        if !dataset_owned_by(&inner, workspace_id, dataset_id) {
            return Err(StorageError::NotFound);
        }
        let mut dataset = inner
            .retrieval_eval_datasets
            .get(&dataset_id)
            .cloned()
            .ok_or(StorageError::NotFound)?;
        dataset.cases = cases_for_dataset(&inner, dataset_id);
        Ok(dataset)
    }

    async fn create_retrieval_eval_case_in_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
        eval_case: RetrievalEvalCase,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let mut inner = self.lock()?;
        if !dataset_owned_by(&inner, workspace_id, dataset_id) {
            return Err(StorageError::NotFound);
        }
        validate_expected_evidence_owned(
            &inner,
            workspace_id,
            &eval_case.expected_document_ids,
            &eval_case.expected_chunk_ids,
        )?;
        inner
            .retrieval_eval_case_datasets
            .insert(eval_case.id, dataset_id);
        inner
            .retrieval_eval_cases
            .insert(eval_case.id, eval_case.clone());
        touch_dataset(&mut inner, dataset_id);
        Ok(eval_case)
    }

    async fn update_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        eval_case: RetrievalEvalCase,
        submitted_evidence: SubmittedExpectedEvidence,
    ) -> Result<RetrievalEvalCase, StorageError> {
        let mut inner = self.lock()?;
        if !eval_case_owned_by(&inner, workspace_id, eval_case.id) {
            return Err(StorageError::NotFound);
        }
        validate_expected_evidence_owned(
            &inner,
            workspace_id,
            submitted_evidence
                .document_ids
                .as_deref()
                .unwrap_or_default(),
            submitted_evidence.chunk_ids.as_deref().unwrap_or_default(),
        )?;
        inner
            .retrieval_eval_cases
            .insert(eval_case.id, eval_case.clone());
        if let Some(dataset_id) = inner
            .retrieval_eval_case_datasets
            .get(&eval_case.id)
            .copied()
        {
            touch_dataset(&mut inner, dataset_id);
        }
        Ok(eval_case)
    }

    async fn delete_retrieval_eval_case(
        &self,
        workspace_id: WorkspaceId,
        case_id: RetrievalEvalCaseId,
    ) -> Result<(), StorageError> {
        let mut inner = self.lock()?;
        if !eval_case_owned_by(&inner, workspace_id, case_id) {
            return Err(StorageError::NotFound);
        }
        if inner.retrieval_eval_cases.remove(&case_id).is_none() {
            return Err(StorageError::NotFound);
        }
        if let Some(dataset_id) = inner.retrieval_eval_case_datasets.remove(&case_id) {
            touch_dataset(&mut inner, dataset_id);
        }
        Ok(())
    }

    async fn save_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
        experiment: RetrievalEvalExperiment,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        let mut inner = self.lock()?;
        if !dataset_owned_by(&inner, workspace_id, experiment.dataset_id) {
            return Err(StorageError::NotFound);
        }
        inner
            .retrieval_eval_experiments
            .insert(experiment.id, experiment.clone());
        touch_dataset(&mut inner, experiment.dataset_id);
        Ok(experiment)
    }

    async fn list_retrieval_eval_experiments(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        let inner = self.lock()?;
        let mut experiments = inner
            .retrieval_eval_experiments
            .values()
            .filter(|experiment| dataset_owned_by(&inner, workspace_id, experiment.dataset_id))
            .cloned()
            .collect::<Vec<_>>();
        experiments.sort_by_key(|experiment| Reverse(experiment.created_at));
        Ok(experiments)
    }

    async fn list_retrieval_eval_experiments_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
    ) -> Result<Vec<RetrievalEvalExperiment>, StorageError> {
        let inner = self.lock()?;
        if !dataset_owned_by(&inner, workspace_id, dataset_id) {
            return Err(StorageError::NotFound);
        }
        let mut experiments = inner
            .retrieval_eval_experiments
            .values()
            .filter(|experiment| experiment.dataset_id == dataset_id)
            .cloned()
            .collect::<Vec<_>>();
        experiments.sort_by_key(|experiment| Reverse(experiment.created_at));
        Ok(experiments)
    }

    async fn get_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
        experiment_id: RetrievalEvalExperimentId,
    ) -> Result<RetrievalEvalExperiment, StorageError> {
        let inner = self.lock()?;
        let experiment = inner
            .retrieval_eval_experiments
            .get(&experiment_id)
            .cloned()
            .ok_or(StorageError::NotFound)?;
        if !dataset_owned_by(&inner, workspace_id, experiment.dataset_id) {
            return Err(StorageError::NotFound);
        }
        Ok(experiment)
    }

    async fn latest_retrieval_eval_experiment(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Option<RetrievalEvalExperiment>, StorageError> {
        let inner = self.lock()?;
        Ok(inner
            .retrieval_eval_experiments
            .values()
            .filter(|experiment| dataset_owned_by(&inner, workspace_id, experiment.dataset_id))
            .max_by_key(|experiment| experiment.created_at)
            .cloned())
    }
}

#[async_trait]
impl AuthRepository for MemoryStore {
    async fn bootstrap_identity(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError> {
        let mut inner = self.lock()?;
        if let Some(existing) = find_user_with_auth(&inner, &user.email)? {
            return Ok(existing.auth);
        }

        inner
            .organizations
            .insert(organization.id, organization.clone());
        inner.workspaces.insert(workspace.id, workspace.clone());
        inner.users.insert(user.id, user.clone());
        inner.user_password_hashes.insert(user.id, password_hash);
        inner.memberships.insert((user.id, workspace.id), role);
        claim_unowned_legacy_data(&mut inner, workspace.id);

        Ok(AuthenticatedUser {
            user,
            organization,
            workspace,
            role,
        })
    }

    async fn create_user_workspace(
        &self,
        organization: Organization,
        workspace: Workspace,
        user: User,
        role: WorkspaceRole,
        password_hash: String,
    ) -> Result<AuthenticatedUser, StorageError> {
        let mut inner = self.lock()?;
        if find_user_with_auth(&inner, &user.email)?.is_some() {
            return Err(StorageError::Conflict(
                "user email already exists".to_owned(),
            ));
        }

        inner
            .organizations
            .insert(organization.id, organization.clone());
        inner.workspaces.insert(workspace.id, workspace.clone());
        inner.users.insert(user.id, user.clone());
        inner.user_password_hashes.insert(user.id, password_hash);
        inner.memberships.insert((user.id, workspace.id), role);
        claim_unowned_legacy_data(&mut inner, workspace.id);

        Ok(AuthenticatedUser {
            user,
            organization,
            workspace,
            role,
        })
    }

    async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithPassword>, StorageError> {
        let inner = self.lock()?;
        find_user_with_auth(&inner, email)
    }

    async fn get_authenticated_user(
        &self,
        user_id: UserId,
        workspace_id: WorkspaceId,
    ) -> Result<AuthenticatedUser, StorageError> {
        let inner = self.lock()?;
        authenticated_user(&inner, user_id, workspace_id)
    }

    async fn create_auth_session(
        &self,
        session: AuthSessionRecord,
    ) -> Result<AuthSessionRecord, StorageError> {
        let mut inner = self.lock()?;
        inner
            .auth_sessions
            .insert(session.token_hash.clone(), session.clone());
        Ok(session)
    }

    async fn find_auth_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<AuthSessionRecord>, StorageError> {
        let inner = self.lock()?;
        Ok(inner.auth_sessions.get(token_hash).cloned())
    }

    async fn revoke_auth_session(&self, token_hash: &str) -> Result<(), StorageError> {
        let mut inner = self.lock()?;
        if let Some(session) = inner.auth_sessions.get_mut(token_hash) {
            session.revoked_at = Some(OffsetDateTime::now_utc());
        }
        Ok(())
    }

    async fn create_api_key(&self, record: ApiKeyRecord) -> Result<ApiKeyRecord, StorageError> {
        let mut inner = self.lock()?;
        inner.api_keys.insert(record.api_key.id, record.clone());
        Ok(record)
    }

    async fn list_api_keys(&self, workspace_id: WorkspaceId) -> Result<Vec<ApiKey>, StorageError> {
        let inner = self.lock()?;
        let mut keys = inner
            .api_keys
            .values()
            .filter(|record| record.api_key.workspace_id == workspace_id)
            .map(|record| record.api_key.clone())
            .collect::<Vec<_>>();
        keys.sort_by_key(|key| Reverse(key.created_at));
        Ok(keys)
    }

    async fn find_api_key(&self, secret_hash: &str) -> Result<Option<ApiKeyRecord>, StorageError> {
        let inner = self.lock()?;
        Ok(inner
            .api_keys
            .values()
            .find(|record| record.secret_hash == secret_hash)
            .cloned())
    }

    async fn touch_api_key(&self, api_key_id: ApiKeyId) -> Result<(), StorageError> {
        let mut inner = self.lock()?;
        if let Some(record) = inner.api_keys.get_mut(&api_key_id) {
            record.api_key.last_used_at = Some(OffsetDateTime::now_utc());
        }
        Ok(())
    }

    async fn revoke_api_key(
        &self,
        workspace_id: WorkspaceId,
        api_key_id: ApiKeyId,
    ) -> Result<(), StorageError> {
        let mut inner = self.lock()?;
        let record = inner
            .api_keys
            .get_mut(&api_key_id)
            .filter(|record| record.api_key.workspace_id == workspace_id)
            .ok_or(StorageError::NotFound)?;
        record.api_key.revoked_at = Some(OffsetDateTime::now_utc());
        Ok(())
    }
}

#[async_trait]
impl CiEvalRepository for MemoryStore {
    async fn save_ci_eval_run(&self, run: CiEvalRun) -> Result<CiEvalRun, StorageError> {
        let mut inner = self.lock()?;
        if !dataset_owned_by(&inner, run.workspace_id, run.dataset_id) {
            return Err(StorageError::NotFound);
        }
        inner.ci_eval_runs.insert(run.id, run.clone());
        Ok(run)
    }

    async fn list_ci_eval_runs(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<CiEvalRun>, StorageError> {
        let inner = self.lock()?;
        let mut runs = inner
            .ci_eval_runs
            .values()
            .filter(|run| run.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        runs.sort_by_key(|run| Reverse(run.created_at));
        Ok(runs)
    }

    async fn get_ci_eval_run(
        &self,
        workspace_id: WorkspaceId,
        id: CiEvalRunId,
    ) -> Result<CiEvalRun, StorageError> {
        let inner = self.lock()?;
        inner
            .ci_eval_runs
            .get(&id)
            .filter(|run| run.workspace_id == workspace_id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    async fn latest_ci_eval_run_for_dataset(
        &self,
        workspace_id: WorkspaceId,
        dataset_id: RetrievalEvalDatasetId,
        config_label: &str,
    ) -> Result<Option<CiEvalRun>, StorageError> {
        let inner = self.lock()?;
        Ok(inner
            .ci_eval_runs
            .values()
            .filter(|run| {
                run.workspace_id == workspace_id
                    && run.dataset_id == dataset_id
                    && run.config_label == config_label
            })
            .max_by_key(|run| run.created_at)
            .cloned())
    }
}

#[async_trait]
impl ReportRepository for MemoryStore {
    async fn save_debug_report(&self, report: DebugReport) -> Result<DebugReport, StorageError> {
        let mut inner = self.lock()?;
        if inner.debug_reports.contains_key(&report.id) {
            return Err(StorageError::Conflict(format!(
                "debug report {}",
                report.id.0
            )));
        }
        inner.debug_reports.insert(report.id, report.clone());
        Ok(report)
    }

    async fn list_debug_reports(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<DebugReport>, StorageError> {
        let inner = self.lock()?;
        let mut reports = inner
            .debug_reports
            .values()
            .filter(|report| report.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        reports.sort_by_key(|report| (Reverse(report.created_at), Reverse(report.id.0)));
        Ok(reports)
    }

    async fn get_debug_report(
        &self,
        workspace_id: WorkspaceId,
        report_id: DebugReportId,
    ) -> Result<DebugReport, StorageError> {
        let inner = self.lock()?;
        inner
            .debug_reports
            .get(&report_id)
            .filter(|report| report.workspace_id == workspace_id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }
}

#[async_trait]
impl DemoRepository for MemoryStore {
    async fn ensure_demo_project(
        &self,
        workspace_id: WorkspaceId,
        project: Project,
    ) -> Result<Project, StorageError> {
        let mut inner = self.lock()?;
        if let Some(existing_workspace) = inner.project_workspaces.get(&project.id) {
            if *existing_workspace != workspace_id {
                return Err(StorageError::NotFound);
            }
        }
        inner
            .projects
            .entry(project.id)
            .or_insert_with(|| project.clone());
        inner.project_workspaces.insert(project.id, workspace_id);
        Ok(inner.projects[&project.id].clone())
    }

    async fn ensure_demo_source(&self, source: Source) -> Result<Source, StorageError> {
        let mut inner = self.lock()?;
        if !inner.projects.contains_key(&source.project_id) {
            return Err(StorageError::NotFound);
        }
        inner
            .sources
            .entry(source.id)
            .or_insert_with(|| source.clone());
        Ok(inner.sources[&source.id].clone())
    }

    async fn upsert_demo_document_with_chunks(
        &self,
        document: Document,
        chunks: Vec<Chunk>,
    ) -> Result<bool, StorageError> {
        let mut inner = self.lock()?;
        if !inner.sources.contains_key(&document.source_id) {
            return Err(StorageError::NotFound);
        }
        let created = !inner.documents.contains_key(&document.id);
        inner.merge_document_with_chunks(document, chunks);
        Ok(created)
    }

    async fn get_demo_source(
        &self,
        workspace_id: WorkspaceId,
        version_marker: &str,
    ) -> Result<Option<SourceSummary>, StorageError> {
        let inner = self.lock()?;
        let source = inner.sources.values().find(|source| {
            inner.project_workspaces.get(&source.project_id) == Some(&workspace_id)
                && matches!(&source.kind, rag_debugger_core::SourceKind::FileSet { root_hint } if root_hint == version_marker)
        });
        Ok(source.map(|source| source_summary(&inner, source)))
    }

    async fn latest_retrieval_query_for_source(
        &self,
        workspace_id: WorkspaceId,
        source_id: rag_debugger_core::SourceId,
    ) -> Result<Option<RetrievalQueryResponse>, StorageError> {
        let inner = self.lock()?;
        if !workspace_owns_source(&inner, workspace_id, source_id) {
            return Ok(None);
        }
        Ok(inner
            .retrieval_runs
            .values()
            .filter(|response| {
                inner.retrieval_run_workspaces.get(&response.run.id) == Some(&workspace_id)
                    && response.hits.iter().any(|hit| hit.source.id == source_id)
            })
            .max_by_key(|response| response.run.created_at)
            .cloned())
    }

    async fn latest_trace_for_source(
        &self,
        workspace_id: WorkspaceId,
        source_id: rag_debugger_core::SourceId,
    ) -> Result<Option<Trace>, StorageError> {
        let inner = self.lock()?;
        if !workspace_owns_source(&inner, workspace_id, source_id) {
            return Ok(None);
        }
        Ok(inner
            .traces
            .values()
            .filter(|trace| {
                inner.trace_workspaces.get(&trace.id) == Some(&workspace_id)
                    && trace.retrieval.as_ref().is_some_and(|response| {
                        response.hits.iter().any(|hit| hit.source.id == source_id)
                    })
            })
            .max_by_key(|trace| trace.started_at)
            .cloned())
    }
}

impl MemoryStore {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, MemoryStoreInner>, StorageError> {
        self.inner
            .lock()
            .map_err(|_| StorageError::Internal("memory store lock poisoned".to_owned()))
    }
}

impl MemoryStoreInner {
    fn replace_document_with_chunks(&mut self, document: Document, mut chunks: Vec<Chunk>) {
        self.remove_document_with_chunks(document.id);
        chunks.sort_by_key(|chunk| (chunk.ordinal, chunk.id.0));

        self.evidence_browse_indexes
            .insert_document(&document.path, document.id);
        self.documents.insert(document.id, document.clone());

        for chunk in &chunks {
            if let Some(referenced_document) = self.documents.get(&chunk.document_id) {
                self.evidence_browse_indexes.insert_chunk(
                    &referenced_document.path,
                    referenced_document.id,
                    chunk.ordinal,
                    chunk.id,
                );
            }
            self.chunks_by_id.insert(chunk.id, chunk.clone());
        }
        self.chunks.insert(document.id, chunks);
    }

    fn merge_document_with_chunks(&mut self, document: Document, chunks: Vec<Chunk>) {
        let mut merged = self.chunks.get(&document.id).cloned().unwrap_or_default();
        for chunk in chunks {
            if let Some(existing) = merged.iter_mut().find(|item| item.id == chunk.id) {
                *existing = chunk;
            } else {
                merged.push(chunk);
            }
        }
        self.replace_document_with_chunks(document, merged);
    }

    fn remove_document_with_chunks(&mut self, document_id: DocumentId) -> Option<Document> {
        let previous_document = self.documents.get(&document_id).cloned();
        if let Some(previous_chunks) = self.chunks.remove(&document_id) {
            for chunk in previous_chunks {
                let referenced_document = if chunk.document_id == document_id {
                    previous_document.as_ref()
                } else {
                    self.documents.get(&chunk.document_id)
                };
                if let Some(referenced_document) = referenced_document {
                    self.evidence_browse_indexes.remove_chunk(
                        &referenced_document.path,
                        referenced_document.id,
                        chunk.ordinal,
                        chunk.id,
                    );
                }
                self.chunks_by_id.remove(&chunk.id);
            }
        }

        if let Some(document) = &previous_document {
            self.evidence_browse_indexes
                .remove_document(&document.path, document.id);
        }
        self.documents.remove(&document_id)
    }
}

fn filtered_chunks(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    request: &EmbeddingIndexRequest,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();

    for document_chunks in inner.chunks.values() {
        for chunk in document_chunks {
            if !workspace_owns_chunk(inner, workspace_id, chunk.id) {
                continue;
            }
            if !request.source_ids.is_empty() && !request.source_ids.contains(&chunk.source_id) {
                continue;
            }

            if !request.document_ids.is_empty()
                && !request.document_ids.contains(&chunk.document_id)
            {
                continue;
            }

            chunks.push(chunk.clone());
        }
    }

    chunks.sort_by(|left, right| {
        left.document_id
            .0
            .cmp(&right.document_id.0)
            .then_with(|| left.ordinal.cmp(&right.ordinal))
    });
    chunks
}

fn source_summary(inner: &MemoryStoreInner, source: &Source) -> SourceSummary {
    let documents = inner
        .documents
        .values()
        .filter(|document| document.source_id == source.id)
        .map(|document| DocumentSummary {
            document: document.clone(),
            chunk_count: inner
                .chunks
                .get(&document.id)
                .map_or(0, |chunks| chunks.len() as u32),
        })
        .collect::<Vec<_>>();
    SourceSummary {
        source: source.clone(),
        document_count: documents.len() as u32,
        chunk_count: documents.iter().map(|document| document.chunk_count).sum(),
        documents,
    }
}

fn legacy_default_eval_dataset_id() -> RetrievalEvalDatasetId {
    RetrievalEvalDatasetId(Uuid::from_u128(0x018f_7a2a_6e2e_7000_a000_0000_0000_e001))
}

fn ensure_default_eval_dataset(
    inner: &mut MemoryStoreInner,
    workspace_id: WorkspaceId,
) -> Result<RetrievalEvalDataset, StorageError> {
    if !inner.workspaces.contains_key(&workspace_id) {
        return Err(StorageError::NotFound);
    }
    if let Some(dataset_id) = inner.default_eval_datasets.get(&workspace_id) {
        if let Some(dataset) = inner.retrieval_eval_datasets.get(dataset_id) {
            return Ok(dataset.clone());
        }
    }

    let now = OffsetDateTime::now_utc();
    let dataset = RetrievalEvalDataset {
        id: if !inner
            .retrieval_eval_datasets
            .contains_key(&legacy_default_eval_dataset_id())
        {
            legacy_default_eval_dataset_id()
        } else {
            RetrievalEvalDatasetId(Uuid::now_v7())
        },
        name: "Default retrieval dataset".to_owned(),
        description: Some("Backfilled and manually saved retrieval eval cases.".to_owned()),
        cases: Vec::new(),
        created_at: now,
        updated_at: now,
    };
    inner
        .retrieval_eval_datasets
        .insert(dataset.id, dataset.clone());
    inner
        .retrieval_eval_dataset_workspaces
        .insert(dataset.id, workspace_id);
    inner.default_eval_datasets.insert(workspace_id, dataset.id);
    for case_id in inner
        .retrieval_eval_cases
        .keys()
        .filter(|case_id| !inner.retrieval_eval_case_datasets.contains_key(case_id))
        .copied()
        .collect::<Vec<_>>()
    {
        inner
            .retrieval_eval_case_datasets
            .entry(case_id)
            .or_insert(dataset.id);
    }
    Ok(dataset)
}

fn touch_dataset(inner: &mut MemoryStoreInner, dataset_id: RetrievalEvalDatasetId) {
    if let Some(dataset) = inner.retrieval_eval_datasets.get_mut(&dataset_id) {
        dataset.updated_at = OffsetDateTime::now_utc();
    }
}

fn cases_for_dataset(
    inner: &MemoryStoreInner,
    dataset_id: RetrievalEvalDatasetId,
) -> Vec<RetrievalEvalCase> {
    let mut cases = inner
        .retrieval_eval_cases
        .values()
        .filter(|eval_case| {
            inner
                .retrieval_eval_case_datasets
                .get(&eval_case.id)
                .copied()
                .unwrap_or(legacy_default_eval_dataset_id())
                == dataset_id
        })
        .cloned()
        .collect::<Vec<_>>();
    cases.sort_by_key(|eval_case| Reverse(eval_case.created_at));
    cases
}

fn workspace_owns_source(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    source_id: rag_debugger_core::SourceId,
) -> bool {
    inner
        .sources
        .get(&source_id)
        .and_then(|source| inner.project_workspaces.get(&source.project_id))
        == Some(&workspace_id)
}

fn workspace_owns_project(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    project_id: ProjectId,
) -> bool {
    inner.project_workspaces.get(&project_id) == Some(&workspace_id)
}

fn workspace_owns_document(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
) -> bool {
    inner
        .documents
        .get(&document_id)
        .is_some_and(|document| workspace_owns_source(inner, workspace_id, document.source_id))
}

fn workspace_owns_chunk(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    chunk_id: ChunkId,
) -> bool {
    inner
        .chunks_by_id
        .get(&chunk_id)
        .is_some_and(|chunk| workspace_owns_source(inner, workspace_id, chunk.source_id))
}

fn dataset_owned_by(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    dataset_id: RetrievalEvalDatasetId,
) -> bool {
    inner.retrieval_eval_dataset_workspaces.get(&dataset_id) == Some(&workspace_id)
}

fn eval_case_owned_by(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    case_id: RetrievalEvalCaseId,
) -> bool {
    inner
        .retrieval_eval_case_datasets
        .get(&case_id)
        .is_some_and(|dataset_id| dataset_owned_by(inner, workspace_id, *dataset_id))
}

fn validate_expected_evidence_owned(
    inner: &MemoryStoreInner,
    workspace_id: WorkspaceId,
    document_ids: &[DocumentId],
    chunk_ids: &[ChunkId],
) -> Result<(), StorageError> {
    if document_ids
        .iter()
        .all(|id| workspace_owns_document(inner, workspace_id, *id))
        && chunk_ids
            .iter()
            .all(|id| workspace_owns_chunk(inner, workspace_id, *id))
    {
        Ok(())
    } else {
        Err(StorageError::UnavailableEvidence)
    }
}

fn claim_unowned_legacy_data(inner: &mut MemoryStoreInner, workspace_id: WorkspaceId) {
    if inner.workspaces.len() != 1 {
        return;
    }
    for project_id in inner.projects.keys().copied().collect::<Vec<_>>() {
        inner
            .project_workspaces
            .entry(project_id)
            .or_insert(workspace_id);
    }
    for dataset_id in inner
        .retrieval_eval_datasets
        .keys()
        .copied()
        .collect::<Vec<_>>()
    {
        inner
            .retrieval_eval_dataset_workspaces
            .entry(dataset_id)
            .or_insert(workspace_id);
        if dataset_id == legacy_default_eval_dataset_id() {
            inner
                .default_eval_datasets
                .entry(workspace_id)
                .or_insert(dataset_id);
        }
    }
    for run_id in inner
        .retrieval_eval_runs
        .keys()
        .copied()
        .collect::<Vec<_>>()
    {
        inner
            .retrieval_eval_run_workspaces
            .entry(run_id)
            .or_insert(workspace_id);
    }
}

fn eval_dataset_summary(
    inner: &MemoryStoreInner,
    dataset: &RetrievalEvalDataset,
) -> RetrievalEvalDatasetSummary {
    let latest_experiment = inner
        .retrieval_eval_experiments
        .values()
        .filter(|experiment| experiment.dataset_id == dataset.id)
        .max_by_key(|experiment| experiment.created_at);
    let best_mode = latest_experiment.and_then(|experiment| {
        experiment.mode_results.iter().max_by(|left, right| {
            left.average_recall_at_k
                .partial_cmp(&right.average_recall_at_k)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    RetrievalEvalDatasetSummary {
        id: dataset.id,
        name: dataset.name.clone(),
        description: dataset.description.clone(),
        case_count: cases_for_dataset(inner, dataset.id).len() as u32,
        latest_experiment_id: latest_experiment.map(|experiment| experiment.id),
        latest_gate: latest_experiment.map(|experiment| experiment.gate.clone()),
        latest_average_recall_at_k: best_mode.map(|mode| mode.average_recall_at_k),
        latest_average_precision_at_k: best_mode.map(|mode| mode.average_precision_at_k),
        updated_at: dataset.updated_at,
    }
}

fn find_user_with_auth(
    inner: &MemoryStoreInner,
    email: &str,
) -> Result<Option<UserWithPassword>, StorageError> {
    let normalized_email = email.trim().to_ascii_lowercase();
    let Some(user) = inner
        .users
        .values()
        .find(|user| user.email == normalized_email)
        .cloned()
    else {
        return Ok(None);
    };
    let Some((&(_, workspace_id), &role)) = inner
        .memberships
        .iter()
        .find(|((user_id, _), _)| *user_id == user.id)
    else {
        return Err(StorageError::InvalidData(
            "user has no workspace membership".to_owned(),
        ));
    };
    let auth = authenticated_user(inner, user.id, workspace_id)?;
    Ok(Some(UserWithPassword {
        auth: AuthenticatedUser { role, ..auth },
        password_hash: inner
            .user_password_hashes
            .get(&user.id)
            .cloned()
            .ok_or_else(|| StorageError::InvalidData("user has no password hash".to_owned()))?,
    }))
}

fn authenticated_user(
    inner: &MemoryStoreInner,
    user_id: UserId,
    workspace_id: WorkspaceId,
) -> Result<AuthenticatedUser, StorageError> {
    let user = inner
        .users
        .get(&user_id)
        .cloned()
        .ok_or(StorageError::NotFound)?;
    let workspace = inner
        .workspaces
        .get(&workspace_id)
        .cloned()
        .ok_or(StorageError::NotFound)?;
    let organization = inner
        .organizations
        .get(&workspace.organization_id)
        .cloned()
        .ok_or(StorageError::NotFound)?;
    let role = inner
        .memberships
        .get(&(user_id, workspace_id))
        .copied()
        .ok_or(StorageError::NotFound)?;

    Ok(AuthenticatedUser {
        user,
        organization,
        workspace,
        role,
    })
}

fn trace_summary_from_trace(trace: &Trace) -> TraceSummary {
    let retrieval = trace.retrieval.as_ref();
    TraceSummary {
        id: trace.id,
        query: trace.input.clone(),
        retrieval_mode: retrieval
            .map(|response| response.run.retrieval_mode)
            .unwrap_or_default(),
        latency_ms: retrieval.map_or(0, |response| response.run.latency_ms),
        evidence_strength: trace
            .evidence_strength
            .or_else(|| {
                retrieval
                    .and_then(|response| response.hits.first().map(|hit| hit.evidence_strength))
            })
            .unwrap_or(rag_debugger_core::EvidenceStrength::Weak),
        failure_labels: trace.failure_labels.clone(),
        span_count: trace
            .ingestion
            .as_ref()
            .map_or(trace.spans.len(), |metadata| metadata.spans.len()) as u32,
        rerun_count: trace.reruns.len() as u32,
        created_at: trace.started_at,
        ingestion_source: trace.ingestion.as_ref().map(|value| value.source),
        external_trace_id: trace
            .ingestion
            .as_ref()
            .map(|value| value.external_trace_id.clone()),
        mapping_status: trace.ingestion.as_ref().map(|value| value.mapping_status),
    }
}
