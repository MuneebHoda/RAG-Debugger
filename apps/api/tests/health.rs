use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rag_debugger_api::{
    app,
    config::{ApiConfig, RuntimeEnvironment},
    state::AppState,
};
use tower::ServiceExt;

fn test_state(environment: RuntimeEnvironment) -> AppState {
    AppState::new(ApiConfig {
        environment,
        bind_addr: "127.0.0.1:0".parse().expect("valid test socket"),
        database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test".to_owned(),
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
    let response = app(test_state(RuntimeEnvironment::Test))
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
