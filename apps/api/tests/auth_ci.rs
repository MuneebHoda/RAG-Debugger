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
use rag_debugger_core::ProductConfig;
use rag_debugger_storage::memory::MemoryStore;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn test_app(environment: RuntimeEnvironment) -> axum::Router {
    let store = MemoryStore::default();
    let config = ApiConfig {
        environment,
        bind_addr: "127.0.0.1:0".parse().expect("valid test socket"),
        storage_backend: StorageBackend::Memory,
        database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test".to_owned(),
        web_origin: "http://127.0.0.1:5173".to_owned(),
        auth: Default::default(),
        product: ProductConfig::default(),
    };
    let repository = Arc::new(store);
    auth::bootstrap_identity(repository.as_ref(), &config.auth)
        .await
        .expect("bootstrap identity");
    app(AppState::new(config, repository))
}

#[tokio::test]
async fn protected_routes_require_login_and_accept_session_cookie() {
    let app = test_app(RuntimeEnvironment::Local).await;

    let unauthorized = app
        .clone()
        .oneshot(empty_request(Method::GET, "/api/v1/overview"))
        .await
        .expect("unauthorized response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let (cookie, body) = login(&app).await;
    assert_eq!(body["user"]["user"]["email"], "demo@corpuslab.ai");

    let authorized = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/overview")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("authorized response");
    assert_eq!(authorized.status(), StatusCode::OK);
}

#[tokio::test]
async fn signup_rejects_duplicate_email_and_logout_revokes_session() {
    let app = test_app(RuntimeEnvironment::Local).await;

    let first_signup = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/signup",
            json!({
                "email": "owner@example.com",
                "password": "VeryStrong#2026",
                "name": "Owner",
                "workspace_name": "Owner Workspace"
            }),
        ))
        .await
        .expect("signup response");
    assert_eq!(first_signup.status(), StatusCode::OK);
    let cookie = first_signup
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();

    let duplicate_signup = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/signup",
            json!({
                "email": "OWNER@example.com",
                "password": "VeryStrong#2026",
                "workspace_name": "Duplicate Workspace"
            }),
        ))
        .await
        .expect("duplicate response");
    assert_eq!(duplicate_signup.status(), StatusCode::CONFLICT);

    let logout = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/logout")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("logout request"),
        )
        .await
        .expect("logout response");
    assert_eq!(logout.status(), StatusCode::OK);

    let rejected = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/overview")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("overview request"),
        )
        .await
        .expect("rejected response");
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_keys_authorize_ci_eval_runs_and_can_be_revoked() {
    let app = test_app(RuntimeEnvironment::Local).await;
    let (cookie, _) = login(&app).await;
    let upload_body = upload_text_file(
        &app,
        &cookie,
        "platform-guide.md",
        "GPU Indexing\n- Local workers refresh embeddings quickly.",
    )
    .await;
    let document_id = upload_body["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");
    let chunk_id = upload_body["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");
    post_json_with_cookie(
        &app,
        "/api/v1/embeddings/index",
        json!({}),
        &cookie,
        StatusCode::OK,
    )
    .await;
    let dataset = post_json_with_cookie(
        &app,
        "/api/v1/eval-lab/datasets",
        json!({ "name": "Release gate" }),
        &cookie,
        StatusCode::OK,
    )
    .await;
    let dataset_id = dataset["id"].as_str().expect("dataset id");
    post_json_with_cookie(
        &app,
        &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
        json!({
            "query": "gpu indexing workers",
            "expected_chunk_ids": [chunk_id],
            "expected_document_ids": [document_id]
        }),
        &cookie,
        StatusCode::OK,
    )
    .await;

    let created_key = post_json_with_cookie(
        &app,
        "/api/v1/api-keys",
        json!({ "name": "GitHub Actions" }),
        &cookie,
        StatusCode::OK,
    )
    .await;
    let secret = created_key["secret"].as_str().expect("secret");
    assert!(secret.starts_with("clab_"));

    let ci_run = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({
            "dataset_id": dataset_id,
            "branch": "feature/evals",
            "commit_sha": "abc123",
            "config_label": "default",
            "fail_on_gate": true
        }),
        secret,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(ci_run["gate_status"], "passed");
    assert_eq!(ci_run["branch"], "feature/evals");

    let api_key_id = created_key["api_key"]["id"].as_str().expect("key id");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/api-keys/{api_key_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("delete response");
    assert_eq!(response.status(), StatusCode::OK);

    let rejected = app
        .oneshot(json_request_with_bearer(
            Method::POST,
            "/api/v1/eval-lab/ci/runs",
            json!({ "dataset_id": dataset_id }),
            secret,
        ))
        .await
        .expect("rejected response");
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
}

async fn login(app: &axum::Router) -> (String, Value) {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/login",
            json!({
                "email": "demo@corpuslab.ai",
                "password": "CorpusLab#2026"
            }),
        ))
        .await
        .expect("login response");
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    (cookie, json_body(response).await)
}

async fn upload_text_file(
    app: &axum::Router,
    cookie: &str,
    file_name: &str,
    content: &str,
) -> Value {
    let response = app
        .clone()
        .oneshot(multipart_request(file_name, content, cookie))
        .await
        .expect("upload response");
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn post_json_with_cookie(
    app: &axum::Router,
    uri: &str,
    body: Value,
    cookie: &str,
    expected_status: StatusCode,
) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), expected_status);
    json_body(response).await
}

async fn post_json_with_bearer(
    app: &axum::Router,
    uri: &str,
    body: Value,
    token: &str,
    expected_status: StatusCode,
) -> Value {
    let response = app
        .clone()
        .oneshot(json_request_with_bearer(Method::POST, uri, body, token))
        .await
        .expect("response");
    let status = response.status();
    if status != expected_status {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        panic!(
            "expected {expected_status}, got {status}: {}",
            String::from_utf8_lossy(&bytes)
        );
    }
    json_body(response).await
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

fn json_request_with_bearer(method: Method, uri: &str, body: Value, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .expect("request")
}

fn empty_request(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("request")
}

fn multipart_request(file_name: &str, content: &str, cookie: &str) -> Request<Body> {
    let boundary = "CORPUSLAB_AUTH_TEST_BOUNDARY";
    let mut body = String::new();
    body.push_str(&format!("--{boundary}\r\n"));
    body.push_str("Content-Disposition: form-data; name=\"target_tokens\"\r\n\r\n40\r\n");
    body.push_str(&format!("--{boundary}\r\n"));
    body.push_str("Content-Disposition: form-data; name=\"overlap_tokens\"\r\n\r\n0\r\n");
    body.push_str(&format!("--{boundary}\r\n"));
    body.push_str(&format!(
        "Content-Disposition: form-data; name=\"files[]\"; filename=\"{file_name}\"\r\n"
    ));
    body.push_str("Content-Type: text/markdown\r\n\r\n");
    body.push_str(content);
    body.push_str("\r\n");
    body.push_str(&format!("--{boundary}--\r\n"));

    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sources/files")
        .header(header::COOKIE, cookie)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("request")
}
