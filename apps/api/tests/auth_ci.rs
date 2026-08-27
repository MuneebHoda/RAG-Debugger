mod support;

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
        database_url: String::new(),
        web_origin: "http://127.0.0.1:5173".to_owned(),
        auth: support::test_auth_config(),
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
    let unauthorized_body = json_body(unauthorized).await;
    assert_eq!(unauthorized_body["error"]["code"], "unauthorized");
    assert!(unauthorized_body["error"]["message"].is_string());

    let (cookie, body) = login(&app).await;
    assert_eq!(body["user"]["user"]["email"], support::TEST_BOOTSTRAP_EMAIL);

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
    let eval_case = post_json_with_cookie(
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
    let case_id = eval_case["id"].as_str().expect("case id");

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
            "base_ref": "main",
            "head_ref": "feature/evals",
            "modes": ["lexical"],
            "config_label": "default",
            "fail_on_gate": true
        }),
        secret,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(ci_run["gate_status"], "passed");
    assert_eq!(ci_run["branch"], "feature/evals");
    assert!(ci_run["eval_regression"]["baseline_experiment_id"].is_null());
    assert_eq!(ci_run["eval_regression"]["classification"], "unchanged");
    assert_eq!(
        ci_run["report"]["experiment"]["provenance"]["informational"]["branch"],
        "feature/evals"
    );
    assert_eq!(
        ci_run["report"]["experiment"]["provenance"]["informational"]["commit_sha"],
        "abc123"
    );

    let compatible_run = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({
            "dataset_id": dataset_id,
            "branch": "feature/evals",
            "commit_sha": "abc124",
            "base_ref": "main",
            "head_ref": "feature/evals",
            "modes": ["lexical"],
            "config_label": "default",
            "fail_on_gate": true
        }),
        secret,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(
        compatible_run["eval_regression"]["baseline_experiment_id"],
        ci_run["report"]["experiment"]["id"]
    );
    assert_eq!(
        compatible_run["eval_regression"]["compatibility"]["classification"],
        "compatible"
    );

    request_json_with_cookie(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "query": "qxzv blorp" }),
        &cookie,
        StatusCode::OK,
    )
    .await;
    let failed_run = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({
            "dataset_id": dataset_id,
            "branch": "feature/evals",
            "commit_sha": "def456",
            "base_ref": "main",
            "head_ref": "feature/evals",
            "modes": ["lexical"],
            "config_label": "default",
            "fail_on_gate": true
        }),
        secret,
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    assert_eq!(failed_run["status"], "failed");
    assert_eq!(failed_run["gate_status"], "failed");
    assert_eq!(failed_run["eval_regression"]["classification"], "unchanged");
    assert!(failed_run["eval_regression"]["baseline_experiment_id"].is_null());
    assert_eq!(
        failed_run["eval_regression"]["compatibility"]["classification"],
        "legacy_unknown"
    );
    assert!(failed_run["regression"].is_null());
    assert!(!failed_run["report"]["failed_cases"]
        .as_array()
        .expect("failed cases")
        .is_empty());

    let failed_run_id = failed_run["id"].as_str().expect("failed run id");
    let persisted = get_json_with_cookie(
        &app,
        &format!("/api/v1/eval-lab/ci/runs/{failed_run_id}"),
        &cookie,
        StatusCode::OK,
    )
    .await;
    assert_eq!(persisted["id"], failed_run["id"]);

    let report = post_json_with_cookie(
        &app,
        "/api/v1/reports/from-ci-run",
        json!({ "run_id": failed_run_id }),
        &cookie,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(report["privacy_mode"], "metadata_only");
    assert_eq!(report["context"]["regression_classification"], "unchanged");
    assert_eq!(
        report["context"]["baseline_compatibility"],
        "legacy_unknown"
    );
    assert_eq!(report["context"]["recovered_cases"], "0");
    assert!(!report.to_string().contains("qxzv blorp"));

    let nonblocking_failure = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({
            "dataset_id": dataset_id,
            "name": "   ",
            "modes": ["lexical"],
            "config_label": "default",
            "fail_on_gate": false
        }),
        secret,
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(nonblocking_failure["status"], "failed");

    let incompatible_modes_run = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({
            "dataset_id": dataset_id,
            "modes": ["hybrid"],
            "config_label": "default",
            "fail_on_gate": false
        }),
        secret,
        StatusCode::CREATED,
    )
    .await;
    assert!(incompatible_modes_run["eval_regression"]["baseline_experiment_id"].is_null());
    assert_eq!(
        incompatible_modes_run["eval_regression"]["classification"],
        "unchanged"
    );
    assert!(incompatible_modes_run["regression"].is_null());

    let incompatible_run = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({
            "dataset_id": dataset_id,
            "modes": ["lexical"],
            "top_k": 1,
            "config_label": "default",
            "fail_on_gate": false
        }),
        secret,
        StatusCode::CREATED,
    )
    .await;
    assert!(incompatible_run["eval_regression"]["baseline_experiment_id"].is_null());
    assert_eq!(
        incompatible_run["eval_regression"]["classification"],
        "unchanged"
    );
    assert!(incompatible_run["regression"].is_null());

    let keys = get_json_with_cookie(&app, "/api/v1/api-keys", &cookie, StatusCode::OK).await;
    assert_eq!(keys[0]["scopes"][0], "ci_eval_runs");
    assert!(keys[0]["last_used_at"].is_string());
    assert!(keys[0].get("secret").is_none());
    assert!(keys[0].get("secret_hash").is_none());

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

#[tokio::test]
async fn api_key_management_and_ci_dataset_access_are_workspace_scoped() {
    let app = test_app(RuntimeEnvironment::Local).await;
    let (alpha_cookie, _) = login(&app).await;
    let alpha_key = post_json_with_cookie(
        &app,
        "/api/v1/api-keys",
        json!({ "name": "Alpha CI" }),
        &alpha_cookie,
        StatusCode::OK,
    )
    .await;
    let alpha_key_id = alpha_key["api_key"]["id"].as_str().expect("key id");
    let alpha_secret = alpha_key["secret"].as_str().expect("key secret");

    let invalid_name = post_json_with_cookie(
        &app,
        "/api/v1/api-keys",
        json!({ "name": "x".repeat(101) }),
        &alpha_cookie,
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(invalid_name["error"]["code"], "bad_request");

    let beta_cookie = signup(&app, "beta-ci@example.test", "Beta CI Workspace").await;
    let beta_dataset = post_json_with_cookie(
        &app,
        "/api/v1/eval-lab/datasets",
        json!({ "name": "Beta private gate" }),
        &beta_cookie,
        StatusCode::OK,
    )
    .await;

    let inaccessible = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({ "dataset_id": beta_dataset["id"] }),
        alpha_secret,
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_eq!(inaccessible["error"]["code"], "not_found");
    assert!(inaccessible["error"]["message"]
        .as_str()
        .expect("not-found message")
        .contains("eval dataset not found"));

    let invalid_metadata = post_json_with_bearer(
        &app,
        "/api/v1/eval-lab/ci/runs",
        json!({
            "dataset_id": beta_dataset["id"],
            "branch": "invalid\nbranch"
        }),
        alpha_secret,
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(invalid_metadata["error"]["code"], "bad_request");

    let cross_workspace_revoke = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/api-keys/{alpha_key_id}"))
                .header(header::COOKIE, beta_cookie)
                .body(Body::empty())
                .expect("cross-workspace revoke request"),
        )
        .await
        .expect("cross-workspace revoke response");
    assert_eq!(cross_workspace_revoke.status(), StatusCode::NOT_FOUND);

    let alpha_keys =
        get_json_with_cookie(&app, "/api/v1/api-keys", &alpha_cookie, StatusCode::OK).await;
    assert!(alpha_keys[0]["revoked_at"].is_null());
}

async fn signup(app: &axum::Router, email: &str, workspace_name: &str) -> String {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/signup",
            json!({
                "email": email,
                "password": "VeryStrong#2026",
                "name": "CI workspace owner",
                "workspace_name": workspace_name
            }),
        ))
        .await
        .expect("signup response");
    assert_eq!(response.status(), StatusCode::OK);
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("signup set-cookie")
        .to_str()
        .expect("signup cookie")
        .split(';')
        .next()
        .expect("signup cookie pair")
        .to_owned()
}

async fn login(app: &axum::Router) -> (String, Value) {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/login",
            json!({
                "email": support::TEST_BOOTSTRAP_EMAIL,
                "password": support::TEST_BOOTSTRAP_PASSWORD
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

async fn request_json_with_cookie(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Value,
    cookie: &str,
    expected_status: StatusCode,
) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
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

async fn get_json_with_cookie(
    app: &axum::Router,
    uri: &str,
    cookie: &str,
    expected_status: StatusCode,
) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
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
