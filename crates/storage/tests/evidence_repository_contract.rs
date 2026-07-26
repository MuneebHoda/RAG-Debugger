use rag_debugger_core::{
    ByteRange, Chunk, ChunkId, ChunkQualityFlag, ChunkSplitReason, ChunkingConfig,
    ChunkingStrategy, Document, DocumentId, DocumentProfile, EvalLabEvidenceSearchQuery,
    EvalLabEvidenceSearchRequest, ExtractionQuality, Organization, OrganizationId, RetrievalMode,
    RetrievalQueryRequest, Source, SourceId, SourceKind, SourceSyncPolicy, User, UserId, Workspace,
    WorkspaceId, WorkspaceRole, EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{
        AuthRepository, DocumentRepository, EvidenceRepository, ProjectRepository,
        RetrievalRepository, SourceRepository,
    },
};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn memory_evidence_repository_is_deterministic_and_bounded() {
    run_evidence_repository_contract(&MemoryStore::default()).await;
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_evidence_repository_is_deterministic_and_bounded() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    run_evidence_repository_contract(&store).await;
}

async fn run_evidence_repository_contract<R>(repository: &R)
where
    R: AuthRepository
        + DocumentRepository
        + EvidenceRepository
        + ProjectRepository
        + RetrievalRepository
        + SourceRepository,
{
    let marker = Uuid::now_v7().simple().to_string();
    let workspace_id = create_workspace(repository, &marker).await;
    let project = repository
        .ensure_default_project(workspace_id)
        .await
        .expect("ensure project");
    let source = source(project.id, &marker);
    repository
        .create_source(workspace_id, source.clone())
        .await
        .expect("create source");

    let alpha_path = format!("alpha/{marker}-account-recovery.md");
    let zeta_path = format!("zeta/{marker}-gpu-platform.md");
    let alpha_document = document(source.id, &alpha_path);
    let zeta_document = document(source.id, &zeta_path);
    let alpha_chunk = chunk(
        source.id,
        alpha_document.id,
        0,
        &format!("Recovery links expire after fifteen minutes. {marker}"),
        &format!("Account recovery {marker}"),
    );
    let long_text = format!(
        "GPU indexing pipeline {marker} {}",
        "é".repeat(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT + 20)
    );
    let zeta_chunk = chunk(
        source.id,
        zeta_document.id,
        0,
        &long_text,
        &format!("Embedding Workers {marker}"),
    );
    repository
        .insert_document_with_chunks(
            workspace_id,
            alpha_document.clone(),
            vec![alpha_chunk.clone()],
        )
        .await
        .expect("insert alpha document");
    repository
        .insert_document_with_chunks(
            workspace_id,
            zeta_document.clone(),
            vec![zeta_chunk.clone()],
        )
        .await
        .expect("insert zeta document");

    let resolved_documents = repository
        .resolve_evidence_documents(
            workspace_id,
            &[
                zeta_document.id,
                alpha_document.id,
                zeta_document.id,
                DocumentId(Uuid::now_v7()),
            ],
        )
        .await
        .expect("resolve documents");
    assert_eq!(
        resolved_documents
            .iter()
            .map(|document| document.id)
            .collect::<Vec<_>>(),
        vec![zeta_document.id, alpha_document.id]
    );

    let resolved_chunks = repository
        .resolve_evidence_chunks(
            workspace_id,
            &[zeta_chunk.id, alpha_chunk.id, zeta_chunk.id],
        )
        .await
        .expect("resolve chunks");
    assert_eq!(
        resolved_chunks
            .iter()
            .map(|chunk| chunk.id)
            .collect::<Vec<_>>(),
        vec![zeta_chunk.id, alpha_chunk.id]
    );
    assert_eq!(resolved_chunks[0].document_id, zeta_document.id);
    assert_eq!(resolved_chunks[0].source_name, source.name);
    assert_eq!(
        resolved_chunks[0].text_preview.chars().count(),
        EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT
    );
    assert!(resolved_chunks[0].preview_truncated);

    let browse = repository
        .search_evidence(workspace_id, &search_request(None, 1, 1))
        .await
        .expect("browse evidence");
    let repeated_browse = repository
        .search_evidence(workspace_id, &search_request(None, 1, 1))
        .await
        .expect("repeat evidence browse");
    assert_eq!(browse.documents.len(), 1);
    assert_eq!(browse.chunks.len(), 1);
    assert_eq!(browse, repeated_browse);

    assert_search_finds_document(repository, workspace_id, &alpha_path, alpha_document.id).await;
    assert_search_finds_document(repository, workspace_id, &source.name, alpha_document.id).await;
    assert_search_finds_document(
        repository,
        workspace_id,
        &zeta_document.id.0.to_string(),
        zeta_document.id,
    )
    .await;
    assert_search_finds_chunk(
        repository,
        workspace_id,
        &format!("Embedding Workers {marker}"),
        zeta_chunk.id,
    )
    .await;
    assert_search_finds_chunk(
        repository,
        workspace_id,
        &format!("GPU indexing pipeline {marker}"),
        zeta_chunk.id,
    )
    .await;
    assert_search_finds_chunk(
        repository,
        workspace_id,
        &alpha_chunk.id.0.to_string(),
        alpha_chunk.id,
    )
    .await;

    let excluded = repository
        .search_evidence(
            workspace_id,
            &EvalLabEvidenceSearchRequest {
                query: EvalLabEvidenceSearchQuery::Text("gpu".to_owned()),
                excluded_document_ids: vec![zeta_document.id],
                excluded_chunk_ids: vec![zeta_chunk.id],
                document_limit: 2,
                chunk_limit: 2,
            },
        )
        .await
        .expect("search with exclusions");
    assert!(excluded
        .documents
        .iter()
        .all(|document| document.id != zeta_document.id));
    assert!(excluded
        .chunks
        .iter()
        .all(|chunk| chunk.id != zeta_chunk.id));

    assert_workspace_isolation(repository, workspace_id, &marker).await;
}

async fn assert_workspace_isolation<R>(
    repository: &R,
    visible_workspace_id: WorkspaceId,
    marker: &str,
) where
    R: AuthRepository
        + DocumentRepository
        + EvidenceRepository
        + ProjectRepository
        + RetrievalRepository
        + SourceRepository,
{
    let private_marker = format!("private-{marker}");
    let private_workspace_id = create_workspace(repository, &private_marker).await;
    let private_project = repository
        .ensure_default_project(private_workspace_id)
        .await
        .expect("ensure private project");
    let private_source = source(private_project.id, &private_marker);
    repository
        .create_source(private_workspace_id, private_source.clone())
        .await
        .expect("create private source");
    let private_document = document(
        private_source.id,
        &format!("private/{private_marker}-retention.md"),
    );
    let private_chunk = chunk(
        private_source.id,
        private_document.id,
        0,
        &format!("Private workspace retention marker {private_marker}"),
        &format!("Private section {private_marker}"),
    );
    repository
        .insert_document_with_chunks(
            private_workspace_id,
            private_document.clone(),
            vec![private_chunk.clone()],
        )
        .await
        .expect("insert private evidence");

    assert!(repository
        .resolve_evidence_documents(visible_workspace_id, &[private_document.id])
        .await
        .expect("resolve inaccessible document")
        .is_empty());
    assert!(repository
        .resolve_evidence_chunks(visible_workspace_id, &[private_chunk.id])
        .await
        .expect("resolve inaccessible chunk")
        .is_empty());
    let inaccessible_search = repository
        .search_evidence(
            visible_workspace_id,
            &search_request(Some(&private_marker), 10, 10),
        )
        .await
        .expect("search inaccessible evidence");
    assert!(inaccessible_search.documents.is_empty());
    assert!(inaccessible_search.chunks.is_empty());
    assert!(repository
        .list_sources(visible_workspace_id)
        .await
        .expect("list visible sources")
        .iter()
        .all(|summary| summary.source.id != private_source.id));
    assert!(repository
        .list_document_chunks(visible_workspace_id, private_document.id)
        .await
        .is_err());
    let candidates = repository
        .list_searchable_chunks(
            visible_workspace_id,
            &RetrievalQueryRequest {
                query: private_marker.clone(),
                top_k: 10,
                retrieval_mode: RetrievalMode::Lexical,
                source_ids: Vec::new(),
                document_ids: Vec::new(),
            },
        )
        .await
        .expect("load visible retrieval candidates");
    assert!(candidates
        .iter()
        .all(|candidate| candidate.chunk.id != private_chunk.id));

    assert_eq!(
        repository
            .resolve_evidence_documents(private_workspace_id, &[private_document.id])
            .await
            .expect("resolve owned private document")[0]
            .id,
        private_document.id
    );
}

async fn create_workspace<R>(repository: &R, marker: &str) -> WorkspaceId
where
    R: AuthRepository,
{
    let now = OffsetDateTime::now_utc();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: format!("Evidence contract {marker}"),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: format!("Evidence contract {marker}"),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: format!("evidence-{marker}@example.test"),
        name: "Evidence contract".to_owned(),
        created_at: now,
    };
    repository
        .create_user_workspace(
            organization,
            workspace.clone(),
            user,
            WorkspaceRole::Owner,
            "test-password-hash".to_owned(),
        )
        .await
        .expect("create evidence contract workspace");
    workspace.id
}

async fn assert_search_finds_document<R>(
    repository: &R,
    workspace_id: WorkspaceId,
    query: &str,
    id: DocumentId,
) where
    R: EvidenceRepository,
{
    let result = repository
        .search_evidence(workspace_id, &search_request(Some(query), 10, 0))
        .await
        .expect("search documents");
    assert_eq!(result.documents[0].id, id);
}

async fn assert_search_finds_chunk<R>(
    repository: &R,
    workspace_id: WorkspaceId,
    query: &str,
    id: ChunkId,
) where
    R: EvidenceRepository,
{
    let result = repository
        .search_evidence(workspace_id, &search_request(Some(query), 0, 10))
        .await
        .expect("search chunks");
    assert_eq!(result.chunks[0].id, id);
}

fn search_request(
    query: Option<&str>,
    document_limit: u32,
    chunk_limit: u32,
) -> EvalLabEvidenceSearchRequest {
    EvalLabEvidenceSearchRequest {
        query: match query {
            None => EvalLabEvidenceSearchQuery::Browse,
            Some(query) => uuid::Uuid::parse_str(query).map_or_else(
                |_| EvalLabEvidenceSearchQuery::Text(query.to_lowercase()),
                EvalLabEvidenceSearchQuery::ExactId,
            ),
        },
        excluded_document_ids: Vec::new(),
        excluded_chunk_ids: Vec::new(),
        document_limit,
        chunk_limit,
    }
}

fn source(project_id: rag_debugger_core::ProjectId, marker: &str) -> Source {
    Source {
        id: SourceId(Uuid::now_v7()),
        project_id,
        name: format!("Storage contract corpus {marker}"),
        kind: SourceKind::FileSet {
            root_hint: "evidence-contract".to_owned(),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking: ChunkingConfig::default(),
    }
}

fn document(source_id: SourceId, path: &str) -> Document {
    Document {
        id: DocumentId(Uuid::now_v7()),
        source_id,
        path: path.to_owned(),
        mime_type: Some("text/markdown".to_owned()),
        checksum: format!("document-{path}"),
        byte_size: 512,
        profile: DocumentProfile::TechnicalDocs,
        extraction_quality: ExtractionQuality::High,
        warnings: Vec::new(),
    }
}

fn chunk(
    source_id: SourceId,
    document_id: DocumentId,
    ordinal: u32,
    text: &str,
    section: &str,
) -> Chunk {
    Chunk {
        id: ChunkId(Uuid::now_v7()),
        source_id,
        document_id,
        ordinal,
        text: text.to_owned(),
        token_count: text.split_whitespace().count() as u32,
        byte_range: ByteRange {
            start: 0,
            end: text.len() as u64,
        },
        checksum: format!("chunk-{document_id:?}-{ordinal}"),
        strategy: ChunkingStrategy::Structured,
        section_title: Some(section.to_owned()),
        split_reason: ChunkSplitReason::DocumentEnd,
        quality_flags: vec![ChunkQualityFlag::GoodEvidenceCandidate],
        is_duplicate: false,
        text_density: 1.0,
        evidence_score_hint: 0.9,
    }
}
