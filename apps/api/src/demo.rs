use rag_debugger_core::{
    ChunkId, ChunkingConfig, ChunkingStrategy, DebugReportSource, DemoLoadResponse, DemoProgress,
    DemoQueryId, DemoStatus, DemoSuggestedQuery, DocumentId, EmbeddingIndexRequest,
    EmbeddingModelInfo, PrivacyMode, Project, ProjectId, Source, SourceId, SourceKind,
    SourceSyncPolicy, WorkspaceId,
};
use rag_debugger_storage::repository::AppRepository;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::ApiError,
    ingestion::{prepare_document, IngestionFile},
};

pub(crate) const DEMO_VERSION: &str = "corpuslab-guided-demo-v1";
const DEMO_PROJECT_NAME: &str = "CorpusLab Guided Demo";
const DEMO_SOURCE_NAME: &str = "CorpusLab Sample Corpus";
const EXPECTED_DOCUMENT_COUNT: usize = 3;

struct DemoFixture {
    path: &'static str,
    contents: &'static str,
}

const FIXTURES: [DemoFixture; EXPECTED_DOCUMENT_COUNT] = [
    DemoFixture {
        path: "policy_docs/data-retention.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/corpora/policy_docs/data-retention.md"
        )),
    },
    DemoFixture {
        path: "support_kb/account-recovery.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/corpora/support_kb/account-recovery.md"
        )),
    },
    DemoFixture {
        path: "technical_docs/gpu-indexing.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/corpora/technical_docs/gpu-indexing.md"
        )),
    },
];

pub(crate) async fn load_demo(
    repository: &dyn AppRepository,
    workspace_id: WorkspaceId,
    model: &EmbeddingModelInfo,
) -> Result<DemoLoadResponse, ApiError> {
    let now = OffsetDateTime::now_utc();
    let project = Project {
        id: ProjectId(stable_uuid(&format!(
            "{}:{DEMO_VERSION}:project",
            workspace_id.0
        ))),
        name: DEMO_PROJECT_NAME.to_owned(),
        privacy_mode: PrivacyMode::LocalOnly,
        created_at: now,
        updated_at: now,
    };
    let project = repository
        .ensure_demo_project(workspace_id, project)
        .await?;
    let source = Source {
        id: SourceId(stable_uuid(&format!(
            "{}:{DEMO_VERSION}:source",
            project.id.0
        ))),
        project_id: project.id,
        name: DEMO_SOURCE_NAME.to_owned(),
        kind: SourceKind::FileSet {
            root_hint: DEMO_VERSION.to_owned(),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking: demo_chunking(),
    };
    let source = repository.ensure_demo_source(source).await?;

    let mut created_documents = 0;
    for fixture in FIXTURES {
        let document_id = DocumentId(stable_uuid(&format!(
            "{}:{}:{}",
            source.id.0, DEMO_VERSION, fixture.path
        )));
        let mut prepared = prepare_document(
            source.id,
            document_id,
            demo_chunking(),
            IngestionFile {
                file_name: fixture.path,
                content_type: Some("text/markdown"),
                bytes: fixture.contents.as_bytes(),
            },
        )
        .map_err(|error| ApiError::BadRequest(format!("{}: {}", error.code, error.message)))?;
        for chunk in &mut prepared.chunks {
            chunk.id = ChunkId(stable_uuid(&format!(
                "{}:{}:{}:{}",
                document_id.0, chunk.ordinal, chunk.checksum, DEMO_VERSION
            )));
        }
        if repository
            .upsert_demo_document_with_chunks(prepared.document, prepared.chunks)
            .await?
        {
            created_documents += 1;
        }
    }

    Ok(DemoLoadResponse {
        created_documents,
        status: demo_status(repository, workspace_id, model).await?,
    })
}

pub(crate) async fn demo_status(
    repository: &dyn AppRepository,
    workspace_id: WorkspaceId,
    model: &EmbeddingModelInfo,
) -> Result<DemoStatus, ApiError> {
    let source = repository
        .get_demo_source(workspace_id, DEMO_VERSION)
        .await?;
    let Some(source) = source else {
        return Ok(DemoStatus {
            version: DEMO_VERSION.to_owned(),
            project_id: None,
            source_id: None,
            progress: DemoProgress::default(),
            suggested_queries: suggested_queries(),
        });
    };

    let embedding_request = EmbeddingIndexRequest {
        source_ids: vec![source.source.id],
        document_ids: Vec::new(),
    };
    let embeddings = repository
        .embedding_status(&embedding_request, model)
        .await?;
    let retrieval = repository
        .latest_retrieval_query_for_source(source.source.id)
        .await?;
    let trace = repository.latest_trace_for_source(source.source.id).await?;
    let report_id = if let Some(trace) = &trace {
        repository
            .list_debug_reports(workspace_id)
            .await?
            .into_iter()
            .find(|report| {
                matches!(report.source, DebugReportSource::Trace { trace_id } if trace_id == trace.id)
            })
            .map(|report| report.id)
    } else {
        None
    };
    let chunks_created = source.document_count == EXPECTED_DOCUMENT_COUNT as u32
        && source
            .documents
            .iter()
            .all(|document| document.chunk_count > 0);

    Ok(DemoStatus {
        version: DEMO_VERSION.to_owned(),
        project_id: Some(source.source.project_id),
        source_id: Some(source.source.id),
        progress: DemoProgress {
            sample_corpus_loaded: source.document_count == EXPECTED_DOCUMENT_COUNT as u32,
            chunks_created,
            embeddings_indexed: chunks_created
                && embeddings.total_chunks > 0
                && embeddings.missing_chunks == 0
                && embeddings.stale_chunks == 0,
            document_count: source.document_count,
            chunk_count: source.chunk_count,
            indexed_chunk_count: embeddings.indexed_chunks,
            retrieval_run_id: retrieval.map(|response| response.run.id),
            trace_id: trace.map(|trace| trace.id),
            report_id,
        },
        suggested_queries: suggested_queries(),
    })
}

fn demo_chunking() -> ChunkingConfig {
    ChunkingConfig {
        target_tokens: 128,
        overlap_tokens: 16,
        strategy: ChunkingStrategy::Structured,
    }
}

fn suggested_queries() -> Vec<DemoSuggestedQuery> {
    vec![
        DemoSuggestedQuery {
            id: DemoQueryId::AccountRecovery,
            question: "How long does a password reset link remain valid, and what duplicated evidence could confuse the answer?".to_owned(),
            description: "Diagnose duplicate support content while verifying the current recovery rule.".to_owned(),
            recommended: true,
        },
        DemoSuggestedQuery {
            id: DemoQueryId::DataRetention,
            question: "When is deleted workspace content removed, and which historical policy should not override it?".to_owned(),
            description: "Test whether retrieval distinguishes current policy from superseded guidance.".to_owned(),
            recommended: false,
        },
        DemoSuggestedQuery {
            id: DemoQueryId::GpuIndexing,
            question: "What should an operator inspect when GPU indexing stalls despite free device memory?".to_owned(),
            description: "Trace a technical troubleshooting answer across indexing stages.".to_owned(),
            recommended: false,
        },
    ]
}

fn stable_uuid(value: &str) -> Uuid {
    let digest = Sha256::digest(value.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rag_debugger_storage::{memory::MemoryStore, repository::DemoRepository};

    #[test]
    fn stable_ids_are_repeatable_and_versioned() {
        assert_eq!(stable_uuid("workspace:v1"), stable_uuid("workspace:v1"));
        assert_ne!(stable_uuid("workspace:v1"), stable_uuid("workspace:v2"));
    }

    #[test]
    fn fixture_manifest_and_query_ids_are_stable() {
        assert_eq!(FIXTURES.len(), 3);
        assert!(FIXTURES.iter().all(|fixture| !fixture.contents.is_empty()));
        let queries = suggested_queries();
        assert_eq!(queries.len(), 3);
        assert_eq!(queries.iter().filter(|query| query.recommended).count(), 1);
        assert_eq!(queries[0].id, DemoQueryId::AccountRecovery);
    }

    #[tokio::test]
    async fn load_repairs_a_partial_fixture_set_and_then_becomes_idempotent() {
        let repository = MemoryStore::default();
        let workspace_id = WorkspaceId(Uuid::from_u128(42));
        let now = OffsetDateTime::now_utc();
        let project = Project {
            id: ProjectId(stable_uuid(&format!(
                "{}:{DEMO_VERSION}:project",
                workspace_id.0
            ))),
            name: DEMO_PROJECT_NAME.to_owned(),
            privacy_mode: PrivacyMode::LocalOnly,
            created_at: now,
            updated_at: now,
        };
        let project = repository
            .ensure_demo_project(workspace_id, project)
            .await
            .expect("demo project");
        let source = repository
            .ensure_demo_source(Source {
                id: SourceId(stable_uuid(&format!(
                    "{}:{DEMO_VERSION}:source",
                    project.id.0
                ))),
                project_id: project.id,
                name: DEMO_SOURCE_NAME.to_owned(),
                kind: SourceKind::FileSet {
                    root_hint: DEMO_VERSION.to_owned(),
                },
                sync_policy: SourceSyncPolicy::Manual,
                chunking: demo_chunking(),
            })
            .await
            .expect("demo source");
        let fixture = &FIXTURES[0];
        let document_id = DocumentId(stable_uuid(&format!(
            "{}:{}:{}",
            source.id.0, DEMO_VERSION, fixture.path
        )));
        let mut prepared = prepare_document(
            source.id,
            document_id,
            demo_chunking(),
            IngestionFile {
                file_name: fixture.path,
                content_type: Some("text/markdown"),
                bytes: fixture.contents.as_bytes(),
            },
        )
        .expect("prepared fixture");
        for chunk in &mut prepared.chunks {
            chunk.id = ChunkId(stable_uuid(&format!(
                "{}:{}:{}:{}",
                document_id.0, chunk.ordinal, chunk.checksum, DEMO_VERSION
            )));
        }
        repository
            .upsert_demo_document_with_chunks(prepared.document, prepared.chunks)
            .await
            .expect("partial fixture");

        let repaired = load_demo(&repository, workspace_id, &EmbeddingModelInfo::default())
            .await
            .expect("repair load");
        assert_eq!(repaired.created_documents, 2);
        assert_eq!(repaired.status.progress.document_count, 3);
        assert!(repaired.status.progress.chunks_created);

        let repeated = load_demo(&repository, workspace_id, &EmbeddingModelInfo::default())
            .await
            .expect("repeat load");
        assert_eq!(repeated.created_documents, 0);
        assert_eq!(
            repeated.status.progress.chunk_count,
            repaired.status.progress.chunk_count
        );
    }
}
