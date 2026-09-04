use std::{
    env,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    sync::Arc,
    time::Duration,
};

use rag_debugger_api::{
    app, auth,
    config::{ApiConfig, MigrationConfig, RuntimeEnvironment, StorageBackend},
    state::AppState,
    telemetry,
};
use rag_debugger_storage::{
    memory::MemoryStore, postgres::PostgresStore, repository::AppRepository,
};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match env::args().nth(1).as_deref() {
        Some("migrate") => return migrate().await,
        Some("healthcheck") => return probe("/healthz"),
        Some("readycheck") => return probe("/readyz"),
        Some(command) => return Err(format!("unknown command: {command}").into()),
        None => {}
    }

    let config = ApiConfig::from_env()?;
    telemetry::init(&config.log_filter);

    let repository: Arc<dyn AppRepository> = match config.storage_backend {
        StorageBackend::Postgres => {
            let store = PostgresStore::connect(&config.database_url).await?;
            if matches!(config.environment, RuntimeEnvironment::Local) {
                store.run_migrations().await?;
            } else {
                store.verify_migrations().await?;
            }
            Arc::new(store)
        }
        StorageBackend::Memory => {
            let store = MemoryStore::default();
            Arc::new(store)
        }
    };
    let authenticated = auth::bootstrap_identity(repository.as_ref(), &config.auth).await?;
    repository
        .ensure_default_project(authenticated.workspace.id)
        .await?;

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let state = AppState::new(config.clone(), repository);

    info!(
        address = %config.bind_addr,
        environment = ?config.environment,
        release_sha = %config.release_sha,
        storage_backend = ?config.storage_backend,
        "starting corpuslab api"
    );

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn migrate() -> Result<(), Box<dyn std::error::Error>> {
    let config = MigrationConfig::from_env()?;
    PostgresStore::connect(&config.database_url)
        .await?
        .run_migrations()
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(cause) = tokio::signal::ctrl_c().await {
            error!(error = %cause, "failed to receive Ctrl+C signal");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(cause) => error!(error = %cause, "failed to receive SIGTERM signal"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

fn probe(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("RAG_DEBUGGER_API_PORT")
        .unwrap_or_else(|_| "8080".to_owned())
        .parse::<u16>()?;
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let timeout = Duration::from_secs(2);
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )?;
    let mut response = [0_u8; 12];
    stream.read_exact(&mut response)?;
    if response == *b"HTTP/1.1 200" {
        Ok(())
    } else {
        Err("probe returned a non-200 response".into())
    }
}

#[cfg(test)]
mod tests {
    use super::probe;

    #[test]
    fn probe_rejects_an_unavailable_listener() {
        std::env::set_var("RAG_DEBUGGER_API_PORT", "0");
        assert!(probe("/healthz").is_err());
        std::env::remove_var("RAG_DEBUGGER_API_PORT");
    }
}
