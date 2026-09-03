use rag_debugger_core::{
    ApiKey, ApiKeyId, ApiKeyRecord, ApiKeyScope, ByteRange, Chunk, ChunkEmbedding, ChunkId,
    ChunkQualityFlag, ChunkSplitReason, ChunkingConfig, ChunkingStrategy, CiEvalReport, CiEvalRun,
    CiEvalRunId, CiEvalRunStatus, Document, DocumentId, DocumentProfile, EmbeddingModelInfo,
    ExtractionQuality, Organization, OrganizationId, ProjectId, RetrievalEvalCase,
    RetrievalEvalCaseId, RetrievalEvalCaseProvenance, RetrievalEvalComparison,
    RetrievalEvalConfigSnapshot, RetrievalEvalDataset, RetrievalEvalDatasetId,
    RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalExperimentProvenance,
    RetrievalEvalGate, RetrievalEvalGateStatus, RetrievalEvalRun, RetrievalEvalRunId,
    RetrievalMode, RetrievalWeights, Source, SourceId, SourceKind, SourceSyncPolicy, Trace,
    TraceId, TraceIngestionMetadata, TraceIngestionPrivacyMode, TraceIngestionSource,
    TraceMappingStatus, TraceStatus, User, UserId, Workspace, WorkspaceId, WorkspaceRole,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{
        AuthRepository, CiEvalRepository, DocumentRepository, EmbeddingRepository, EvalRepository,
        ProjectRepository, RetrievalEvalDatasetImportWrite, SourceRepository,
        SubmittedExpectedEvidence, TraceRepository,
    },
    StorageError,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[tokio::test]
async fn memory_eval_repository_enforces_workspace_ownership() {
    run_eval_workspace_contract(&MemoryStore::default()).await;
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_eval_repository_enforces_workspace_ownership() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    run_eval_workspace_contract(&store).await;
}

#[tokio::test]
async fn memory_ci_baseline_lookup_reaches_past_one_hundred_incompatible_runs() {
    run_ci_baseline_history_contract(&MemoryStore::default()).await;
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_ci_baseline_lookup_reaches_past_one_hundred_incompatible_runs() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    run_ci_baseline_history_contract(&store).await;
}

#[tokio::test]
async fn memory_golden_dataset_import_is_atomic_and_workspace_scoped() {
    run_golden_dataset_import_contract(&MemoryStore::default()).await;
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_golden_dataset_import_is_atomic_and_workspace_scoped() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    run_golden_dataset_import_contract(&store).await;
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_eval_provenance_lock_blocks_concurrent_trace_mutation() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    let workspace_id = create_workspace(&store, "provenance-lock").await;
    let project = store
        .ensure_default_project(workspace_id)
        .await
        .expect("default project");
    let trace = full_local_trace(project.id);
    store
        .upsert_imported_trace(workspace_id, trace.clone())
        .await
        .expect("save source trace");
    let dataset = dataset("Provenance lock dataset");
    store
        .create_retrieval_eval_dataset(workspace_id, dataset.clone())
        .await
        .expect("create provenance lock dataset");
    let mut imported_case = eval_case("Provenance lock case");
    imported_case.query.clone_from(&trace.input);
    imported_case.provenance = Some(RetrievalEvalCaseProvenance {
        source_trace_id: trace.id,
        source: TraceIngestionSource::Native,
        privacy_mode: TraceIngestionPrivacyMode::FullLocalOnly,
    });

    let mut blocker = store.pool().begin().await.expect("blocking transaction");
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM debug_traces WHERE id = $1 FOR UPDATE")
        .bind(trace.id.0)
        .fetch_one(&mut *blocker)
        .await
        .expect("hold source trace update lock");

    let creator = store.clone();
    let create = tokio::spawn(async move {
        creator
            .create_retrieval_eval_case_in_dataset(workspace_id, dataset.id, imported_case)
            .await
    });
    let mut saw_validation_lock = false;
    for _ in 0..100 {
        saw_validation_lock = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1 FROM pg_stat_activity
                 WHERE datname = current_database()
                   AND wait_event_type = 'Lock'
                   AND query LIKE 'SELECT id FROM debug_traces%'
                   AND query LIKE '%FOR SHARE%'
             )",
        )
        .fetch_one(store.pool())
        .await
        .expect("inspect blocked validation query");
        if saw_validation_lock {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        saw_validation_lock,
        "repository creation must validate provenance with a shared row lock"
    );

    let mut changed = trace;
    changed.input = "changed after validation".to_owned();
    let writer = store.clone();
    let update =
        tokio::spawn(async move { writer.upsert_imported_trace(workspace_id, changed).await });
    blocker.commit().await.expect("release source trace lock");
    create
        .await
        .expect("join Eval case creation")
        .expect("create provenance-locked Eval case");
    update
        .await
        .expect("join source trace update")
        .expect("update source trace after validation");
}

#[tokio::test]
#[ignore = "requires a migrated Postgres database"]
async fn postgres_eval_corpus_snapshot_stays_consistent_across_concurrent_mutation() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let store = PostgresStore::connect(&database_url)
        .await
        .expect("connect Postgres store");
    let workspace_id = create_workspace(&store, "eval-snapshot").await;
    let project = store
        .ensure_default_project(workspace_id)
        .await
        .expect("default project");
    let source = Source {
        id: SourceId(Uuid::now_v7()),
        project_id: project.id,
        name: "Snapshot source".to_owned(),
        kind: SourceKind::FileSet {
            root_hint: "snapshot".to_owned(),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking: ChunkingConfig::default(),
    };
    store
        .create_source(workspace_id, source.clone())
        .await
        .expect("create source");
    let document = Document {
        id: DocumentId(Uuid::now_v7()),
        source_id: source.id,
        path: "snapshot.md".to_owned(),
        mime_type: Some("text/markdown".to_owned()),
        checksum: "document-before".to_owned(),
        byte_size: 16,
        profile: DocumentProfile::TechnicalDocs,
        extraction_quality: ExtractionQuality::High,
        warnings: Vec::new(),
    };
    let chunk = Chunk {
        id: ChunkId(Uuid::now_v7()),
        source_id: source.id,
        document_id: document.id,
        ordinal: 0,
        text: "snapshot evidence".to_owned(),
        token_count: 2,
        byte_range: ByteRange { start: 0, end: 16 },
        checksum: "chunk-before".to_owned(),
        strategy: ChunkingStrategy::Structured,
        section_title: None,
        split_reason: ChunkSplitReason::DocumentEnd,
        quality_flags: vec![ChunkQualityFlag::GoodEvidenceCandidate],
        is_duplicate: false,
        text_density: 1.0,
        evidence_score_hint: 1.0,
    };
    store
        .insert_document_with_chunks(workspace_id, document.clone(), vec![chunk.clone()])
        .await
        .expect("insert document and chunk");
    let model = EmbeddingModelInfo::default();
    store
        .upsert_chunk_embeddings(
            workspace_id,
            vec![ChunkEmbedding {
                chunk_id: chunk.id,
                chunk_checksum: chunk.checksum.clone(),
                model: model.clone(),
                vector: vec![0.0; model.dimension as usize],
                indexed_at: OffsetDateTime::now_utc(),
            }],
        )
        .await
        .expect("insert embedding");

    let mut blocker = store.pool().begin().await.expect("blocking transaction");
    sqlx::query("LOCK TABLE chunk_embeddings IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .expect("block snapshot candidate phase");
    let snapshot_store = store.clone();
    let snapshot = tokio::spawn(async move {
        snapshot_store
            .retrieval_eval_corpus_snapshot(workspace_id)
            .await
    });
    let mut candidate_phase_blocked = false;
    for _ in 0..500 {
        candidate_phase_blocked = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1
                 FROM pg_locks held
                 INNER JOIN pg_class relation ON relation.oid = held.relation
                 WHERE relation.relname = 'chunk_embeddings'
                   AND NOT held.granted
             )",
        )
        .fetch_one(store.pool())
        .await
        .expect("inspect blocked snapshot query");
        if candidate_phase_blocked {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(candidate_phase_blocked, "candidate phase must be blocked");

    sqlx::query("UPDATE sources SET target_tokens = target_tokens + 1 WHERE id = $1")
        .bind(source.id.0)
        .execute(store.pool())
        .await
        .expect("mutate source");
    sqlx::query("UPDATE documents SET checksum = 'document-after' WHERE id = $1")
        .bind(document.id.0)
        .execute(store.pool())
        .await
        .expect("mutate document");
    sqlx::query("UPDATE chunks SET checksum = 'chunk-after' WHERE id = $1")
        .bind(chunk.id.0)
        .execute(store.pool())
        .await
        .expect("mutate chunk");
    sqlx::query("UPDATE chunk_embeddings SET vector[1] = 1.0 WHERE chunk_id = $1")
        .bind(chunk.id.0)
        .execute(&mut *blocker)
        .await
        .expect("mutate embedding");
    blocker.commit().await.expect("release candidate phase");

    let snapshot = snapshot
        .await
        .expect("join snapshot")
        .expect("capture snapshot");
    assert_eq!(snapshot.sources[0].source.chunking, source.chunking);
    assert_eq!(
        snapshot.sources[0].documents[0].document.checksum,
        "document-before"
    );
    assert_eq!(snapshot.candidates[0].document.checksum, "document-before");
    assert_eq!(snapshot.candidates[0].chunk.checksum, "chunk-before");
    assert_eq!(
        snapshot.candidates[0].embedding.as_ref().unwrap().vector[0],
        0.0
    );

    let fresh = store
        .retrieval_eval_corpus_snapshot(workspace_id)
        .await
        .expect("capture post-mutation snapshot");
    assert_ne!(fresh.sources[0].source.chunking, source.chunking);
    assert_eq!(fresh.candidates[0].document.checksum, "document-after");
    assert_eq!(fresh.candidates[0].chunk.checksum, "chunk-after");
    assert_eq!(
        fresh.candidates[0].embedding.as_ref().unwrap().vector[0],
        1.0
    );
}

async fn run_eval_workspace_contract<R>(repository: &R)
where
    R: AuthRepository + CiEvalRepository + EvalRepository + ProjectRepository + TraceRepository,
{
    let workspace_a = create_workspace(repository, "alpha").await;
    let workspace_b = create_workspace(repository, "beta").await;
    let dataset_a = dataset("Alpha dataset");
    let dataset_b = dataset("Beta dataset");
    repository
        .create_retrieval_eval_dataset(workspace_a, dataset_a.clone())
        .await
        .expect("create alpha dataset");
    repository
        .create_retrieval_eval_dataset(workspace_b, dataset_b.clone())
        .await
        .expect("create beta dataset");

    let project_a = repository
        .ensure_default_project(workspace_a)
        .await
        .expect("alpha project");
    let project_b = repository
        .ensure_default_project(workspace_b)
        .await
        .expect("beta project");
    let imported_trace = full_local_trace(project_a.id);
    repository
        .upsert_imported_trace(workspace_a, imported_trace.clone())
        .await
        .expect("save full-local source trace");
    let mut imported_case = eval_case("Imported alpha case");
    imported_case.query.clone_from(&imported_trace.input);
    imported_case.provenance = Some(RetrievalEvalCaseProvenance {
        source_trace_id: imported_trace.id,
        source: TraceIngestionSource::Native,
        privacy_mode: TraceIngestionPrivacyMode::FullLocalOnly,
    });
    let saved_imported_case = repository
        .create_retrieval_eval_case_in_dataset(workspace_a, dataset_a.id, imported_case.clone())
        .await
        .expect("save imported eval provenance");
    assert_eq!(saved_imported_case.provenance, imported_case.provenance);
    assert_eq!(
        repository
            .get_retrieval_eval_case(workspace_a, imported_case.id)
            .await
            .expect("read imported eval provenance")
            .provenance,
        imported_case.provenance
    );
    let mut mismatched_query = eval_case("Mismatched imported case");
    mismatched_query.provenance = saved_imported_case.provenance.clone();
    assert!(matches!(
        repository
            .create_retrieval_eval_case_in_dataset(workspace_a, dataset_a.id, mismatched_query)
            .await,
        Err(StorageError::NotFound)
    ));
    let mut cross_workspace_provenance = eval_case("Forged imported case");
    cross_workspace_provenance.provenance = saved_imported_case.provenance;
    assert!(matches!(
        repository
            .create_retrieval_eval_case_in_dataset(
                workspace_b,
                dataset_b.id,
                cross_workspace_provenance,
            )
            .await,
        Err(StorageError::NotFound)
    ));

    let case_a = eval_case("Alpha case");
    let case_b = eval_case("Beta case");
    repository
        .create_retrieval_eval_case_in_dataset(workspace_a, dataset_a.id, case_a.clone())
        .await
        .expect("create alpha case");
    repository
        .create_retrieval_eval_case_in_dataset(workspace_b, dataset_b.id, case_b.clone())
        .await
        .expect("create beta case");

    let cross_dataset = repository
        .get_retrieval_eval_dataset(workspace_a, dataset_b.id)
        .await;
    assert!(
        matches!(cross_dataset, Err(StorageError::NotFound)),
        "cross-workspace dataset lookup returned {cross_dataset:?}"
    );
    assert!(matches!(
        repository
            .get_retrieval_eval_case(workspace_a, case_b.id)
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repository
            .update_retrieval_eval_case(
                workspace_a,
                RetrievalEvalCase {
                    name: "Cross-workspace mutation".to_owned(),
                    ..case_b.clone()
                },
                SubmittedExpectedEvidence::default(),
            )
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repository
            .delete_retrieval_eval_case(workspace_a, case_b.id)
            .await,
        Err(StorageError::NotFound)
    ));

    let mut experiment_b = experiment(dataset_b.id, &dataset_b.name);
    experiment_b.provenance = Some(experiment_provenance(
        workspace_b,
        project_b.id,
        dataset_b.id,
    ));
    repository
        .save_retrieval_eval_experiment(workspace_b, experiment_b.clone())
        .await
        .expect("save beta experiment");
    assert_eq!(
        repository
            .get_retrieval_eval_experiment(workspace_b, experiment_b.id)
            .await
            .expect("read beta experiment provenance")
            .provenance,
        experiment_b.provenance
    );
    assert!(repository
        .latest_retrieval_eval_experiment(workspace_a)
        .await
        .expect("read alpha latest experiment")
        .is_none());
    assert_eq!(
        repository
            .latest_retrieval_eval_experiment(workspace_b)
            .await
            .expect("read beta latest experiment")
            .map(|experiment| experiment.id),
        Some(experiment_b.id)
    );
    assert!(matches!(
        repository
            .save_retrieval_eval_experiment(workspace_b, experiment_b.clone())
            .await,
        Err(StorageError::Conflict(_))
    ));
    let mut forged_experiment = experiment(dataset_b.id, &dataset_b.name);
    forged_experiment.provenance = Some(experiment_provenance(
        workspace_b,
        project_a.id,
        dataset_b.id,
    ));
    assert!(matches!(
        repository
            .save_retrieval_eval_experiment(workspace_b, forged_experiment)
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repository
            .get_retrieval_eval_experiment(workspace_a, experiment_b.id)
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        repository
            .list_retrieval_eval_experiments_for_dataset(workspace_a, dataset_b.id)
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(repository
        .list_retrieval_eval_experiments(workspace_a)
        .await
        .expect("list alpha experiments")
        .iter()
        .all(|experiment| experiment.id != experiment_b.id));

    let ci_run_b = ci_run(workspace_b, &dataset_b, &experiment_b);
    repository
        .save_ci_eval_run(ci_run_b.clone())
        .await
        .expect("save beta CI run");
    assert!(matches!(
        repository.get_ci_eval_run(workspace_a, ci_run_b.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(repository
        .list_ci_eval_runs(workspace_a)
        .await
        .expect("list alpha CI runs")
        .is_empty());
    assert!(repository
        .latest_compatible_ci_eval_run(workspace_a, &ci_run_b.config_label, &experiment_b)
        .await
        .expect("read alpha CI baseline")
        .is_none());
    assert_eq!(
        repository
            .get_ci_eval_run(workspace_b, ci_run_b.id)
            .await
            .expect("read beta CI run")
            .id,
        ci_run_b.id
    );
    assert!(matches!(
        repository
            .save_ci_eval_run(CiEvalRun {
                id: CiEvalRunId(Uuid::now_v7()),
                workspace_id: workspace_a,
                ..ci_run_b.clone()
            })
            .await,
        Err(StorageError::NotFound)
    ));

    let beta_key = api_key_record(workspace_b);
    repository
        .create_api_key(beta_key.clone())
        .await
        .expect("create beta API key");
    assert!(matches!(
        repository
            .revoke_api_key(workspace_a, beta_key.api_key.id)
            .await,
        Err(StorageError::NotFound)
    ));
    assert!(repository
        .find_api_key(&beta_key.secret_hash)
        .await
        .expect("read beta API key")
        .expect("beta API key exists")
        .api_key
        .revoked_at
        .is_none());
    repository
        .revoke_api_key(workspace_b, beta_key.api_key.id)
        .await
        .expect("revoke owned beta API key");

    let run_b = RetrievalEvalRun {
        id: RetrievalEvalRunId(Uuid::now_v7()),
        retrieval_mode: RetrievalMode::Lexical,
        case_count: 0,
        passed_count: 0,
        average_recall_at_k: 0.0,
        average_precision_at_k: 0.0,
        created_at: OffsetDateTime::now_utc(),
        results: Vec::new(),
    };
    repository
        .save_retrieval_eval_run(workspace_b, &run_b)
        .await
        .expect("save beta run");
    assert!(repository
        .latest_retrieval_eval_run(workspace_a)
        .await
        .expect("read alpha latest run")
        .is_none());
    assert_eq!(
        repository
            .latest_retrieval_eval_run(workspace_b)
            .await
            .expect("read beta latest run")
            .map(|run| run.id),
        Some(run_b.id)
    );

    assert_eq!(
        repository
            .get_retrieval_eval_dataset(workspace_a, dataset_a.id)
            .await
            .expect("read owned alpha dataset")
            .cases[0]
            .id,
        case_a.id
    );
}

async fn run_ci_baseline_history_contract<R>(repository: &R)
where
    R: AuthRepository + CiEvalRepository + EvalRepository + ProjectRepository,
{
    let workspace_id = create_workspace(repository, "ci-baseline-history").await;
    let dataset = dataset("CI baseline history");
    repository
        .create_retrieval_eval_dataset(workspace_id, dataset.clone())
        .await
        .expect("create CI baseline dataset");
    let project = repository
        .ensure_default_project(workspace_id)
        .await
        .expect("create CI baseline project");
    let now = OffsetDateTime::now_utc();

    let mut baseline = experiment(dataset.id, &dataset.name);
    baseline.provenance = Some(experiment_provenance(workspace_id, project.id, dataset.id));
    baseline.created_at = now - Duration::hours(3);
    repository
        .save_retrieval_eval_experiment(workspace_id, baseline.clone())
        .await
        .expect("save compatible baseline experiment");
    let mut baseline_run = ci_run(workspace_id, &dataset, &baseline);
    baseline_run.created_at = baseline.created_at + Duration::minutes(1);
    repository
        .save_ci_eval_run(baseline_run.clone())
        .await
        .expect("save compatible baseline run");

    let mut incompatible = experiment(dataset.id, &dataset.name);
    incompatible.provenance = baseline.provenance.clone();
    incompatible.top_k = 1;
    incompatible.config_snapshot.top_k = 1;
    incompatible
        .provenance
        .as_mut()
        .expect("incompatible provenance")
        .identity
        .retrieval
        .top_k = 1;
    incompatible.created_at = now - Duration::hours(2);
    repository
        .save_retrieval_eval_experiment(workspace_id, incompatible.clone())
        .await
        .expect("save incompatible experiment");
    for index in 0..101 {
        let mut run = ci_run(workspace_id, &dataset, &incompatible);
        run.created_at = now - Duration::hours(1) + Duration::seconds(index);
        repository
            .save_ci_eval_run(run)
            .await
            .expect("save newer incompatible run");
    }

    let mut current = experiment(dataset.id, &dataset.name);
    current.provenance = baseline.provenance.clone();
    current.created_at = now;
    let selected = repository
        .latest_compatible_ci_eval_run(workspace_id, &baseline_run.config_label, &current)
        .await
        .expect("select compatible baseline")
        .expect("older compatible baseline");
    assert_eq!(selected.id, baseline_run.id);
}

async fn run_golden_dataset_import_contract<R>(repository: &R)
where
    R: AuthRepository + ProjectRepository + SourceRepository + DocumentRepository + EvalRepository,
{
    let workspace_a = create_workspace(repository, "golden-import-a").await;
    let workspace_b = create_workspace(repository, "golden-import-b").await;
    let (document_a, chunk_a) = create_eval_evidence(repository, workspace_a, "a").await;
    let (document_b, chunk_b) = create_eval_evidence(repository, workspace_b, "b").await;

    let identities_a = repository
        .list_golden_dataset_evidence_identities(workspace_a)
        .await
        .expect("list workspace A portable evidence");
    assert!(identities_a
        .iter()
        .all(|identity| identity.document_id == document_a.id));
    assert!(!identities_a
        .iter()
        .any(|identity| identity.document_id == document_b.id));

    let now = OffsetDateTime::now_utc();
    let dataset = RetrievalEvalDataset {
        id: RetrievalEvalDatasetId(Uuid::now_v7()),
        name: "Imported golden dataset".to_owned(),
        description: Some("Versioned fixture".to_owned()),
        cases: Vec::new(),
        created_at: now,
        updated_at: now,
    };
    let mut foreign_case = eval_case("foreign-evidence");
    foreign_case.expected_document_ids = vec![document_b.id];
    foreign_case.expected_chunk_ids = vec![chunk_b.id];
    let rejected = repository
        .apply_retrieval_eval_dataset_import(
            workspace_a,
            RetrievalEvalDatasetImportWrite {
                dataset: dataset.clone(),
                expected_updated_at: None,
                cases_to_create: vec![foreign_case],
                cases_to_update: Vec::new(),
                case_ids_to_delete: Vec::new(),
            },
        )
        .await;
    assert!(matches!(rejected, Err(StorageError::UnavailableEvidence)));
    assert!(matches!(
        repository
            .get_retrieval_eval_dataset(workspace_a, dataset.id)
            .await,
        Err(StorageError::NotFound)
    ));

    let mut alpha = eval_case("alpha");
    alpha.expected_document_ids = vec![document_a.id];
    alpha.expected_chunk_ids = vec![chunk_a.id];
    let created = repository
        .apply_retrieval_eval_dataset_import(
            workspace_a,
            RetrievalEvalDatasetImportWrite {
                dataset: dataset.clone(),
                expected_updated_at: None,
                cases_to_create: vec![alpha.clone()],
                cases_to_update: Vec::new(),
                case_ids_to_delete: Vec::new(),
            },
        )
        .await
        .expect("create imported dataset atomically");
    assert_eq!(created.cases.len(), 1);
    assert_eq!(created.cases[0].case_key, "alpha");

    let stale = repository
        .apply_retrieval_eval_dataset_import(
            workspace_a,
            RetrievalEvalDatasetImportWrite {
                dataset: RetrievalEvalDataset {
                    updated_at: now + Duration::seconds(1),
                    ..created.clone()
                },
                expected_updated_at: Some(now - Duration::seconds(1)),
                cases_to_create: vec![eval_case("must-not-persist")],
                cases_to_update: Vec::new(),
                case_ids_to_delete: Vec::new(),
            },
        )
        .await;
    assert!(matches!(stale, Err(StorageError::Conflict(_))));

    let mut changed_alpha = created.cases[0].clone();
    changed_alpha.notes = Some("merged change".to_owned());
    let mut beta = eval_case("beta");
    beta.expected_document_ids = vec![document_a.id];
    beta.expected_chunk_ids = vec![chunk_a.id];
    let merged = repository
        .apply_retrieval_eval_dataset_import(
            workspace_a,
            RetrievalEvalDatasetImportWrite {
                dataset: RetrievalEvalDataset {
                    updated_at: created.updated_at + Duration::seconds(1),
                    cases: Vec::new(),
                    ..created.clone()
                },
                expected_updated_at: Some(created.updated_at),
                cases_to_create: vec![beta],
                cases_to_update: vec![changed_alpha],
                case_ids_to_delete: Vec::new(),
            },
        )
        .await
        .expect("merge imported cases atomically");
    assert_eq!(merged.cases.len(), 2);
    assert!(merged
        .cases
        .iter()
        .any(|case| case.case_key == "alpha" && case.notes.as_deref() == Some("merged change")));
    assert!(!merged
        .cases
        .iter()
        .any(|case| case.case_key == "must-not-persist"));

    let alpha_id = merged
        .cases
        .iter()
        .find(|case| case.case_key == "alpha")
        .expect("alpha case")
        .id;
    let replaced = repository
        .apply_retrieval_eval_dataset_import(
            workspace_a,
            RetrievalEvalDatasetImportWrite {
                dataset: RetrievalEvalDataset {
                    updated_at: merged.updated_at + Duration::seconds(1),
                    cases: Vec::new(),
                    ..merged.clone()
                },
                expected_updated_at: Some(merged.updated_at),
                cases_to_create: Vec::new(),
                cases_to_update: Vec::new(),
                case_ids_to_delete: vec![alpha_id],
            },
        )
        .await
        .expect("replace removes omitted cases");
    assert_eq!(replaced.cases.len(), 1);
    assert_eq!(replaced.cases[0].case_key, "beta");
}

async fn create_eval_evidence<R>(
    repository: &R,
    workspace_id: WorkspaceId,
    label: &str,
) -> (Document, Chunk)
where
    R: ProjectRepository + SourceRepository + DocumentRepository,
{
    let project = repository
        .ensure_default_project(workspace_id)
        .await
        .expect("default evidence project");
    let source = Source {
        id: SourceId(Uuid::now_v7()),
        project_id: project.id,
        name: format!("Golden source {label}"),
        kind: SourceKind::FileSet {
            root_hint: label.to_owned(),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking: ChunkingConfig::default(),
    };
    repository
        .create_source(workspace_id, source.clone())
        .await
        .expect("create golden source");
    let document = Document {
        id: DocumentId(Uuid::now_v7()),
        source_id: source.id,
        path: format!("{label}.md"),
        mime_type: Some("text/markdown".to_owned()),
        checksum: format!("document-{label}"),
        byte_size: 8,
        profile: DocumentProfile::TechnicalDocs,
        extraction_quality: ExtractionQuality::High,
        warnings: Vec::new(),
    };
    let chunk = Chunk {
        id: ChunkId(Uuid::now_v7()),
        source_id: source.id,
        document_id: document.id,
        ordinal: 0,
        text: format!("evidence {label}"),
        token_count: 2,
        byte_range: ByteRange { start: 0, end: 8 },
        checksum: format!("chunk-{label}"),
        strategy: ChunkingStrategy::Structured,
        section_title: None,
        split_reason: ChunkSplitReason::DocumentEnd,
        quality_flags: Vec::new(),
        is_duplicate: false,
        text_density: 1.0,
        evidence_score_hint: 1.0,
    };
    repository
        .insert_document_with_chunks(workspace_id, document.clone(), vec![chunk.clone()])
        .await
        .expect("insert golden evidence");
    (document, chunk)
}

async fn create_workspace<R>(repository: &R, label: &str) -> WorkspaceId
where
    R: AuthRepository,
{
    let now = OffsetDateTime::now_utc();
    let marker = Uuid::now_v7();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: format!("{label} organization {marker}"),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: format!("{label} workspace {marker}"),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: format!("{label}-{marker}@example.test"),
        name: format!("{label} user"),
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
        .expect("create eval contract workspace");
    workspace.id
}

fn dataset(name: &str) -> RetrievalEvalDataset {
    let now = OffsetDateTime::now_utc();
    RetrievalEvalDataset {
        id: RetrievalEvalDatasetId(Uuid::now_v7()),
        name: name.to_owned(),
        description: None,
        cases: Vec::new(),
        created_at: now,
        updated_at: now,
    }
}

fn eval_case(name: &str) -> RetrievalEvalCase {
    RetrievalEvalCase {
        id: RetrievalEvalCaseId(Uuid::now_v7()),
        case_key: name.to_ascii_lowercase().replace(' ', "-"),
        name: name.to_owned(),
        query: format!("{name} query"),
        top_k: 5,
        expected_chunk_ids: Vec::new(),
        expected_document_ids: Vec::new(),
        notes: None,
        provenance: None,
        created_at: OffsetDateTime::now_utc(),
    }
}

fn full_local_trace(project_id: rag_debugger_core::ProjectId) -> Trace {
    let now = OffsetDateTime::now_utc();
    Trace {
        id: TraceId(Uuid::now_v7()),
        project_id,
        input: "private local query".to_owned(),
        output: None,
        started_at: now,
        completed_at: Some(now),
        retrieval_runs: Vec::new(),
        generation: None,
        failure_labels: Vec::new(),
        source_run_id: None,
        summary: "Imported trace".to_owned(),
        status: TraceStatus::Completed,
        evidence_strength: None,
        spans: Vec::new(),
        retrieval: None,
        reruns: Vec::new(),
        diagnosis: None,
        ingestion: Some(TraceIngestionMetadata {
            source: TraceIngestionSource::Native,
            external_trace_id: format!("eval-source-{}", Uuid::now_v7()),
            schema_version: "1".to_owned(),
            mapper_version: "1".to_owned(),
            mapping_status: TraceMappingStatus::Complete,
            privacy_mode: TraceIngestionPrivacyMode::FullLocalOnly,
            service_name: None,
            service_version: None,
            deployment_environment: None,
            instrumentation_scope_name: None,
            instrumentation_scope_version: None,
            known_failure_labels: Vec::new(),
            status_supplied: false,
            limitations: Vec::new(),
            prompt: None,
            retrieval_mode: Some(RetrievalMode::Hybrid),
            top_k: Some(5),
            model_config: None,
            evidence: Vec::new(),
            spans: Vec::new(),
            evaluation_passed: None,
            evaluation_label: None,
            timestamps_supplied: true,
        }),
    }
}

fn experiment(dataset_id: RetrievalEvalDatasetId, dataset_name: &str) -> RetrievalEvalExperiment {
    RetrievalEvalExperiment {
        id: RetrievalEvalExperimentId(Uuid::now_v7()),
        dataset_id,
        dataset_name: dataset_name.to_owned(),
        name: "Workspace isolation experiment".to_owned(),
        modes: vec![RetrievalMode::Lexical],
        top_k: 5,
        config_snapshot: RetrievalEvalConfigSnapshot {
            top_k: 5,
            scoring_weights: RetrievalWeights::default(),
            embedding_model: EmbeddingModelInfo::default(),
            dataset_case_count: 1,
        },
        provenance: None,
        mode_results: Vec::new(),
        comparison: RetrievalEvalComparison {
            best_mode: None,
            mode_count: 0,
            recall_delta: 0.0,
            precision_delta: 0.0,
            latency_delta_ms: 0,
            summary: "No mode results.".to_owned(),
        },
        gate: RetrievalEvalGate {
            status: RetrievalEvalGateStatus::Failed,
            average_recall_at_k: 0.0,
            weak_evidence_rate: 0.0,
            critical_failure_count: 0,
            recall_threshold: 0.8,
            weak_evidence_limit: 0.2,
            reasons: vec!["No results.".to_owned()],
        },
        failures: Vec::new(),
        created_at: OffsetDateTime::now_utc(),
    }
}

fn experiment_provenance(
    workspace_id: WorkspaceId,
    project_id: ProjectId,
    dataset_id: RetrievalEvalDatasetId,
) -> RetrievalEvalExperimentProvenance {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "fingerprint": "storage-contract",
        "identity": {
            "workspace_id": workspace_id,
            "project_ids": [project_id],
            "dataset": {"dataset_id": dataset_id, "revision_fingerprint": "dataset", "case_count": 1},
            "corpus": {"source_ids": [], "document_count": 0, "document_set_fingerprint": "documents", "documents": []},
            "chunking": {"fingerprint": "chunking", "sources": []},
            "chunk_set": {"fingerprint": "chunks", "chunk_count": 0},
            "embedding": {"provider": "local", "model_name": "local-hash-v1", "dimension": 384, "index_fingerprint": "index", "indexed_chunk_count": 0, "missing_chunk_count": 0, "stale_chunk_count": 0},
            "retrieval": {
                "modes": ["lexical"], "top_k": 5,
                "scoring": {
                    "weights": RetrievalWeights::default(),
                    "min_evidence_score": 0.35,
                    "min_semantic_similarity": 0.25,
                    "answer_citation_limit": 3,
                    "answerability": rag_debugger_core::AnswerabilityConfig::default()
                },
                "filters": {"source_ids": [], "document_ids": []},
                "runtime_flags": {}
            }
        },
        "informational": {"application_version": "test", "deployment_mode": "local", "runtime_environment": "test", "storage_backend": "memory", "labels": {}}
    }))
    .expect("valid experiment provenance fixture")
}

fn ci_run(
    workspace_id: WorkspaceId,
    dataset: &RetrievalEvalDataset,
    experiment: &RetrievalEvalExperiment,
) -> CiEvalRun {
    CiEvalRun {
        id: CiEvalRunId(Uuid::now_v7()),
        workspace_id,
        dataset_id: dataset.id,
        dataset_name: dataset.name.clone(),
        experiment_id: experiment.id,
        status: CiEvalRunStatus::Failed,
        gate_status: RetrievalEvalGateStatus::Failed,
        branch: Some("feature/workspace-contract".to_owned()),
        commit_sha: Some("0123456789abcdef".to_owned()),
        base_ref: Some("main".to_owned()),
        head_ref: Some("feature/workspace-contract".to_owned()),
        config_label: "workspace-contract".to_owned(),
        regression: None,
        eval_regression: None,
        report: CiEvalReport {
            title: "CI workspace contract".to_owned(),
            summary: "The gate failed.".to_owned(),
            gate: experiment.gate.clone(),
            experiment: experiment.clone(),
            failed_cases: experiment.failures.clone(),
        },
        created_at: OffsetDateTime::now_utc(),
    }
}

fn api_key_record(workspace_id: WorkspaceId) -> ApiKeyRecord {
    let id = ApiKeyId(Uuid::now_v7());
    ApiKeyRecord {
        api_key: ApiKey {
            id,
            workspace_id,
            name: "Workspace contract key".to_owned(),
            prefix: format!("clab_{}", &id.0.simple().to_string()[..8]),
            scopes: vec![ApiKeyScope::CiEvalRuns],
            created_at: OffsetDateTime::now_utc(),
            last_used_at: None,
            revoked_at: None,
        },
        secret_hash: format!("workspace-contract-secret-{}", id.0),
    }
}
