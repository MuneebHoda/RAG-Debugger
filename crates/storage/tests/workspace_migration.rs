use std::{borrow::Cow, path::PathBuf};

use rag_debugger_storage::{postgres::PostgresStore, StorageError};
use sqlx::{
    migrate::{MigrateError, Migrator},
    postgres::PgPoolOptions,
    Executor, PgPool,
};
use uuid::Uuid;

const WORKSPACE_ISOLATION_MIGRATION: i64 = 20_260_726_120_000;
const LEGACY_DEFAULT_DATASET: &str = "018f7a2a-6e2e-7000-a000-00000000e001";

#[tokio::test]
#[ignore = "creates temporary Postgres databases"]
async fn workspace_ownership_migration_backfills_singletons_and_quarantines_ambiguity() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    run_single_workspace_backfill(&database_url).await;
    run_ambiguous_workspace_quarantine(&database_url).await;
}

#[tokio::test]
#[ignore = "creates temporary Postgres databases"]
async fn postgres_migration_verification_rejects_incompatible_states() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let database = TemporaryDatabase::create(&database_url, "verification").await;
    let store = PostgresStore::new(database.pool.clone());

    assert!(matches!(
        store.verify_migrations().await,
        Err(StorageError::PendingMigration(
            WORKSPACE_ISOLATION_MIGRATION
        ))
    ));

    database.apply_workspace_migration().await;
    store
        .verify_migrations()
        .await
        .expect("fully migrated database verifies");

    let latest_version =
        sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(&database.pool)
            .await
            .expect("read latest migration version")
            .expect("at least one migration");
    sqlx::query("UPDATE _sqlx_migrations SET success = FALSE WHERE version = $1")
        .bind(latest_version)
        .execute(&database.pool)
        .await
        .expect("mark migration dirty");
    assert!(matches!(
        store.verify_migrations().await,
        Err(StorageError::Migrate(MigrateError::Dirty(version))) if version == latest_version
    ));
    sqlx::query("UPDATE _sqlx_migrations SET success = TRUE WHERE version = $1")
        .bind(latest_version)
        .execute(&database.pool)
        .await
        .expect("restore migration success state");

    let unexpected_version = 99_999_999_999_999_i64;
    sqlx::query(
        "INSERT INTO _sqlx_migrations (
             version, description, installed_on, success, checksum, execution_time
         )
         VALUES ($1, 'unexpected test migration', NOW(), TRUE, decode('00', 'hex'), 0)",
    )
    .bind(unexpected_version)
    .execute(&database.pool)
    .await
    .expect("insert unexpected migration");
    assert!(matches!(
        store.verify_migrations().await,
        Err(StorageError::Migrate(MigrateError::VersionMissing(version)))
            if version == unexpected_version
    ));
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = $1")
        .bind(unexpected_version)
        .execute(&database.pool)
        .await
        .expect("remove unexpected migration");

    sqlx::query("UPDATE _sqlx_migrations SET checksum = decode('00', 'hex') WHERE version = $1")
        .bind(latest_version)
        .execute(&database.pool)
        .await
        .expect("replace migration checksum");
    assert!(matches!(
        store.verify_migrations().await,
        Err(StorageError::Migrate(MigrateError::VersionMismatch(version)))
            if version == latest_version
    ));

    database.drop().await;
}

async fn run_single_workspace_backfill(database_url: &str) {
    let database = TemporaryDatabase::create(database_url, "singleton").await;
    let workspace_id = Uuid::now_v7();
    let project_id = Uuid::now_v7();
    insert_workspace(&database.pool, workspace_id, "Singleton").await;
    sqlx::query(
        "INSERT INTO projects (id, name, privacy_mode, created_at, updated_at, workspace_id)
         VALUES ($1, 'Legacy project', 'local', NOW(), NOW(), NULL)",
    )
    .bind(project_id)
    .execute(&database.pool)
    .await
    .expect("insert singleton legacy project");
    let run_id = Uuid::now_v7();
    let trace_id = Uuid::now_v7();
    insert_legacy_retrieval_run(&database.pool, run_id, "singleton legacy run").await;
    insert_legacy_trace(
        &database.pool,
        trace_id,
        project_id,
        Some(run_id),
        "singleton legacy trace",
    )
    .await;

    database.apply_workspace_migration().await;

    let project_owner: Option<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_one(&database.pool)
            .await
            .expect("read singleton project owner");
    let dataset_owner: Option<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM retrieval_eval_datasets WHERE id = $1")
            .bind(Uuid::parse_str(LEGACY_DEFAULT_DATASET).expect("default dataset UUID"))
            .fetch_one(&database.pool)
            .await
            .expect("read singleton dataset owner");
    assert_eq!(project_owner, Some(workspace_id));
    assert_eq!(dataset_owner, Some(workspace_id));
    assert_eq!(
        retrieval_run_owner(&database.pool, run_id).await,
        Some(workspace_id)
    );
    assert_eq!(
        trace_owner(&database.pool, trace_id).await,
        Some(workspace_id)
    );
    let partial_ingestion =
        sqlx::query("UPDATE debug_traces SET ingestion_mapper_version = '1' WHERE id = $1")
            .bind(trace_id)
            .execute(&database.pool)
            .await
            .expect_err("partially populated ingestion identity must be rejected");
    assert!(matches!(
        partial_ingestion,
        sqlx::Error::Database(ref error)
            if error.constraint() == Some("debug_traces_ingestion_identity_check")
    ));

    database.drop().await;
}

async fn run_ambiguous_workspace_quarantine(database_url: &str) {
    let database = TemporaryDatabase::create(database_url, "ambiguous").await;
    let workspace_a = Uuid::now_v7();
    let workspace_b = Uuid::now_v7();
    insert_workspace(&database.pool, workspace_a, "Alpha").await;
    insert_workspace(&database.pool, workspace_b, "Beta").await;
    let project_a = Uuid::now_v7();
    let project_b = Uuid::now_v7();
    let unowned_project = Uuid::now_v7();
    insert_project(
        &database.pool,
        project_a,
        Some(workspace_a),
        "Alpha project",
    )
    .await;
    insert_project(&database.pool, project_b, Some(workspace_b), "Beta project").await;
    insert_project(&database.pool, unowned_project, None, "Ambiguous project").await;
    let source_a = Uuid::now_v7();
    let source_b = Uuid::now_v7();
    insert_source(&database.pool, source_a, project_a, "Alpha source").await;
    insert_source(&database.pool, source_b, project_b, "Beta source").await;
    let document_a = Uuid::now_v7();
    let document_b = Uuid::now_v7();
    insert_document(&database.pool, document_a, source_a, "alpha.md").await;
    insert_document(&database.pool, document_b, source_b, "beta.md").await;
    let conflicting_run = Uuid::now_v7();
    let alpha_trace = Uuid::now_v7();
    let beta_trace = Uuid::now_v7();
    insert_legacy_retrieval_run(&database.pool, conflicting_run, "conflicting run").await;
    insert_legacy_trace(
        &database.pool,
        alpha_trace,
        project_a,
        Some(conflicting_run),
        "alpha ownership signal",
    )
    .await;
    insert_legacy_trace(
        &database.pool,
        beta_trace,
        project_b,
        Some(conflicting_run),
        "beta ownership signal",
    )
    .await;
    let quarantined_run = Uuid::now_v7();
    let quarantined_trace = Uuid::now_v7();
    insert_legacy_retrieval_run(&database.pool, quarantined_run, "unowned run").await;
    insert_legacy_trace(
        &database.pool,
        quarantined_trace,
        unowned_project,
        Some(quarantined_run),
        "unowned trace",
    )
    .await;
    sqlx::query(
        "INSERT INTO retrieval_eval_cases (
             id, dataset_id, name, query, top_k, expected_chunk_ids,
             expected_document_ids, notes, created_at
         )
         VALUES ($1, $2, 'Ambiguous case', 'ambiguous evidence', 5, '{}', $3, NULL, NOW())",
    )
    .bind(Uuid::now_v7())
    .bind(Uuid::parse_str(LEGACY_DEFAULT_DATASET).expect("default dataset UUID"))
    .bind(vec![document_a, document_b])
    .execute(&database.pool)
    .await
    .expect("insert ambiguous legacy case");

    database.apply_workspace_migration().await;

    let dataset_owner: Option<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM retrieval_eval_datasets WHERE id = $1")
            .bind(Uuid::parse_str(LEGACY_DEFAULT_DATASET).expect("default dataset UUID"))
            .fetch_one(&database.pool)
            .await
            .expect("read ambiguous dataset owner");
    let project_owner: Option<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM projects WHERE id = $1")
            .bind(unowned_project)
            .fetch_one(&database.pool)
            .await
            .expect("read ambiguous project owner");
    assert_eq!(dataset_owner, None);
    assert_eq!(project_owner, None);
    assert_eq!(
        retrieval_run_owner(&database.pool, conflicting_run).await,
        None
    );
    assert_eq!(
        retrieval_run_owner(&database.pool, quarantined_run).await,
        None
    );
    assert_eq!(
        trace_owner(&database.pool, alpha_trace).await,
        Some(workspace_a)
    );
    assert_eq!(
        trace_owner(&database.pool, beta_trace).await,
        Some(workspace_b)
    );
    assert_eq!(trace_owner(&database.pool, quarantined_trace).await, None);

    database.drop().await;
}

struct TemporaryDatabase {
    admin_pool: PgPool,
    pool: PgPool,
    name: String,
    migrations: Migrator,
}

impl TemporaryDatabase {
    async fn create(database_url: &str, label: &str) -> Self {
        let (admin_url, database_prefix, suffix) = database_urls(database_url);
        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&admin_url)
            .await
            .expect("connect Postgres admin database");
        let name = format!("corpuslab_{label}_{}", Uuid::now_v7().simple());
        admin_pool
            .execute(format!("CREATE DATABASE \"{name}\"").as_str())
            .await
            .expect("create temporary migration database");
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&format!("{database_prefix}/{name}{suffix}"))
            .await
            .expect("connect temporary migration database");
        let migrations = Migrator::new(migrations_path())
            .await
            .expect("load repository migrations");
        let before_workspace_isolation = Migrator {
            migrations: Cow::Owned(
                migrations
                    .iter()
                    .filter(|migration| migration.version < WORKSPACE_ISOLATION_MIGRATION)
                    .cloned()
                    .collect(),
            ),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };
        before_workspace_isolation
            .run(&pool)
            .await
            .expect("apply migrations before workspace isolation");

        Self {
            admin_pool,
            pool,
            name,
            migrations,
        }
    }

    async fn apply_workspace_migration(&self) {
        self.migrations
            .run(&self.pool)
            .await
            .expect("apply workspace isolation migration");
    }

    async fn drop(self) {
        self.pool.close().await;
        self.admin_pool
            .execute(format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.name).as_str())
            .await
            .expect("drop temporary migration database");
        self.admin_pool.close().await;
    }
}

fn database_urls(database_url: &str) -> (String, String, String) {
    let (prefix, database_and_query) = database_url
        .rsplit_once('/')
        .expect("DATABASE_URL must include a database name");
    let suffix = database_and_query
        .find('?')
        .map_or_else(String::new, |index| database_and_query[index..].to_owned());
    (
        format!("{prefix}/postgres{suffix}"),
        prefix.to_owned(),
        suffix,
    )
}

fn migrations_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

async fn insert_workspace(pool: &PgPool, workspace_id: Uuid, label: &str) {
    let organization_id = Uuid::now_v7();
    sqlx::query("INSERT INTO organizations (id, name, created_at) VALUES ($1, $2, NOW())")
        .bind(organization_id)
        .bind(format!("{label} organization"))
        .execute(pool)
        .await
        .expect("insert migration-test organization");
    sqlx::query(
        "INSERT INTO workspaces (id, organization_id, name, created_at)
         VALUES ($1, $2, $3, NOW())",
    )
    .bind(workspace_id)
    .bind(organization_id)
    .bind(format!("{label} workspace"))
    .execute(pool)
    .await
    .expect("insert migration-test workspace");
}

async fn insert_project(pool: &PgPool, project_id: Uuid, workspace_id: Option<Uuid>, name: &str) {
    sqlx::query(
        "INSERT INTO projects (id, name, privacy_mode, created_at, updated_at, workspace_id)
         VALUES ($1, $2, 'local', NOW(), NOW(), $3)",
    )
    .bind(project_id)
    .bind(name)
    .bind(workspace_id)
    .execute(pool)
    .await
    .expect("insert migration-test project");
}

async fn insert_source(pool: &PgPool, source_id: Uuid, project_id: Uuid, name: &str) {
    sqlx::query(
        "INSERT INTO sources (
             id, project_id, name, source_kind, root_hint, sync_policy,
             target_tokens, overlap_tokens, chunking_strategy, created_at
         )
         VALUES ($1, $2, $3, 'file_set', 'migration-test', 'manual', 128, 16, 'structured', NOW())",
    )
    .bind(source_id)
    .bind(project_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert migration-test source");
}

async fn insert_document(pool: &PgPool, document_id: Uuid, source_id: Uuid, path: &str) {
    sqlx::query(
        "INSERT INTO documents (
             id, source_id, path, mime_type, checksum, byte_size, created_at
         )
         VALUES ($1, $2, $3, 'text/markdown', $4, 16, NOW())",
    )
    .bind(document_id)
    .bind(source_id)
    .bind(path)
    .bind(format!("checksum-{document_id}"))
    .execute(pool)
    .await
    .expect("insert migration-test document");
}

async fn insert_legacy_retrieval_run(pool: &PgPool, run_id: Uuid, query: &str) {
    sqlx::query(
        "INSERT INTO retrieval_playground_runs (
             id, query, top_k, answer_status, answer_text, latency_ms,
             created_at, retrieval_mode, response_json
         )
         VALUES ($1, $2, 5, 'insufficient_evidence', '', 1, NOW(), 'lexical', NULL)",
    )
    .bind(run_id)
    .bind(query)
    .execute(pool)
    .await
    .expect("insert legacy retrieval run");
}

async fn insert_legacy_trace(
    pool: &PgPool,
    trace_id: Uuid,
    project_id: Uuid,
    source_run_id: Option<Uuid>,
    query: &str,
) {
    sqlx::query(
        "INSERT INTO debug_traces (
             id, project_id, source_run_id, query, retrieval_mode, summary, status,
             evidence_strength, failure_labels, span_count, rerun_count, latency_ms,
             trace_json, created_at, updated_at
         )
         VALUES (
             $1, $2, $3, $4, 'lexical', 'legacy trace', 'completed',
             'weak', '{}', 0, 0, 1, '{}'::jsonb, NOW(), NOW()
         )",
    )
    .bind(trace_id)
    .bind(project_id)
    .bind(source_run_id)
    .bind(query)
    .execute(pool)
    .await
    .expect("insert legacy trace");
}

async fn retrieval_run_owner(pool: &PgPool, run_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("SELECT workspace_id FROM retrieval_playground_runs WHERE id = $1")
        .bind(run_id)
        .fetch_one(pool)
        .await
        .expect("read retrieval run owner")
}

async fn trace_owner(pool: &PgPool, trace_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("SELECT workspace_id FROM debug_traces WHERE id = $1")
        .bind(trace_id)
        .fetch_one(pool)
        .await
        .expect("read trace owner")
}
