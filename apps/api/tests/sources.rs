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
use serde_json::Value;
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
async fn uploads_text_file_and_returns_preview_chunks() {
    let app = test_app();
    let response = app
        .oneshot(multipart_request(
            vec![("guide.md", "text/markdown", "one two three four five")],
            Some(("target_tokens", "2")),
            Some(("overlap_tokens", "0")),
            Some("whitespace"),
        ))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json_body(response).await;

    assert_eq!(body["totals"]["documents_created"], 1);
    assert_eq!(body["totals"]["chunks_created"], 3);
    assert_eq!(body["documents"][0]["status"], "success");
    assert_eq!(body["documents"][0]["preview_chunks"][0]["text"], "one two");
    assert_eq!(
        body["documents"][0]["preview_chunks"][0]["strategy"],
        "whitespace"
    );
    assert_eq!(
        body["documents"][0]["preview_chunks"][0]["split_reason"],
        "token_limit"
    );
}

#[tokio::test]
async fn defaults_to_structured_chunking() {
    let app = test_app();
    let response = app
        .oneshot(multipart_request(
            vec![(
                "product-guide.md",
                "text/markdown",
                "Overview\nBuilder of useful tools.\n\nImplementation\n- Built RAG systems.",
            )],
            Some(("target_tokens", "20")),
            Some(("overlap_tokens", "0")),
            None,
        ))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json_body(response).await;

    assert_eq!(body["source"]["chunking"]["strategy"], "structured");
    assert_eq!(
        body["documents"][0]["preview_chunks"][0]["strategy"],
        "structured"
    );
    assert_eq!(
        body["documents"][0]["preview_chunks"][0]["section_title"],
        "Overview"
    );
}

#[tokio::test]
async fn accepts_legacy_smart_section_chunking_alias() {
    let app = test_app();
    let response = app
        .oneshot(multipart_request(
            vec![(
                "system-notes.md",
                "text/markdown",
                "Implementation\n- Built GPU indexing experiments.",
            )],
            Some(("target_tokens", "20")),
            Some(("overlap_tokens", "0")),
            Some("smart_sections"),
        ))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json_body(response).await;

    assert_eq!(body["source"]["chunking"]["strategy"], "structured");
    assert_eq!(
        body["documents"][0]["preview_chunks"][0]["section_title"],
        "Implementation"
    );
}

#[tokio::test]
async fn unsupported_file_returns_structured_unprocessable_response() {
    let app = test_app();
    let response = app
        .oneshot(multipart_request(
            vec![("archive.zip", "application/zip", "not useful here")],
            None,
            None,
            None,
        ))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(response).await;

    assert_eq!(body["totals"]["documents_created"], 0);
    assert_eq!(body["totals"]["failed_files"], 1);
    assert_eq!(body["documents"][0]["error_code"], "unsupported_file_type");
}

#[tokio::test]
async fn uploaded_documents_are_listed_and_chunks_are_retrievable() {
    let app = test_app();
    let upload_response = app
        .clone()
        .oneshot(multipart_request(
            vec![("notes.txt", "text/plain", "alpha beta gamma delta")],
            Some(("target_tokens", "2")),
            Some(("overlap_tokens", "0")),
            Some("whitespace"),
        ))
        .await
        .expect("upload response");

    assert_eq!(upload_response.status(), StatusCode::CREATED);
    let upload_body = json_body(upload_response).await;
    let document_id = upload_body["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");

    let sources_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/sources")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("sources response");
    assert_eq!(sources_response.status(), StatusCode::OK);
    let sources_body = json_body(sources_response).await;
    assert_eq!(sources_body[0]["document_count"], 1);

    let chunks_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/documents/{document_id}/chunks"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("chunks response");
    assert_eq!(chunks_response.status(), StatusCode::OK);
    let chunks_body = json_body(chunks_response).await;
    assert_eq!(chunks_body.as_array().expect("chunks").len(), 2);
    assert_eq!(chunks_body[0]["strategy"], "whitespace");
}

fn multipart_request(
    files: Vec<(&str, &str, &str)>,
    target_tokens: Option<(&str, &str)>,
    overlap_tokens: Option<(&str, &str)>,
    chunking_strategy: Option<&str>,
) -> Request<Body> {
    let boundary = "RAG_DEBUGGER_TEST_BOUNDARY";
    let mut body = String::new();

    if let Some((name, value)) = target_tokens {
        push_text_part(&mut body, boundary, name, value);
    }

    if let Some((name, value)) = overlap_tokens {
        push_text_part(&mut body, boundary, name, value);
    }

    if let Some(value) = chunking_strategy {
        push_text_part(&mut body, boundary, "chunking_strategy", value);
    }

    for (file_name, content_type, content) in files {
        body.push_str(&format!("--{boundary}\r\n"));
        body.push_str(&format!(
            "Content-Disposition: form-data; name=\"files[]\"; filename=\"{file_name}\"\r\n"
        ));
        body.push_str(&format!("Content-Type: {content_type}\r\n\r\n"));
        body.push_str(content);
        body.push_str("\r\n");
    }

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
