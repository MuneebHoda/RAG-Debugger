use std::{path::PathBuf, sync::Arc};

use rag_debugger_api::{
    app, auth,
    config::{ApiConfig, StorageBackend},
    state::AppState,
    telemetry,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    postgres::PostgresStore,
    repository::{AppRepository, ProjectRepository},
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ApiConfig::from_env()?;
    telemetry::init();

    let repository: Arc<dyn AppRepository> = match config.storage_backend {
        StorageBackend::Postgres => {
            let store = PostgresStore::connect(&config.database_url).await?;
            let migrations_path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
            store.run_migrations(&migrations_path).await?;
            store.ensure_default_project().await?;
            Arc::new(store)
        }
        StorageBackend::Memory => {
            let store = MemoryStore::default();
            store.ensure_default_project().await?;
            Arc::new(store)
        }
    };
    auth::bootstrap_identity(repository.as_ref(), &config.auth).await?;

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let state = AppState::new(config.clone(), repository);

    info!(
        address = %config.bind_addr,
        environment = ?config.environment,
        storage_backend = ?config.storage_backend,
        "starting corpuslab api"
    );

    axum::serve(listener, app(state)).await?;
    Ok(())
}
