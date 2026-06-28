use rag_debugger_core::{
    ByteRange, Chunk, ChunkEmbedding, ChunkId, ChunkQualityFlag, ChunkSplitReason, ChunkingConfig,
    ChunkingStrategy, Document, DocumentId, DocumentProfile, EmbeddingIndexRequest,
    EmbeddingModelInfo, ExtractionQuality, Source, SourceId, SourceKind, SourceSyncPolicy,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    repository::{
        DocumentRepository, EmbeddingRepository, HealthRepository, ProjectRepository,
        SourceRepository,
    },
};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn memory_store_honors_ingestion_and_embedding_contracts() {
    let store = MemoryStore::default();
    store.ping().await.expect("memory health check");

    let project = store
        .ensure_default_project()
        .await
        .expect("create default project");
    let same_project = store
        .ensure_default_project()
        .await
        .expect("reuse default project");
    assert_eq!(project.id, same_project.id);

    let source = source(project.id);
    store
        .create_source(source.clone())
        .await
        .expect("create source");

    let document = document(source.id);
    let chunks = vec![
        chunk(source.id, document.id, 1, "second chunk"),
        chunk(source.id, document.id, 0, "first chunk"),
    ];
    store
        .insert_document_with_chunks(document.clone(), chunks)
        .await
        .expect("insert document and chunks");

    let summaries = store.list_sources().await.expect("list sources");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].source, source);
    assert_eq!(summaries[0].documents[0].document, document);
    assert_eq!(summaries[0].chunk_count, 2);

    let stored_chunks = store
        .list_document_chunks(document.id)
        .await
        .expect("list document chunks");
    assert_eq!(
        stored_chunks
            .iter()
            .map(|chunk| chunk.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );

    let index_request = EmbeddingIndexRequest::default();
    let model = EmbeddingModelInfo::default();
    let missing_status = store
        .embedding_status(&index_request, &model)
        .await
        .expect("read missing embedding status");
    assert_eq!(missing_status.total_chunks, 2);
    assert_eq!(missing_status.missing_chunks, 2);

    let candidates = store
        .list_embedding_candidates(&index_request)
        .await
        .expect("list embedding candidates");
    assert_eq!(candidates.len(), 2);

    let indexed_at = OffsetDateTime::now_utc();
    store
        .upsert_chunk_embeddings(
            candidates
                .iter()
                .map(|candidate| ChunkEmbedding {
                    chunk_id: candidate.chunk_id,
                    chunk_checksum: candidate.checksum.clone(),
                    model: model.clone(),
                    vector: vec![0.0; model.dimension as usize],
                    indexed_at,
                })
                .collect(),
        )
        .await
        .expect("upsert embeddings");

    let indexed_status = store
        .embedding_status(&index_request, &model)
        .await
        .expect("read indexed embedding status");
    assert_eq!(indexed_status.indexed_chunks, 2);
    assert_eq!(indexed_status.missing_chunks, 0);
    assert_eq!(indexed_status.stale_chunks, 0);
}

fn source(project_id: rag_debugger_core::ProjectId) -> Source {
    Source {
        id: SourceId(Uuid::now_v7()),
        project_id,
        name: "Storage contract corpus".to_owned(),
        kind: SourceKind::FileSet {
            root_hint: "storage-contract".to_owned(),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking: ChunkingConfig::default(),
    }
}

fn document(source_id: SourceId) -> Document {
    Document {
        id: DocumentId(Uuid::now_v7()),
        source_id,
        path: "contract.md".to_owned(),
        mime_type: Some("text/markdown".to_owned()),
        checksum: "document-checksum".to_owned(),
        byte_size: 24,
        profile: DocumentProfile::TechnicalDocs,
        extraction_quality: ExtractionQuality::High,
        warnings: Vec::new(),
    }
}

fn chunk(source_id: SourceId, document_id: DocumentId, ordinal: u32, text: &str) -> Chunk {
    Chunk {
        id: ChunkId(Uuid::now_v7()),
        source_id,
        document_id,
        ordinal,
        text: text.to_owned(),
        token_count: 2,
        byte_range: ByteRange { start: 0, end: 12 },
        checksum: format!("chunk-{ordinal}"),
        strategy: ChunkingStrategy::Structured,
        section_title: Some("Storage".to_owned()),
        split_reason: ChunkSplitReason::DocumentEnd,
        quality_flags: vec![ChunkQualityFlag::GoodEvidenceCandidate],
        is_duplicate: false,
        text_density: 1.0,
        evidence_score_hint: 0.9,
    }
}
