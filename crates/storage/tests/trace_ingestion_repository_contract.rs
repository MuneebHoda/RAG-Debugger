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
    let trace_id = run_contract(&store).await;
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("connect verification pool");
    let persisted: (String, String, OffsetDateTime, i32, i64) = sqlx::query_as(
        "SELECT ingestion_mapper_version, trace_json->'ingestion'->>'mapper_version', \
                created_at, rerun_count, \
                (SELECT COUNT(*) FROM trace_rerun_experiments WHERE trace_id = debug_traces.id) \
         FROM debug_traces WHERE id = $1",
    )
    .bind(trace_id.0)
    .fetch_one(&pool)
    .await
    .expect("read persisted mapper versions");
    assert_eq!(persisted.0, "2");
    assert_eq!(persisted.1, "2");
    assert_eq!(
        persisted.2,
        OffsetDateTime::from_unix_timestamp(1_699_996_400).expect("valid merged timestamp")
    );
    assert_eq!(persisted.3, 1);
    assert_eq!(persisted.4, 1);
}

async fn run_contract<R>(repository: &R) -> TraceId
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
    let first_started_at =
        OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid first timestamp");
    first.started_at = first_started_at;
    first.completed_at = Some(first_started_at);
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
    let second_started_at = first_started_at - time::Duration::hours(1);
    let mut second = second;
    second.started_at = second_started_at;
    second.completed_at = Some(first_started_at + time::Duration::seconds(1));
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
    assert_eq!(updated.trace.started_at, second_started_at);

    let mut mapper_advance = imported_trace(project.id, "external-1", "span-2");
    let mapper_metadata = mapper_advance.ingestion.as_mut().expect("mapper metadata");
    mapper_metadata.mapper_version = "2".to_owned();
    mapper_metadata.spans[0].operation = ImportedSpanOperation::Generation;
    mapper_metadata.spans[0].name = "Updated generation".to_owned();
    let advanced = repository
        .upsert_imported_trace(first_workspace, mapper_advance.clone())
        .await
        .expect("advance mapper version");
    assert_eq!(advanced.disposition, TraceIngestionDisposition::Updated);
    let advanced_metadata = advanced
        .trace
        .ingestion
        .as_ref()
        .expect("advanced metadata");
    assert_eq!(advanced_metadata.mapper_version, "2");
    assert_eq!(advanced_metadata.spans.len(), 2);
    assert_eq!(
        advanced_metadata
            .spans
            .iter()
            .find(|span| span.external_span_id == "span-2")
            .expect("replaced span")
            .operation,
        ImportedSpanOperation::Generation
    );
    let mut with_rerun = advanced.trace.clone();
    with_rerun.reruns.push(rerun_comparison());
    repository
        .save_trace(first_workspace, with_rerun)
        .await
        .expect("save imported trace rerun history");
    let unchanged = repository
        .upsert_imported_trace(first_workspace, mapper_advance)
        .await
        .expect("idempotent mapper retry");
    assert_eq!(unchanged.disposition, TraceIngestionDisposition::Unchanged);
    assert_eq!(unchanged.trace.reruns.len(), 1);
    let summaries = repository
        .list_traces(first_workspace)
        .await
        .expect("list imported traces");
    assert_eq!(summaries[0].span_count, 2);
    assert_eq!(summaries[0].rerun_count, 1);
    assert_eq!(summaries[0].created_at, second_started_at);
    assert_eq!(
        summaries[0].ingestion_source,
        Some(TraceIngestionSource::Native)
    );
    assert_eq!(
        summaries[0].external_trace_id.as_deref(),
        Some("external-1")
    );
    assert_eq!(
        summaries[0].mapping_status,
        Some(TraceMappingStatus::PartiallyMapped)
    );
    let mut failed_evaluation = imported_trace(project.id, "external-1", "span-2");
    failed_evaluation
        .ingestion
        .as_mut()
        .expect("failed evaluation metadata")
        .evaluation_passed = Some(false);
    failed_evaluation.status = TraceStatus::Failed;
    let failed = repository
        .upsert_imported_trace(first_workspace, failed_evaluation)
        .await
        .expect("failed evaluation raises aggregate status");
    assert_eq!(failed.trace.status, TraceStatus::Failed);

    let harmless_retry = repository
        .upsert_imported_trace(
            first_workspace,
            imported_trace(project.id, "external-1", "span-2"),
        )
        .await
        .expect("incomplete retry cannot lower aggregate status");
    assert_eq!(harmless_retry.trace.status, TraceStatus::Failed);
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

    let mut same_workspace_collision =
        imported_trace(project.id, "external-collision", "span-collision");
    same_workspace_collision.id = created.trace.id;
    assert!(matches!(
        repository
            .upsert_imported_trace(first_workspace, same_workspace_collision)
            .await,
        Err(StorageError::Conflict(_))
    ));
    assert!(matches!(
        repository
            .get_trace_detail(second_workspace, created.trace.id)
            .await,
        Err(StorageError::NotFound)
    ));

    let second_project = repository
        .ensure_default_project(second_workspace)
        .await
        .expect("second project");
    let mut colliding_trace = imported_trace(second_project.id, "external-2", "span-collision");
    colliding_trace.id = created.trace.id;
    assert!(matches!(
        repository
            .upsert_imported_trace(second_workspace, colliding_trace)
            .await,
        Err(StorageError::NotFound)
    ));
    let preserved = repository
        .get_trace_detail(first_workspace, created.trace.id)
        .await
        .expect("original trace remains owned");
    assert_eq!(
        preserved
            .ingestion
            .as_ref()
            .expect("preserved metadata")
            .external_trace_id,
        "external-1"
    );

    let mut ranked = imported_trace(project.id, "external-1", "span-2");
    ranked.ingestion.as_mut().expect("ranked metadata").evidence = vec![evidence("chunk-1", 1)];
    repository
        .upsert_imported_trace(first_workspace, ranked.clone())
        .await
        .expect("add ranked evidence");
    repository
        .upsert_imported_trace(first_workspace, ranked)
        .await
        .expect("identical ranked evidence retry");
    let mut rank_conflict = imported_trace(project.id, "external-1", "span-2");
    rank_conflict
        .ingestion
        .as_mut()
        .expect("conflicting metadata")
        .evidence = vec![evidence("chunk-2", 1)];
    assert!(matches!(
        repository
            .upsert_imported_trace(first_workspace, rank_conflict)
            .await,
        Err(StorageError::TraceMerge(
            TraceMergeError::DuplicateEvidenceRank
        ))
    ));
    created.trace.id
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
                kind: ImportedSpanKind::Internal,
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

fn rerun_comparison() -> TraceRerunComparison {
    let created_at =
        OffsetDateTime::from_unix_timestamp(1_700_000_100).expect("valid rerun timestamp");
    let response = RetrievalQueryResponse {
        run: RetrievalQueryRun {
            id: RetrievalQueryRunId(Uuid::now_v7()),
            query: "rerun".to_owned(),
            top_k: 1,
            retrieval_mode: RetrievalMode::Hybrid,
            latency_ms: 1,
            created_at,
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
            total_chunks: 0,
            indexed_chunks: 0,
            missing_chunks: 0,
            stale_chunks: 0,
        },
        diagnosis: None,
    };
    TraceRerunComparison {
        id: TraceRerunId(Uuid::now_v7()),
        request: RetrievalQueryRequest {
            query: response.run.query.clone(),
            top_k: 1,
            retrieval_mode: RetrievalMode::Hybrid,
            source_ids: Vec::new(),
            document_ids: Vec::new(),
        },
        response,
        score_delta: 0.0,
        latency_delta_ms: 0,
        overlap_count: 0,
        changed_rank_count: 0,
        diagnosis: None,
        created_at,
    }
}

fn evidence(external_chunk_id: &str, rank: u32) -> ImportedEvidence {
    ImportedEvidence {
        external_chunk_id: external_chunk_id.to_owned(),
        document_label: None,
        rank,
        score: 0.5,
        lexical_score: None,
        semantic_score: None,
        citation_label: None,
        snippet: None,
        answer_support_status: AnswerSupportStatus::Unassessed,
        answer_support_reason: AnswerSupportReason::Unassessed,
    }
}
