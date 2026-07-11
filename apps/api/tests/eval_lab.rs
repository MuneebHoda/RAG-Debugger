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
async fn eval_lab_manages_datasets_and_cases() {
    let app = test_app();
    let upload_body = upload_text_file(
        &app,
        "refund-policy.md",
        "Refund Policy\nExceptions require manager approval.",
    )
    .await;
    let document_id = upload_body["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");
    let chunk_id = upload_body["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");

    let dataset = create_dataset(&app, "Support quality").await;
    let dataset_id = dataset["id"].as_str().expect("dataset id");
    let case = create_case(
        &app,
        dataset_id,
        json!({
            "name": "Refund policy",
            "query": "refund exception",
            "expected_document_ids": [document_id],
            "expected_chunk_ids": [chunk_id, chunk_id]
        }),
    )
    .await;
    let case_id = case["id"].as_str().expect("case id");
    assert_eq!(
        case["expected_chunk_ids"].as_array().expect("chunks").len(),
        1
    );

    let evidence = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        json!({
            "query": "refund",
            "document_ids": [document_id],
            "chunk_ids": [chunk_id],
            "include_chunks": true
        }),
    )
    .await;
    assert_eq!(evidence["documents"][0]["path"], "refund-policy.md");
    assert_eq!(evidence["chunks"][0]["id"], chunk_id);
    assert!(evidence["unresolved_document_ids"]
        .as_array()
        .expect("unresolved docs")
        .is_empty());

    let detail = get_json(&app, &format!("/api/v1/eval-lab/datasets/{dataset_id}")).await;
    assert_eq!(detail["cases"].as_array().expect("cases").len(), 1);

    let updated = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({
            "name": "Refund exception policy",
            "query": "refund policy exception",
            "expected_document_ids": [document_id],
            "expected_chunk_ids": [chunk_id]
        }),
    )
    .await;
    assert_eq!(updated["name"], "Refund exception policy");

    let invalid_update = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/api/v1/eval-lab/cases/{case_id}"),
            json!({
                "expected_chunk_ids": ["018f7a2a-6e2e-7000-a000-000000000999"]
            }),
        ))
        .await
        .expect("invalid update response");
    assert_eq!(invalid_update.status(), StatusCode::BAD_REQUEST);

    let delete_response = app
        .clone()
        .oneshot(json_request(
            Method::DELETE,
            &format!("/api/v1/eval-lab/cases/{case_id}"),
            json!({}),
        ))
        .await
        .expect("delete response");
    assert_eq!(delete_response.status(), StatusCode::OK);

    let detail = get_json(&app, &format!("/api/v1/eval-lab/datasets/{dataset_id}")).await;
    assert!(detail["cases"].as_array().expect("cases").is_empty());
}

#[tokio::test]
async fn eval_lab_runs_multi_mode_experiment_with_gate() {
    let app = test_app();
    let upload_body = upload_text_file(
        &app,
        "platform-guide.md",
        "GPU Indexing\n- Local GPU workers refresh embeddings quickly.\n\nReports\n- Evidence reports explain citations.",
    )
    .await;
    let document_id = upload_body["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");
    let chunk_id = upload_body["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");
    let wrong_upload_body = upload_text_file(
        &app,
        "release-notes.md",
        "Billing\n- Annual invoices are generated through finance approval.",
    )
    .await;
    let wrong_chunk_id = wrong_upload_body["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("wrong chunk id");
    index_embeddings(&app).await;

    let dataset = create_dataset(&app, "Platform regression set").await;
    let dataset_id = dataset["id"].as_str().expect("dataset id");
    let eval_case = create_case(
        &app,
        dataset_id,
        json!({
            "name": "GPU indexing evidence",
            "query": "gpu indexing workers",
            "top_k": 5,
            "expected_chunk_ids": [chunk_id],
            "expected_document_ids": [document_id]
        }),
    )
    .await;
    let case_id = eval_case["id"].as_str().expect("case id");

    let experiment = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/experiments",
        json!({
            "dataset_id": dataset_id,
            "name": "Mode comparison",
            "modes": ["lexical", "vector", "hybrid"],
            "top_k": 5
        }),
    )
    .await;

    assert_eq!(experiment["dataset_id"], dataset_id);
    assert_eq!(
        experiment["mode_results"].as_array().expect("modes").len(),
        3
    );
    assert_eq!(experiment["gate"]["status"], "passed");
    assert_eq!(
        experiment["failures"].as_array().expect("failures").len(),
        0
    );

    let experiment_id = experiment["id"].as_str().expect("experiment id");
    let history = get_json(
        &app,
        &format!("/api/v1/eval-lab/datasets/{dataset_id}/experiments"),
    )
    .await;
    assert_eq!(history.as_array().expect("history").len(), 1);
    assert_eq!(history[0]["id"], experiment_id);

    let comparison = request_json(
        &app,
        Method::POST,
        &format!("/api/v1/eval-lab/experiments/{experiment_id}/compare"),
        json!({ "modes": ["hybrid", "lexical"] }),
    )
    .await;
    assert_eq!(comparison["mode_count"], 2);

    request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({
            "expected_chunk_ids": [wrong_chunk_id],
            "expected_document_ids": [document_id]
        }),
    )
    .await;
    let regressed = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/experiments",
        json!({
            "dataset_id": dataset_id,
            "name": "Regressed comparison",
            "modes": ["lexical", "vector", "hybrid"],
            "top_k": 5
        }),
    )
    .await;
    let regressed_id = regressed["id"].as_str().expect("regressed id");
    let regression = get_json(
        &app,
        &format!("/api/v1/eval-lab/experiments/{regressed_id}/regression"),
    )
    .await;
    assert_eq!(regression["classification"], "regressed");
    assert_eq!(regression["baseline_experiment_id"], experiment_id);
    assert!(!regression["newly_failed_cases"]
        .as_array()
        .expect("newly failed")
        .is_empty());

    let trend = get_json(
        &app,
        &format!("/api/v1/eval-lab/datasets/{dataset_id}/trends?limit=99"),
    )
    .await;
    assert_eq!(trend["window_limit"], 50);
    assert_eq!(trend["latest_experiment_id"], regressed_id);
    assert_eq!(trend["latest_regression"]["classification"], "regressed");

    let overview = get_json(&app, "/api/v1/overview").await;
    assert_eq!(overview["latest_eval_experiment"]["id"], regressed_id);
}

async fn create_dataset(app: &axum::Router, name: &str) -> Value {
    request_json(
        app,
        Method::POST,
        "/api/v1/eval-lab/datasets",
        json!({ "name": name, "description": "Regression coverage" }),
    )
    .await
}

async fn create_case(app: &axum::Router, dataset_id: &str, body: Value) -> Value {
    request_json(
        app,
        Method::POST,
        &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
        body,
    )
    .await
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
    request_json(app, Method::POST, "/api/v1/embeddings/index", json!({})).await
}

async fn get_json(app: &axum::Router, uri: &str) -> Value {
    let response = app
        .clone()
        .oneshot(empty_request(Method::GET, uri))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn request_json(app: &axum::Router, method: Method, uri: &str, body: Value) -> Value {
    let response = app
        .clone()
        .oneshot(json_request(method, uri, body))
        .await
        .expect("response");
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
    let boundary = "CORPUSLAB_EVAL_LAB_TEST_BOUNDARY";
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
