use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rag_debugger_api::{
    app,
    config::{ApiConfig, RuntimeEnvironment, StorageBackend},
    state::AppState,
};
use rag_debugger_core::ProductConfig;
use rag_debugger_storage::memory::MemoryStore;
use std::sync::Arc;
use tower::ServiceExt;

fn test_state(environment: RuntimeEnvironment) -> AppState {
    AppState::new(
        ApiConfig {
            environment,
            bind_addr: "127.0.0.1:0".parse().expect("valid test socket"),
            storage_backend: StorageBackend::Memory,
            database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test"
                .to_owned(),
            web_origin: "http://127.0.0.1:5173".to_owned(),
            product: ProductConfig::default(),
        },
        Arc::new(MemoryStore::default()),
    )
}

fn not_ready_state() -> AppState {
    AppState::without_repository(ApiConfig {
        environment: RuntimeEnvironment::Test,
        bind_addr: "127.0.0.1:0".parse().expect("valid test socket"),
        storage_backend: StorageBackend::Memory,
        database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test".to_owned(),
        web_origin: "http://127.0.0.1:5173".to_owned(),
        product: ProductConfig::default(),
    })
}

#[tokio::test]
async fn healthz_returns_ok() {
    let response = app(test_state(RuntimeEnvironment::Local))
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_returns_ok_when_ready() {
    let response = app(test_state(RuntimeEnvironment::Local))
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_returns_unavailable_when_not_ready() {
    let response = app(not_ready_state())
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
