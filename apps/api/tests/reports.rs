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
use rag_debugger_core::{
    CiEvalReport, CiEvalRun, CiEvalRunId, CiEvalRunStatus, ProductConfig, RetrievalEvalExperiment,
    RetrievalEvalGateStatus,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    repository::{CiEvalRepository, ProjectRepository},
};
use serde_json::{json, Value};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

struct TestContext {
    app: axum::Router,
    store: Arc<MemoryStore>,
    cookie: String,
    workspace_id: rag_debugger_core::WorkspaceId,
}

#[tokio::test]
async fn report_routes_require_an_authenticated_session() {
    let context = setup().await;

    let response = context
        .app
        .oneshot(empty_request(Method::GET, "/api/v1/reports", None))
        .await
        .expect("report list response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(response).await["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn reports_are_created_listed_opened_and_exported() {
    let context = setup().await;
    let upload = upload_text_file(&context).await;
    let document_id = upload["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");
    let chunk_id = upload["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");
    request_json(
        &context,
        Method::POST,
        "/api/v1/embeddings/index",
        json!({}),
    )
    .await;
    let retrieval = request_json(
        &context,
        Method::POST,
        "/api/v1/retrieval/query",
        json!({ "query": "when is the index published" }),
    )
    .await;
    let trace = request_json(
        &context,
        Method::POST,
        "/api/v1/traces/from-retrieval-run",
        json!({ "run_id": retrieval["run"]["id"] }),
    )
    .await;
    let experiment = create_experiment(&context, document_id, chunk_id).await;
    let experiment_value: RetrievalEvalExperiment =
        serde_json::from_value(experiment.clone()).expect("experiment contract");
    let ci_run = CiEvalRun {
        id: CiEvalRunId(Uuid::now_v7()),
        workspace_id: context.workspace_id,
        dataset_id: experiment_value.dataset_id,
        dataset_name: experiment_value.dataset_name.clone(),
        experiment_id: experiment_value.id,
        status: CiEvalRunStatus::Failed,
        gate_status: RetrievalEvalGateStatus::Failed,
        branch: Some("feature/audit-reports".to_owned()),
        commit_sha: Some("abc123def456".to_owned()),
        base_ref: Some("main".to_owned()),
        head_ref: Some("feature/audit-reports".to_owned()),
        config_label: "report-api-test".to_owned(),
        regression: None,
        report: CiEvalReport {
            title: "CI audit fixture".to_owned(),
            summary: "CI gate fixture".to_owned(),
            gate: experiment_value.gate.clone(),
            failed_cases: experiment_value.failures.clone(),
            experiment: experiment_value,
        },
        created_at: OffsetDateTime::now_utc(),
    };
    context
        .store
        .save_ci_eval_run(ci_run.clone())
        .await
        .expect("save CI run");

    let trace_report = create_report(
        &context,
        "/api/v1/reports/from-trace",
        json!({ "trace_id": trace["id"] }),
    )
    .await;
    assert_eq!(trace_report["privacy_mode"], "metadata_only");
    assert!(trace_report["diagnosis"]["outcome"].is_string());
    let experiment_report = create_report(
        &context,
        "/api/v1/reports/from-experiment",
        json!({
            "experiment_id": experiment["id"],
            "privacy_mode": "snippets_allowed"
        }),
    )
    .await;
    assert_eq!(experiment_report["source"]["type"], "eval_experiment");
    let ci_report = create_report(
        &context,
        "/api/v1/reports/from-ci-run",
        json!({ "run_id": ci_run.id }),
    )
    .await;
    assert_eq!(ci_report["source"]["type"], "ci_eval_run");

    let list = get_json(&context, "/api/v1/reports").await;
    assert_eq!(list.as_array().expect("report list").len(), 3);
    let trace_report_id = trace_report["id"].as_str().expect("report id");
    let detail = get_json(&context, &format!("/api/v1/reports/{trace_report_id}")).await;
    assert_eq!(detail["id"], trace_report["id"]);

    let export = context
        .app
        .clone()
        .oneshot(empty_request(
            Method::GET,
            &format!("/api/v1/reports/{trace_report_id}/export.md"),
            Some(&context.cookie),
        ))
        .await
        .expect("Markdown response");
    assert_eq!(export.status(), StatusCode::OK);
    assert_eq!(
        export
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/markdown; charset=utf-8")
    );
    assert!(export.headers().contains_key(header::CONTENT_DISPOSITION));
    let markdown = String::from_utf8(
        to_bytes(export.into_body(), usize::MAX)
            .await
            .expect("Markdown bytes")
            .to_vec(),
    )
    .expect("Markdown text");
    assert!(markdown.contains("# RAG trace audit"));
    assert!(markdown.contains("## Deterministic Diagnosis"));
    assert!(!markdown.contains(trace["input"].as_str().expect("trace input")));

    let local_report = create_report(
        &context,
        "/api/v1/reports/from-trace",
        json!({
            "trace_id": trace["id"],
            "privacy_mode": "full_local_only"
        }),
    )
    .await;
    let local_report_id = local_report["id"].as_str().expect("local report id");
    let rejected = context
        .app
        .clone()
        .oneshot(empty_request(
            Method::GET,
            &format!("/api/v1/reports/{local_report_id}/export.md"),
            Some(&context.cookie),
        ))
        .await
        .expect("rejected export response");
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json_body(rejected).await["error"]["code"],
        "unprocessable_entity"
    );

    let missing = context
        .app
        .oneshot(empty_request(
            Method::GET,
            "/api/v1/reports/00000000-0000-0000-0000-000000000999",
            Some(&context.cookie),
        ))
        .await
        .expect("missing report response");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

async fn setup() -> TestContext {
    let store = Arc::new(MemoryStore::default());
    store
        .ensure_default_project()
        .await
        .expect("default project");
    let config = ApiConfig {
        environment: RuntimeEnvironment::Local,
        bind_addr: "127.0.0.1:0".parse().expect("valid test socket"),
        storage_backend: StorageBackend::Memory,
        database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test".to_owned(),
        web_origin: "http://127.0.0.1:5173".to_owned(),
        auth: Default::default(),
        product: ProductConfig::default(),
    };
    let user = auth::bootstrap_identity(store.as_ref(), &config.auth)
        .await
        .expect("bootstrap user");
    let app = app(AppState::new(config.clone(), store.clone()));
    let login = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/auth/login",
            json!({
                "email": config.auth.bootstrap_email,
                "password": config.auth.bootstrap_password
            }),
            None,
        ))
        .await
        .expect("login response");
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("session cookie")
        .to_owned();

    TestContext {
        app,
        store,
        cookie,
        workspace_id: user.workspace.id,
    }
}

async fn create_experiment(context: &TestContext, document_id: &str, chunk_id: &str) -> Value {
    let dataset = request_json(
        context,
        Method::POST,
        "/api/v1/eval-lab/datasets",
        json!({ "name": "Report API dataset" }),
    )
    .await;
    let dataset_id = dataset["id"].as_str().expect("dataset id");
    request_json(
        context,
        Method::POST,
        &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
        json!({
            "name": "Index publication",
            "query": "when is the index published",
            "expected_document_ids": [document_id],
            "expected_chunk_ids": [chunk_id]
        }),
    )
    .await;
    request_json(
        context,
        Method::POST,
        "/api/v1/eval-lab/experiments",
        json!({
            "dataset_id": dataset_id,
            "modes": ["lexical", "hybrid"],
            "top_k": 5
        }),
    )
    .await
}

async fn upload_text_file(context: &TestContext) -> Value {
    let boundary = "CORPUSLAB_REPORT_API_TEST";
    let content = "Index publication\nThe index is published after checksum validation.";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"target_tokens\"\r\n\r\n40\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"overlap_tokens\"\r\n\r\n0\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"files[]\"; filename=\"index.md\"\r\nContent-Type: text/markdown\r\n\r\n{content}\r\n--{boundary}--\r\n"
    );
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sources/files")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::COOKIE, &context.cookie)
        .body(Body::from(body))
        .expect("upload request");
    let response = context
        .app
        .clone()
        .oneshot(request)
        .await
        .expect("upload response");
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn create_report(context: &TestContext, uri: &str, body: Value) -> Value {
    let response = context
        .app
        .clone()
        .oneshot(json_request(Method::POST, uri, body, Some(&context.cookie)))
        .await
        .expect("create report response");
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn request_json(context: &TestContext, method: Method, uri: &str, body: Value) -> Value {
    let response = context
        .app
        .clone()
        .oneshot(json_request(method, uri, body, Some(&context.cookie)))
        .await
        .expect("JSON response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn get_json(context: &TestContext, uri: &str) -> Value {
    let response = context
        .app
        .clone()
        .oneshot(empty_request(Method::GET, uri, Some(&context.cookie)))
        .await
        .expect("GET response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

fn json_request(method: Method, uri: &str, body: Value, cookie: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    request
        .body(Body::from(body.to_string()))
        .expect("JSON request")
}

fn empty_request(method: Method, uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    request.body(Body::empty()).expect("empty request")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("JSON body")
}
