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
async fn retrieval_query_searches_all_indexed_documents() {
    let app = test_app();
    upload_text_file(
        &app,
        "resume.md",
        "Projects\n- Built GPU indexing experiments.\n\nExperience\n- Built dashboards.",
    )
    .await;
    index_embeddings(&app).await;

    let response = app
        .oneshot(json_request(json!({
            "query": "gpu indexing",
            "top_k": 5
        })))
        .await
        .expect("retrieval response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["answer"]["status"], "answered");
    assert!(body["answer"]["text"]
        .as_str()
        .expect("answer text")
        .contains("[1]"));
    assert_eq!(body["hits"][0]["rank"], 1);
    assert_eq!(body["hits"][0]["matched_terms"][0]["term"], "gpu");
    assert_eq!(body["hits"][0]["document"]["path"], "resume.md");
    assert!(body["diagnosis"]["outcome"].is_string());
    assert!(body["diagnosis"]["score_explanations"][0]["summary"].is_string());
}

#[tokio::test]
async fn retrieval_query_respects_document_filter_and_top_k() {
    let app = test_app();
    let upload_body = upload_text_file(
        &app,
        "resume.md",
        "Projects\n- Built GPU indexing experiments.",
    )
    .await;
    upload_text_file(
        &app,
        "notes.md",
        "Projects\n- Built GPU monitoring dashboards.",
    )
    .await;
    index_embeddings(&app).await;
    let document_id = upload_body["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");

    let response = app
        .oneshot(json_request(json!({
            "query": "gpu projects",
            "top_k": 1,
            "document_ids": [document_id]
        })))
        .await
        .expect("retrieval response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["run"]["top_k"], 1);
    assert_eq!(body["hits"].as_array().expect("hits").len(), 1);
    assert_eq!(body["hits"][0]["document"]["id"], document_id);
}

#[tokio::test]
async fn retrieval_query_returns_insufficient_evidence_when_no_chunks_match() {
    let app = test_app();
    upload_text_file(&app, "resume.md", "Projects\n- Built frontend dashboards.").await;
    index_embeddings(&app).await;

    let response = app
        .oneshot(json_request(json!({
            "query": "kubernetes operator",
            "top_k": 5
        })))
        .await
        .expect("retrieval response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["answer"]["status"], "insufficient_evidence");
    assert!(body["hits"].as_array().expect("hits").is_empty());
    assert_eq!(body["diagnosis"]["outcome"], "failing");
    assert_eq!(
        body["diagnosis"]["primary_issue"]["code"],
        "missing_document"
    );
}

#[tokio::test]
async fn retrieval_query_reports_missing_embeddings_for_default_hybrid_mode() {
    let app = test_app();
    upload_text_file(
        &app,
        "resume.md",
        "Projects\n- Built GPU indexing experiments.",
    )
    .await;

    let response = app
        .oneshot(json_request(json!({
            "query": "gpu indexing",
            "top_k": 5
        })))
        .await
        .expect("retrieval response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["embedding_status"]["readiness"], "missing");
    assert_eq!(body["answer"]["status"], "insufficient_evidence");
    assert!(body["answer"]["text"]
        .as_str()
        .expect("answer text")
        .contains("not indexed yet"));
}

#[tokio::test]
async fn retrieval_query_supports_explicit_lexical_mode_without_embeddings() {
    let app = test_app();
    upload_text_file(
        &app,
        "resume.md",
        "Projects\n- Built GPU indexing experiments.",
    )
    .await;

    let response = app
        .oneshot(json_request(json!({
            "query": "gpu indexing",
            "top_k": 5,
            "retrieval_mode": "lexical"
        })))
        .await
        .expect("retrieval response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["embedding_status"]["readiness"], "not_required");
    assert_eq!(body["hits"][0]["score_breakdown"]["semantic"], 0.0);
}

#[tokio::test]
async fn embedding_status_and_indexing_are_exposed() {
    let app = test_app();
    upload_text_file(
        &app,
        "resume.md",
        "Projects\n- Built GPU indexing experiments.",
    )
    .await;

    let status_before = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/embeddings/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("status response");
    let status_body = json_body(status_before).await;
    assert_eq!(status_body["total_chunks"], 1);
    assert_eq!(status_body["missing_chunks"], 1);

    let indexed_body = index_embeddings(&app).await;
    assert_eq!(indexed_body["indexed_chunks"], 1);
    assert_eq!(indexed_body["status"]["indexed_chunks"], 1);
}

#[tokio::test]
async fn retrieval_eval_cases_can_be_created_and_run() {
    let app = test_app();
    let upload_body = upload_text_file(
        &app,
        "resume.md",
        "Projects\n- Built GPU indexing experiments.",
    )
    .await;
    let chunk_id = upload_body["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");
    index_embeddings(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/retrieval/evals")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "GPU project evidence",
                        "query": "gpu acceleration",
                        "top_k": 5,
                        "expected_chunk_ids": [chunk_id]
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create eval response");
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = json_body(create_response).await;
    assert_eq!(create_body["query"], "gpu acceleration");

    let run_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/retrieval/evals/run")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "retrieval_mode": "hybrid"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("run eval response");
    assert_eq!(run_response.status(), StatusCode::OK);
    let run_body = json_body(run_response).await;

    assert_eq!(run_body["case_count"], 1);
    assert_eq!(run_body["passed_count"], 1);
    assert_eq!(run_body["results"][0]["recall_at_k"], 1.0);
}

#[tokio::test]
async fn retrieval_query_rejects_empty_query() {
    let app = test_app();
    let response = app
        .oneshot(json_request(json!({
            "query": " ",
            "top_k": 5
        })))
        .await
        .expect("retrieval response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/embeddings/index")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("embedding index response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

fn json_request(body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/retrieval/query")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

fn multipart_request(file_name: &str, content: &str) -> Request<Body> {
    let boundary = "RAG_DEBUGGER_RETRIEVAL_TEST_BOUNDARY";
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
