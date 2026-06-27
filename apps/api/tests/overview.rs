use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use rag_debugger_api::{
    app,
    config::{ApiConfig, RuntimeEnvironment, StorageBackend},
    state::AppState,
};
use rag_debugger_core::ProductConfig;
use rag_debugger_storage::memory::MemoryStore;
use serde_json::{json, Value};
use tower::ServiceExt;

fn test_app() -> axum::Router {
    app(AppState::new(
        ApiConfig {
            environment: RuntimeEnvironment::Test,
            bind_addr: "127.0.0.1:0".parse().expect("valid test socket"),
            storage_backend: StorageBackend::Memory,
            database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test"
                .to_owned(),
            web_origin: "http://127.0.0.1:5173".to_owned(),
            auth: Default::default(),
            product: ProductConfig::default(),
        },
        Arc::new(MemoryStore::default()),
    ))
}

#[tokio::test]
async fn overview_empty_state_recommends_ingestion() {
    let app = test_app();

    let response = app
        .oneshot(empty_request(Method::GET, "/api/v1/overview"))
        .await
        .expect("overview response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["health"]["status"], "needs_documents");
    assert_eq!(body["health"]["score"], 0);
    assert_eq!(
        body["health"]["primary_action"]["route"],
        json!("/app/sources")
    );
    assert!(contains_id(&body["issues"], "no_documents"));
}

#[tokio::test]
async fn overview_with_documents_and_missing_embeddings_recommends_indexing() {
    let app = test_app();
    upload_text_file(
        &app,
        "platform-guide.md",
        "Retrieval\n- Corpus teams inspect evidence and citations.",
    )
    .await;

    let response = app
        .oneshot(empty_request(Method::GET, "/api/v1/overview"))
        .await
        .expect("overview response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["health"]["status"], "needs_indexing");
    assert_eq!(
        body["health"]["primary_action"]["route"],
        json!("/app/retrieval")
    );
    assert!(contains_id(&body["issues"], "missing_embeddings"));
    assert_eq!(metric_value(&body, "documents"), Some("1"));
}

#[tokio::test]
async fn overview_populated_state_summarizes_workflow_activity() {
    let app = test_app();
    let upload_body = upload_text_file(
        &app,
        "operations-guide.md",
        "GPU Indexing\n- Local workers refresh embeddings quickly.\n\nEvidence Reports\n- Teams review citations and weak traces.",
    )
    .await;
    let chunk_id = upload_body["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");
    index_embeddings(&app).await;

    let retrieval_body = query_retrieval(&app, json!({ "query": "gpu indexing workers" })).await;
    let run_id = retrieval_body["run"]["id"].as_str().expect("run id");
    save_trace(&app, run_id).await;
    create_eval_case(&app, chunk_id).await;
    run_evals(&app).await;

    let response = app
        .oneshot(empty_request(Method::GET, "/api/v1/overview"))
        .await
        .expect("overview response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["health"]["status"], "ready");
    assert_eq!(body["embedding_status"]["missing_chunks"], 0);
    assert_eq!(metric_value(&body, "traces"), Some("1"));
    assert_eq!(metric_value(&body, "evals"), Some("1"));
    assert!(pipeline_step_status(&body, "embed") == Some("complete"));
    assert!(pipeline_step_status(&body, "trace") == Some("complete"));
    assert!(pipeline_step_status(&body, "eval") == Some("complete"));
    assert!(!body["recent_activity"]
        .as_array()
        .expect("activity")
        .is_empty());
    assert_eq!(body["latest_eval_run"]["passed_count"], 1);
}

async fn upload_text_file(app: &axum::Router, file_name: &str, content: &str) -> Value {
    let response = app
        .clone()
        .oneshot(multipart_request(file_name, content))
        .await
        .expect("upload response");
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn index_embeddings(app: &axum::Router) -> Value {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/embeddings/index",
            json!({}),
        ))
        .await
        .expect("embedding index response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn query_retrieval(app: &axum::Router, body: Value) -> Value {
    let response = app
        .clone()
        .oneshot(json_request(Method::POST, "/api/v1/retrieval/query", body))
        .await
        .expect("retrieval response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn save_trace(app: &axum::Router, run_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/traces/from-retrieval-run",
            json!({ "run_id": run_id }),
        ))
        .await
        .expect("trace response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn create_eval_case(app: &axum::Router, chunk_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/retrieval/evals",
            json!({
                "name": "GPU worker evidence",
                "query": "gpu indexing workers",
                "top_k": 5,
                "expected_chunk_ids": [chunk_id]
            }),
        ))
        .await
        .expect("eval create response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn run_evals(app: &axum::Router) -> Value {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/retrieval/evals/run",
            json!({ "retrieval_mode": "hybrid" }),
        ))
        .await
        .expect("eval run response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
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

fn multipart_request(file_name: &str, content: &str) -> Request<Body> {
    let boundary = "CORPUSLAB_OVERVIEW_TEST_BOUNDARY";
    let mut body = String::new();

    push_text_part(&mut body, boundary, "target_tokens", "40");
    push_text_part(&mut body, boundary, "overlap_tokens", "0");
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
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("request")
}

fn push_text_part(body: &mut String, boundary: &str, name: &str, value: &str) {
    body.push_str(&format!("--{boundary}\r\n"));
    body.push_str(&format!(
        "Content-Disposition: form-data; name=\"{name}\"\r\n\r\n"
    ));
    body.push_str(value);
    body.push_str("\r\n");
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

fn contains_id(items: &Value, id: &str) -> bool {
    items
        .as_array()
        .expect("array")
        .iter()
        .any(|item| item["id"] == id)
}

fn metric_value<'a>(body: &'a Value, id: &str) -> Option<&'a str> {
    body["metrics"]
        .as_array()?
        .iter()
        .find(|metric| metric["id"] == id)?
        .get("value")?
        .as_str()
}

fn pipeline_step_status<'a>(body: &'a Value, id: &str) -> Option<&'a str> {
    body["pipeline"]
        .as_array()?
        .iter()
        .find(|step| step["id"] == id)?
        .get("status")?
        .as_str()
}
