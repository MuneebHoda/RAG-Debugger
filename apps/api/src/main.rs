use rag_debugger_api::{app, config::ApiConfig, state::AppState, telemetry};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ApiConfig::from_env()?;
    telemetry::init();

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let state = AppState::new(config.clone());

    info!(
        address = %config.bind_addr,
        environment = ?config.environment,
        "starting rag debugger api"
    );

    axum::serve(listener, app(state)).await?;
    Ok(())
}
