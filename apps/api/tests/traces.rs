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
async fn retrieval_run_can_be_saved_as_trace_and_rerun() {
    let app = test_app();
    upload_text_file(
        &app,
        "platform-guide.md",
        "Indexing\n- GPU workers speed up embedding refreshes.\n\nReports\n- Evidence reports explain citations.",
    )
    .await;
    index_embeddings(&app).await;
    let retrieval_body = query_retrieval(&app, json!({ "query": "gpu embedding workers" })).await;
    let run_id = retrieval_body["run"]["id"].as_str().expect("run id");

    let create_response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/traces/from-retrieval-run",
            json!({ "run_id": run_id }),
        ))
        .await
        .expect("create trace response");
    assert_eq!(create_response.status(), StatusCode::OK);
    let trace_body = json_body(create_response).await;
    let trace_id = trace_body["id"].as_str().expect("trace id");
    assert_eq!(trace_body["input"], "gpu embedding workers");
    assert_eq!(trace_body["spans"].as_array().expect("spans").len(), 4);
    assert!(trace_body["started_at"].is_string());
    assert!(trace_body["retrieval"]["run"]["created_at"].is_string());
    assert!(trace_body["diagnosis"]["outcome"].is_string());
    assert!(trace_body["diagnosis"]["recommendations"].is_array());

    let list_response = app
        .clone()
        .oneshot(empty_request(Method::GET, "/api/v1/traces"))
        .await
        .expect("list response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = json_body(list_response).await;
    assert_eq!(list_body[0]["id"], trace_id);
    assert_eq!(list_body[0]["retrieval_mode"], "hybrid");
    assert!(list_body[0]["created_at"].is_string());

    let detail_response = app
        .clone()
        .oneshot(empty_request(
            Method::GET,
            &format!("/api/v1/traces/{trace_id}"),
        ))
        .await
        .expect("detail response");
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_body = json_body(detail_response).await;
    assert_eq!(detail_body["retrieval"]["run"]["id"], run_id);
    assert!(detail_body["diagnosis"]["score_explanations"].is_array());

    let rerun_response = app
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/traces/{trace_id}/rerun"),
            json!({ "retrieval_mode": "lexical", "top_k": 3 }),
        ))
        .await
        .expect("rerun response");
    assert_eq!(rerun_response.status(), StatusCode::OK);
    let rerun_body = json_body(rerun_response).await;
    assert_eq!(
        rerun_body["comparison"]["response"]["run"]["retrieval_mode"],
        "lexical"
    );
    assert_eq!(
        rerun_body["trace"]["reruns"]
            .as_array()
            .expect("reruns")
            .len(),
        1
    );
    assert!(rerun_body["comparison"]["diagnosis"]["summary"].is_string());
}

#[tokio::test]
async fn creating_trace_without_retrieval_run_is_rejected() {
    let app = test_app();

    let response = app
        .oneshot(json_request(
            Method::POST,
            "/api/v1/traces/from-retrieval-run",
            json!({}),
        ))
        .await
        .expect("create trace response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "bad_request");
    assert!(body["error"]["message"]
        .as_str()
        .expect("error message")
        .contains("retrieval query"));
}

#[tokio::test]
async fn missing_trace_returns_not_found() {
    let app = test_app();

    let response = app
        .oneshot(empty_request(
            Method::GET,
            "/api/v1/traces/018f6ec8-ec36-7d45-a0a2-a23ee7a2c000",
        ))
        .await
        .expect("missing trace response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "not_found");
    assert!(body["error"]["message"].is_string());
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
    let boundary = "CORPUSLAB_TRACE_TEST_BOUNDARY";
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
