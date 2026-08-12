use rag_debugger_core::{
    ByteRange, Chunk, ChunkEmbedding, ChunkId, ChunkQualityFlag, ChunkSplitReason, ChunkingConfig,
    ChunkingStrategy, Document, DocumentId, DocumentProfile, EmbeddingIndexRequest,
    EmbeddingModelInfo, EvidenceStrength, ExtractionQuality, ExtractiveAnswer,
    ExtractiveAnswerStatus, Organization, OrganizationId, ProjectId, RetrievalEmbeddingReadiness,
    RetrievalEmbeddingStatus, RetrievalMode, RetrievalQueryResponse, RetrievalQueryRun,
    RetrievalQueryRunId, Source, SourceId, SourceKind, SourceSyncPolicy, Trace, TraceId,
    TraceStatus, User, UserId, Workspace, WorkspaceId, WorkspaceRole,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{
        AuthRepository, DocumentRepository, EmbeddingRepository, ProjectRepository,
        RetrievalRepository, SourceRepository, TraceRepository,
    },
    StorageError,
};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn memory_runtime_repository_enforces_workspace_ownership() {
    run_runtime_workspace_contract(&MemoryStore::default()).await;
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_runtime_repository_enforces_workspace_ownership() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    run_runtime_workspace_contract(&store).await;
}

async fn run_runtime_workspace_contract<R>(repository: &R)
where
    R: AuthRepository
        + DocumentRepository
        + EmbeddingRepository
        + ProjectRepository
        + RetrievalRepository
        + SourceRepository
        + TraceRepository,
{
    let alpha = create_corpus(repository, "alpha").await;
    let beta = create_corpus(repository, "beta").await;
    let request = EmbeddingIndexRequest::default();
    let model = EmbeddingModelInfo::default();

    let alpha_candidates = repository
        .list_embedding_candidates(alpha.workspace_id, &request)
        .await
        .expect("list alpha embedding candidates");
    assert_eq!(
        alpha_candidates
            .iter()
            .map(|candidate| candidate.chunk_id)
            .collect::<Vec<_>>(),
        vec![alpha.chunk.id]
    );
    assert_eq!(
        repository
            .embedding_status(alpha.workspace_id, &request, &model)
            .await
            .expect("read alpha embedding status")
            .total_chunks,
        1
    );
    assert_eq!(
        repository
            .embedding_status(beta.workspace_id, &request, &model)
            .await
            .expect("read beta embedding status")
            .total_chunks,
        1
    );

    let beta_embedding = embedding(&beta.chunk, &model);
    assert!(matches!(
        repository
            .upsert_chunk_embeddings(alpha.workspace_id, vec![beta_embedding.clone()])
            .await,
        Err(StorageError::NotFound)
    ));
    assert_eq!(
        repository
            .embedding_status(beta.workspace_id, &request, &model)
            .await
            .expect("read unchanged beta embedding status")
            .missing_chunks,
        1
    );
    repository
        .upsert_chunk_embeddings(beta.workspace_id, vec![beta_embedding])
        .await
        .expect("index beta embedding in owning workspace");
    assert_eq!(
        repository
            .embedding_status(alpha.workspace_id, &request, &model)
            .await
            .expect("read isolated alpha embedding status")
            .indexed_chunks,
        0
    );
    assert_eq!(
        repository
            .embedding_status(beta.workspace_id, &request, &model)
            .await
            .expect("read indexed beta embedding status")
            .indexed_chunks,
        1
    );

    let beta_run = retrieval_response("beta retrieval");
    repository
        .save_retrieval_query(beta.workspace_id, &beta_run)
        .await
        .expect("save beta retrieval run");
    assert!(matches!(
        repository
            .get_retrieval_query(alpha.workspace_id, beta_run.run.id)
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repository.latest_retrieval_query(alpha.workspace_id).await,
        Err(StorageError::NotFound)
    ));
    assert_eq!(
        repository
            .get_retrieval_query(beta.workspace_id, beta_run.run.id)
            .await
            .expect("read owned beta retrieval run")
            .run
            .id,
        beta_run.run.id
    );

    let beta_trace = trace(beta.project_id, &beta_run);
    repository
        .save_trace(beta.workspace_id, beta_trace.clone())
        .await
        .expect("save beta trace");
    assert!(repository
        .list_traces(alpha.workspace_id)
        .await
        .expect("list alpha traces")
        .is_empty());
    assert!(matches!(
        repository
            .get_trace_detail(alpha.workspace_id, beta_trace.id)
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repository
            .save_trace(alpha.workspace_id, beta_trace.clone())
            .await,
        Err(StorageError::NotFound)
    ));

    let trace_with_foreign_run = Trace {
        id: TraceId(Uuid::now_v7()),
        project_id: alpha.project_id,
        source_run_id: Some(beta_run.run.id),
        ..trace(alpha.project_id, &retrieval_response("alpha trace"))
    };
    assert!(matches!(
        repository
            .save_trace(alpha.workspace_id, trace_with_foreign_run)
            .await,
        Err(StorageError::NotFound)
    ));

    let alpha_run = retrieval_response("alpha retrieval");
    repository
        .save_retrieval_query(alpha.workspace_id, &alpha_run)
        .await
        .expect("save alpha retrieval run");
    let alpha_trace = trace(alpha.project_id, &alpha_run);
    repository
        .save_trace(alpha.workspace_id, alpha_trace.clone())
        .await
        .expect("save alpha trace");
    assert_eq!(
        repository
            .list_traces(alpha.workspace_id)
            .await
            .expect("list owned alpha traces")
            .iter()
            .map(|summary| summary.id)
            .collect::<Vec<_>>(),
        vec![alpha_trace.id]
    );
    assert_eq!(
        repository
            .get_trace_detail(alpha.workspace_id, alpha_trace.id)
            .await
            .expect("read owned alpha trace")
            .id,
        alpha_trace.id
    );
}

struct CorpusFixture {
    workspace_id: WorkspaceId,
    project_id: ProjectId,
    chunk: Chunk,
}

async fn create_corpus<R>(repository: &R, label: &str) -> CorpusFixture
where
    R: AuthRepository + DocumentRepository + ProjectRepository + SourceRepository,
{
    let workspace_id = create_workspace(repository, label).await;
    let project = repository
        .ensure_default_project(workspace_id)
        .await
        .expect("ensure workspace project");
    let source = Source {
        id: SourceId(Uuid::now_v7()),
        project_id: project.id,
        name: format!("{label} corpus"),
        kind: SourceKind::FileSet {
            root_hint: format!("{label}-runtime-contract"),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking: ChunkingConfig::default(),
    };
    repository
        .create_source(workspace_id, source.clone())
        .await
        .expect("create workspace source");
    let document = Document {
        id: DocumentId(Uuid::now_v7()),
        source_id: source.id,
        path: format!("{label}/runtime-contract.md"),
        mime_type: Some("text/markdown".to_owned()),
        checksum: format!("{label}-document-checksum"),
        byte_size: 32,
        profile: DocumentProfile::TechnicalDocs,
        extraction_quality: ExtractionQuality::High,
        warnings: Vec::new(),
    };
    let chunk = Chunk {
        id: ChunkId(Uuid::now_v7()),
        source_id: source.id,
        document_id: document.id,
        ordinal: 0,
        text: format!("{label} workspace private evidence"),
        token_count: 4,
        byte_range: ByteRange { start: 0, end: 32 },
        checksum: format!("{label}-chunk-checksum"),
        strategy: ChunkingStrategy::Structured,
        section_title: Some("Runtime isolation".to_owned()),
        split_reason: ChunkSplitReason::DocumentEnd,
        quality_flags: vec![ChunkQualityFlag::GoodEvidenceCandidate],
        is_duplicate: false,
        text_density: 1.0,
        evidence_score_hint: 0.9,
    };
    repository
        .insert_document_with_chunks(workspace_id, document, vec![chunk.clone()])
        .await
        .expect("insert workspace document");

    CorpusFixture {
        workspace_id,
        project_id: project.id,
        chunk,
    }
}

async fn create_workspace<R>(repository: &R, label: &str) -> WorkspaceId
where
    R: AuthRepository,
{
    let now = OffsetDateTime::now_utc();
    let marker = Uuid::now_v7();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: format!("{label} runtime organization {marker}"),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: format!("{label} runtime workspace {marker}"),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: format!("{label}-runtime-{marker}@example.test"),
        name: format!("{label} runtime user"),
        created_at: now,
    };
    repository
        .create_user_workspace(
            organization,
            workspace.clone(),
            user,
            WorkspaceRole::Owner,
            "unused-test-password-hash".to_owned(),
        )
        .await
        .expect("create runtime contract workspace");
    workspace.id
}

fn embedding(chunk: &Chunk, model: &EmbeddingModelInfo) -> ChunkEmbedding {
    ChunkEmbedding {
        chunk_id: chunk.id,
        chunk_checksum: chunk.checksum.clone(),
        model: model.clone(),
        vector: vec![0.0; model.dimension as usize],
        indexed_at: OffsetDateTime::now_utc(),
    }
}

fn retrieval_response(query: &str) -> RetrievalQueryResponse {
    RetrievalQueryResponse {
        run: RetrievalQueryRun {
            id: RetrievalQueryRunId(Uuid::now_v7()),
            query: query.to_owned(),
            top_k: 5,
            retrieval_mode: RetrievalMode::Lexical,
            latency_ms: 1,
            created_at: OffsetDateTime::now_utc(),
        },
        answer: ExtractiveAnswer {
            status: ExtractiveAnswerStatus::InsufficientEvidence,
            text: "Insufficient evidence.".to_owned(),
            citations: Vec::new(),
        },
        hits: Vec::new(),
        embedding_status: RetrievalEmbeddingStatus {
            readiness: RetrievalEmbeddingReadiness::NotRequired,
            required: false,
            model: EmbeddingModelInfo::default(),
            total_chunks: 1,
            indexed_chunks: 0,
            missing_chunks: 1,
            stale_chunks: 0,
        },
        diagnosis: None,
    }
}

fn trace(project_id: ProjectId, response: &RetrievalQueryResponse) -> Trace {
    Trace {
        id: TraceId(Uuid::now_v7()),
        project_id,
        input: response.run.query.clone(),
        output: None,
        started_at: response.run.created_at,
        completed_at: Some(OffsetDateTime::now_utc()),
        retrieval_runs: Vec::new(),
        generation: None,
        failure_labels: Vec::new(),
        source_run_id: Some(response.run.id),
        summary: "Runtime workspace isolation trace.".to_owned(),
        status: TraceStatus::Completed,
        evidence_strength: Some(EvidenceStrength::Weak),
        spans: Vec::new(),
        retrieval: Some(response.clone()),
        reruns: Vec::new(),
        diagnosis: None,
        ingestion: None,
    }
}
