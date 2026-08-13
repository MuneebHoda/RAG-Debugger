use rag_debugger_core::{
    ApiKey, ApiKeyId, ApiKeyRecord, ApiKeyScope, CiEvalReport, CiEvalRun, CiEvalRunId,
    CiEvalRunStatus, EmbeddingModelInfo, Organization, OrganizationId, RetrievalEvalCase,
    RetrievalEvalCaseId, RetrievalEvalCaseProvenance, RetrievalEvalComparison,
    RetrievalEvalConfigSnapshot, RetrievalEvalDataset, RetrievalEvalDatasetId,
    RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalGate, RetrievalEvalGateStatus,
    RetrievalEvalRun, RetrievalEvalRunId, RetrievalMode, RetrievalWeights, Trace, TraceId,
    TraceIngestionMetadata, TraceIngestionPrivacyMode, TraceIngestionSource, TraceMappingStatus,
    TraceStatus, User, UserId, Workspace, WorkspaceId, WorkspaceRole,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{
        AuthRepository, CiEvalRepository, EvalRepository, ProjectRepository,
        SubmittedExpectedEvidence, TraceRepository,
    },
    StorageError,
};
use time::OffsetDateTime;
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

    let metadata = trace.ingestion.as_ref().expect("source metadata");
    let mut validation = store.pool().begin().await.expect("validation transaction");
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM debug_traces
         WHERE id = $1 AND workspace_id = $2
           AND ingestion_source = $3 AND ingestion_privacy_mode = $4
           AND trace_json ->> 'input' = $5
         FOR SHARE",
    )
    .bind(trace.id.0)
    .bind(workspace_id.0)
    .bind(metadata.source.as_str())
    .bind(metadata.privacy_mode.as_str())
    .bind(&trace.input)
    .fetch_one(&mut *validation)
    .await
    .expect("lock matching source trace");

    let mut changed = trace;
    changed.input = "changed after validation".to_owned();
    let writer = store.clone();
    let mut update =
        tokio::spawn(async move { writer.upsert_imported_trace(workspace_id, changed).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut update)
            .await
            .is_err(),
        "source trace update must wait for provenance validation"
    );
    validation.commit().await.expect("commit validation lock");
    update
        .await
        .expect("join source trace update")
        .expect("update source trace after validation");
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

    let experiment_b = experiment(dataset_b.id, &dataset_b.name);
    repository
        .save_retrieval_eval_experiment(workspace_b, experiment_b.clone())
        .await
        .expect("save beta experiment");
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
        .latest_ci_eval_run_for_dataset(workspace_a, dataset_b.id, &ci_run_b.config_label)
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
