use rag_debugger_core::*;
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{AuthRepository, ProjectRepository, TraceRepository},
    StorageError,
};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn memory_trace_ingestion_repository_contract() {
    run_contract(&MemoryStore::default()).await;
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_trace_ingestion_repository_contract() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    run_contract(&store).await;
}

async fn run_contract<R>(repository: &R)
where
    R: AuthRepository + ProjectRepository + TraceRepository,
{
    let first_workspace = workspace(repository, "first").await;
    let second_workspace = workspace(repository, "second").await;
    let project = repository
        .ensure_default_project(first_workspace)
        .await
        .expect("first project");
    let mut first = imported_trace(project.id, "external-1", "span-1");
    let first_metadata = first.ingestion.as_mut().expect("first metadata");
    first_metadata.spans[0].parent_span_id = Some("span-2".to_owned());
    first_metadata
        .limitations
        .push("orphan_parent_span".to_owned());
    first_metadata.limitations.sort();
    let created = repository
        .upsert_imported_trace(first_workspace, first.clone())
        .await
        .expect("create imported trace");
    assert_eq!(created.disposition, TraceIngestionDisposition::Created);
    let unchanged = repository
        .upsert_imported_trace(first_workspace, first.clone())
        .await
        .expect("idempotent retry");
    assert_eq!(unchanged.disposition, TraceIngestionDisposition::Unchanged);

    let second = imported_trace(project.id, "external-1", "span-2");
    let updated = repository
        .upsert_imported_trace(first_workspace, second)
        .await
        .expect("merge second span");
    assert_eq!(updated.disposition, TraceIngestionDisposition::Updated);
    let metadata = updated.trace.ingestion.expect("metadata");
    assert_eq!(metadata.spans.len(), 2);
    assert!(!metadata
        .limitations
        .iter()
        .any(|value| value == "orphan_parent_span"));
    let summaries = repository
        .list_traces(first_workspace)
        .await
        .expect("list imported traces");
    assert_eq!(summaries[0].span_count, 2);
    let mut privacy_conflict = first.clone();
    privacy_conflict
        .ingestion
        .as_mut()
        .expect("conflict metadata")
        .privacy_mode = TraceIngestionPrivacyMode::FullLocalOnly;
    assert!(matches!(
        repository
            .upsert_imported_trace(first_workspace, privacy_conflict)
            .await,
        Err(StorageError::Conflict(_))
    ));
    assert!(matches!(
        repository
            .upsert_imported_trace(second_workspace, first.clone())
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repository
            .get_trace_detail(second_workspace, created.trace.id)
            .await,
        Err(StorageError::NotFound)
    ));
}

async fn workspace<R: AuthRepository>(repository: &R, label: &str) -> WorkspaceId {
    let now = OffsetDateTime::now_utc();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: format!("{label} trace ingestion org"),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: format!("{label} trace ingestion workspace"),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: format!("{label}-{}@example.test", Uuid::now_v7()),
        name: label.to_owned(),
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
        .expect("create workspace");
    workspace.id
}

fn imported_trace(project_id: ProjectId, external_trace_id: &str, span_id: &str) -> Trace {
    let now = OffsetDateTime::now_utc();
    Trace {
        id: TraceId(Uuid::now_v7()),
        project_id,
        input: String::new(),
        output: None,
        started_at: now,
        completed_at: Some(now),
        retrieval_runs: Vec::new(),
        generation: None,
        failure_labels: Vec::new(),
        source_run_id: None,
        summary: "Imported metadata trace.".to_owned(),
        status: TraceStatus::Warning,
        evidence_strength: Some(EvidenceStrength::Weak),
        spans: Vec::new(),
        retrieval: None,
        reruns: Vec::new(),
        diagnosis: None,
        ingestion: Some(TraceIngestionMetadata {
            source: TraceIngestionSource::Native,
            external_trace_id: external_trace_id.to_owned(),
            schema_version: "1".to_owned(),
            mapper_version: "1".to_owned(),
            mapping_status: TraceMappingStatus::PartiallyMapped,
            privacy_mode: TraceIngestionPrivacyMode::MetadataOnly,
            service_name: None,
            service_version: None,
            deployment_environment: None,
            instrumentation_scope_name: None,
            instrumentation_scope_version: None,
            known_failure_labels: Vec::new(),
            status_supplied: false,
            limitations: vec![
                "evidence_content_not_retained".to_owned(),
                "query_not_retained".to_owned(),
                "retrieval_evidence_missing".to_owned(),
            ],
            prompt: None,
            retrieval_mode: Some(RetrievalMode::Hybrid),
            top_k: Some(1),
            model_config: None,
            evidence: Vec::new(),
            spans: vec![ImportedSpan {
                external_span_id: span_id.to_owned(),
                parent_span_id: None,
                operation: ImportedSpanOperation::Retrieval,
                name: "Retrieval".to_owned(),
                started_at: now,
                completed_at: Some(now),
                latency_ms: 1,
                status: ImportedSpanStatus::Succeeded,
                provider: None,
                model: None,
                input_tokens: None,
                output_tokens: None,
                error_type: None,
            }],
            evaluation_passed: None,
            evaluation_label: None,
            timestamps_supplied: true,
        }),
    }
}
