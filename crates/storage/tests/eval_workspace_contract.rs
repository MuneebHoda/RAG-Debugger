use rag_debugger_core::{
    EmbeddingModelInfo, Organization, OrganizationId, RetrievalEvalCase, RetrievalEvalCaseId,
    RetrievalEvalComparison, RetrievalEvalConfigSnapshot, RetrievalEvalDataset,
    RetrievalEvalDatasetId, RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalGate,
    RetrievalEvalGateStatus, RetrievalEvalRun, RetrievalEvalRunId, RetrievalMode, RetrievalWeights,
    User, UserId, Workspace, WorkspaceId, WorkspaceRole,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{AuthRepository, EvalRepository, SubmittedExpectedEvidence},
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
    R: AuthRepository + EvalRepository,
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
