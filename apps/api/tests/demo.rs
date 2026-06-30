use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use rag_debugger_api::{
    app, auth,
    config::{ApiConfig, RuntimeEnvironment, StorageBackend},
    state::AppState,
};
use rag_debugger_core::{DemoLoadResponse, DemoStatus, ProductConfig};
use rag_debugger_storage::memory::MemoryStore;
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn demo_requires_authentication_and_loads_idempotently() {
    let (app, config) = setup().await;
    let unauthorized = app
        .clone()
        .oneshot(empty_request(Method::GET, "/api/v1/demo", None))
        .await
        .expect("unauthorized response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let cookie = login(
        &app,
        &config.auth.bootstrap_email,
        &config.auth.bootstrap_password,
    )
    .await;
    let initial: DemoStatus = response_json(
        app.clone()
            .oneshot(empty_request(Method::GET, "/api/v1/demo", Some(&cookie)))
            .await
            .expect("initial status"),
    )
    .await;
    assert!(!initial.progress.sample_corpus_loaded);

    let first = app
        .clone()
        .oneshot(empty_request(
            Method::POST,
            "/api/v1/demo/load",
            Some(&cookie),
        ))
        .await
        .expect("first load");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first: DemoLoadResponse = response_json(first).await;
    assert_eq!(first.created_documents, 3);
    assert_eq!(first.status.progress.document_count, 3);
    assert!(first.status.progress.chunks_created);
    assert!(first.status.progress.chunk_count >= 3);

    let second = app
        .clone()
        .oneshot(empty_request(
            Method::POST,
            "/api/v1/demo/load",
            Some(&cookie),
        ))
        .await
        .expect("repeat load");
    assert_eq!(second.status(), StatusCode::OK);
    let second: DemoLoadResponse = response_json(second).await;
    assert_eq!(second.created_documents, 0);
    assert_eq!(second.status.progress.document_count, 3);
    assert_eq!(
        second.status.progress.chunk_count,
        first.status.progress.chunk_count
    );
    assert_eq!(second.status.source_id, first.status.source_id);
}

#[tokio::test]
async fn demo_status_and_records_are_isolated_by_workspace() {
    let (app, config) = setup().await;
    let first_cookie = login(
        &app,
        &config.auth.bootstrap_email,
        &config.auth.bootstrap_password,
    )
    .await;
    let loaded: DemoLoadResponse = response_json(
        app.clone()
            .oneshot(empty_request(
                Method::POST,
                "/api/v1/demo/load",
                Some(&first_cookie),
            ))
            .await
            .expect("first workspace load"),
    )
    .await;

    let signup = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/signup",
            json!({
                "email": "second@corpuslab.test",
                "password": "SecondWorkspace#2026",
                "name": "Second Owner",
                "workspace_name": "Second Workspace"
            }),
        ))
        .await
        .expect("signup response");
    assert_eq!(signup.status(), StatusCode::OK);
    let second_cookie = session_cookie(&signup);
    let second_status: DemoStatus = response_json(
        app.clone()
            .oneshot(empty_request(
                Method::GET,
                "/api/v1/demo",
                Some(&second_cookie),
            ))
            .await
            .expect("second workspace status"),
    )
    .await;
    assert!(!second_status.progress.sample_corpus_loaded);
    assert_ne!(second_status.source_id, loaded.status.source_id);

    let second_load: DemoLoadResponse = response_json(
        app.oneshot(empty_request(
            Method::POST,
            "/api/v1/demo/load",
            Some(&second_cookie),
        ))
        .await
        .expect("second workspace load"),
    )
    .await;
    assert_eq!(second_load.created_documents, 3);
    assert_ne!(second_load.status.source_id, loaded.status.source_id);
}

async fn setup() -> (axum::Router, ApiConfig) {
    let repository = Arc::new(MemoryStore::default());
    let config = ApiConfig {
        environment: RuntimeEnvironment::Local,
        bind_addr: "127.0.0.1:0".parse().expect("test socket"),
        storage_backend: StorageBackend::Memory,
        database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test".to_owned(),
        web_origin: "http://127.0.0.1:5173".to_owned(),
        auth: Default::default(),
        product: ProductConfig::default(),
    };
    auth::bootstrap_identity(repository.as_ref(), &config.auth)
        .await
        .expect("bootstrap identity");
    (app(AppState::new(config.clone(), repository)), config)
}

async fn login(app: &axum::Router, email: &str, password: &str) -> String {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/login",
            json!({ "email": email, "password": password }),
        ))
        .await
        .expect("login response");
    assert_eq!(response.status(), StatusCode::OK);
    session_cookie(&response)
}

fn session_cookie(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("session cookie")
        .to_owned()
}

fn empty_request(method: Method, uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    request.body(Body::empty()).expect("request")
}

fn json_request(method: Method, uri: &str, value: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("request")
}

async fn response_json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("JSON response")
}
