use rag_debugger_core::{
    ApiKey, ApiKeyId, ApiKeyRecord, ApiKeyScope, CiEvalReport, CiEvalRun, CiEvalRunId,
    CiEvalRunStatus, EmbeddingModelInfo, Organization, OrganizationId, RetrievalEvalCase,
    RetrievalEvalCaseId, RetrievalEvalComparison, RetrievalEvalConfigSnapshot,
    RetrievalEvalDataset, RetrievalEvalDatasetId, RetrievalEvalExperiment,
    RetrievalEvalExperimentId, RetrievalEvalGate, RetrievalEvalGateStatus, RetrievalEvalRun,
    RetrievalEvalRunId, RetrievalMode, RetrievalWeights, User, UserId, Workspace, WorkspaceId,
    WorkspaceRole,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{AuthRepository, CiEvalRepository, EvalRepository, SubmittedExpectedEvidence},
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

async fn run_eval_workspace_contract<R>(repository: &R)
where
    R: AuthRepository + CiEvalRepository + EvalRepository,
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
        created_at: OffsetDateTime::now_utc(),
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
