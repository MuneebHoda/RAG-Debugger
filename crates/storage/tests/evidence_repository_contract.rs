use rag_debugger_core::{
    ByteRange, Chunk, ChunkId, ChunkQualityFlag, ChunkSplitReason, ChunkingConfig,
    ChunkingStrategy, Document, DocumentId, DocumentProfile, EvalLabEvidenceSearchRequest,
    ExtractionQuality, Source, SourceId, SourceKind, SourceSyncPolicy,
    EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{DocumentRepository, EvidenceRepository, ProjectRepository, SourceRepository},
};
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
    R: DocumentRepository + EvidenceRepository + ProjectRepository + SourceRepository,
{
    let project = repository
        .ensure_default_project()
        .await
        .expect("ensure project");
    let marker = Uuid::now_v7().simple().to_string();
    let source = source(project.id, &marker);
    repository
        .create_source(source.clone())
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
        .insert_document_with_chunks(alpha_document.clone(), vec![alpha_chunk.clone()])
        .await
        .expect("insert alpha document");
    repository
        .insert_document_with_chunks(zeta_document.clone(), vec![zeta_chunk.clone()])
        .await
        .expect("insert zeta document");

    let resolved_documents = repository
        .resolve_evidence_documents(&[
            zeta_document.id,
            alpha_document.id,
            zeta_document.id,
            DocumentId(Uuid::now_v7()),
        ])
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
        .resolve_evidence_chunks(&[zeta_chunk.id, alpha_chunk.id, zeta_chunk.id])
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
        .search_evidence(&search_request(None, 1, 1))
        .await
        .expect("browse evidence");
    let repeated_browse = repository
        .search_evidence(&search_request(None, 1, 1))
        .await
        .expect("repeat evidence browse");
    assert_eq!(browse.documents.len(), 1);
    assert_eq!(browse.chunks.len(), 1);
    assert_eq!(browse, repeated_browse);

    assert_search_finds_document(repository, &alpha_path, alpha_document.id).await;
    assert_search_finds_document(repository, &source.name, alpha_document.id).await;
    assert_search_finds_document(
        repository,
        &zeta_document.id.0.to_string(),
        zeta_document.id,
    )
    .await;
    assert_search_finds_chunk(
        repository,
        &format!("Embedding Workers {marker}"),
        zeta_chunk.id,
    )
    .await;
    assert_search_finds_chunk(
        repository,
        &format!("GPU indexing pipeline {marker}"),
        zeta_chunk.id,
    )
    .await;
    assert_search_finds_chunk(repository, &alpha_chunk.id.0.to_string(), alpha_chunk.id).await;

    let excluded = repository
        .search_evidence(&EvalLabEvidenceSearchRequest {
            query: Some("gpu".to_owned()),
            excluded_document_ids: vec![zeta_document.id],
            excluded_chunk_ids: vec![zeta_chunk.id],
            document_limit: 2,
            chunk_limit: 2,
        })
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
}

async fn assert_search_finds_document<R>(repository: &R, query: &str, id: DocumentId)
where
    R: EvidenceRepository,
{
    let result = repository
        .search_evidence(&search_request(Some(query), 10, 0))
        .await
        .expect("search documents");
    assert_eq!(result.documents[0].id, id);
}

async fn assert_search_finds_chunk<R>(repository: &R, query: &str, id: ChunkId)
where
    R: EvidenceRepository,
{
    let result = repository
        .search_evidence(&search_request(Some(query), 0, 10))
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
        query: query.map(str::to_owned),
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
