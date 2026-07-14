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
use rag_debugger_core::{ChunkId, DocumentId, ProductConfig, RetrievalEvalCase};
use rag_debugger_storage::{memory::MemoryStore, repository::EvalRepository};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn test_app() -> axum::Router {
    test_app_with_store().0
}

fn test_app_with_store() -> (axum::Router, Arc<MemoryStore>) {
    let store = Arc::new(MemoryStore::default());
    let router = app(AppState::new(
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
        store.clone(),
    ));
    (router, store)
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
    let chunk_only = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({
            "expected_document_ids": [],
            "expected_chunk_ids": [chunk_id, chunk_id]
        }),
    )
    .await;
    assert_eq!(
        chunk_only["expected_document_ids"]
            .as_array()
            .expect("document ids")
            .len(),
        0
    );
    assert_eq!(
        chunk_only["expected_chunk_ids"]
            .as_array()
            .expect("chunk ids")
            .len(),
        1
    );
    let document_only = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({
            "expected_document_ids": [document_id, document_id],
            "expected_chunk_ids": []
        }),
    )
    .await;
    assert_eq!(
        document_only["expected_document_ids"]
            .as_array()
            .expect("document ids")
            .len(),
        1
    );
    assert_eq!(
        document_only["expected_chunk_ids"]
            .as_array()
            .expect("chunk ids")
            .len(),
        0
    );

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
async fn update_case_repairs_legacy_stale_evidence_atomically() {
    let (app, store) = test_app_with_store();
    let upload = upload_text_file(
        &app,
        "repair-guide.md",
        "Repair Guide\nUse verified evidence when repairing legacy cases.",
    )
    .await;
    let document_id = upload["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");
    let chunk_id = upload["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");
    let dataset = create_dataset(&app, "Legacy repair dataset").await;
    let dataset_id = dataset["id"].as_str().expect("dataset id");
    let empty_create = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
            json!({
                "name": "Invalid empty case",
                "query": "Creation still requires expected evidence",
                "expected_document_ids": [],
                "expected_chunk_ids": []
            }),
        ))
        .await
        .expect("empty create response");
    assert_eq!(empty_create.status(), StatusCode::BAD_REQUEST);

    let created = create_case(
        &app,
        dataset_id,
        json!({
            "name": "Legacy case",
            "query": "How are legacy cases repaired?",
            "expected_document_ids": [document_id],
            "expected_chunk_ids": [chunk_id]
        }),
    )
    .await;
    let case_id = created["id"].as_str().expect("case id").to_owned();
    let stale_chunk = ChunkId(
        Uuid::parse_str("018f7a2a-6e2e-7000-a000-000000000991").expect("valid stale chunk id"),
    );
    let stale_document = DocumentId(
        Uuid::parse_str("018f7a2a-6e2e-7000-a000-000000000992").expect("valid stale document id"),
    );
    let mut legacy: RetrievalEvalCase =
        serde_json::from_value(created).expect("deserialize created case");
    legacy.expected_chunk_ids = vec![stale_chunk];
    legacy.expected_document_ids = vec![stale_document];
    store
        .update_retrieval_eval_case(legacy.clone())
        .await
        .expect("seed legacy stale case");

    let readable = get_json(&app, &format!("/api/v1/eval-lab/datasets/{dataset_id}")).await;
    assert_eq!(
        readable["cases"][0]["expected_chunk_ids"][0],
        stale_chunk.0.to_string()
    );
    assert_eq!(
        readable["cases"][0]["expected_document_ids"][0],
        stale_document.0.to_string()
    );

    let renamed = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "name": "Renamed legacy case" }),
    )
    .await;
    assert_eq!(renamed["name"], "Renamed legacy case");
    assert_eq!(renamed["expected_chunk_ids"][0], stale_chunk.0.to_string());
    assert_eq!(
        renamed["expected_document_ids"][0],
        stale_document.0.to_string()
    );

    let noted = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "notes": "Stale evidence intentionally retained." }),
    )
    .await;
    assert_eq!(noted["notes"], "Stale evidence intentionally retained.");
    assert_eq!(noted["expected_chunk_ids"][0], stale_chunk.0.to_string());
    assert_eq!(
        noted["expected_document_ids"][0],
        stale_document.0.to_string()
    );

    let chunk_replaced = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "expected_chunk_ids": [chunk_id, chunk_id] }),
    )
    .await;
    assert_eq!(
        chunk_replaced["expected_chunk_ids"]
            .as_array()
            .expect("chunk ids")
            .len(),
        1
    );
    assert_eq!(
        chunk_replaced["expected_document_ids"][0],
        stale_document.0.to_string(),
        "an omitted stale document must not be revalidated"
    );

    let chunk_cleared = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "expected_chunk_ids": [] }),
    )
    .await;
    assert!(chunk_cleared["expected_chunk_ids"]
        .as_array()
        .expect("chunk ids")
        .is_empty());
    assert_eq!(
        chunk_cleared["expected_document_ids"][0],
        stale_document.0.to_string()
    );

    let all_cleared = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "expected_document_ids": [] }),
    )
    .await;
    assert!(all_cleared["expected_chunk_ids"]
        .as_array()
        .expect("chunk ids")
        .is_empty());
    assert!(all_cleared["expected_document_ids"]
        .as_array()
        .expect("document ids")
        .is_empty());

    store
        .update_retrieval_eval_case(legacy)
        .await
        .expect("restore legacy stale case");
    let invalid = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/api/v1/eval-lab/cases/{case_id}"),
            json!({
                "name": "Must not persist",
                "expected_chunk_ids": [stale_chunk.0, chunk_id]
            }),
        ))
        .await
        .expect("invalid stale evidence response");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let error = json_body(invalid).await;
    assert_eq!(
        error["error"]["message"],
        "bad request: Some selected evidence is unavailable. Remove or replace stale evidence before saving."
    );

    let invalid_document = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/api/v1/eval-lab/cases/{case_id}"),
            json!({
                "notes": "Must not persist",
                "expected_document_ids": [stale_document.0, document_id]
            }),
        ))
        .await
        .expect("invalid stale document response");
    assert_eq!(invalid_document.status(), StatusCode::BAD_REQUEST);

    let unchanged = get_json(&app, &format!("/api/v1/eval-lab/datasets/{dataset_id}")).await;
    assert_eq!(unchanged["cases"][0]["name"], "Legacy case");
    assert_eq!(unchanged["cases"][0]["notes"], Value::Null);
    assert_eq!(
        unchanged["cases"][0]["expected_chunk_ids"],
        json!([stale_chunk.0])
    );
    assert_eq!(
        unchanged["cases"][0]["expected_document_ids"],
        json!([stale_document.0])
    );
}

#[tokio::test]
async fn evidence_lookup_prioritizes_explicit_document_and_chunk_ids() {
    let app = test_app();
    upload_text_file(&app, "unrelated.md", "Unrelated\nBackground material.").await;
    let target = upload_text_file(
        &app,
        "target-evidence.md",
        "Target Evidence\nThe exact answer lives in this chunk.",
    )
    .await;
    let document_id = target["documents"][0]["document"]["id"]
        .as_str()
        .expect("target document id");
    let chunk_id = target["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("target chunk id");

    let evidence = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        json!({
            "document_ids": [document_id],
            "chunk_ids": [chunk_id],
            "include_chunks": true,
            "limit": 1
        }),
    )
    .await;

    assert_eq!(evidence["documents"][0]["id"], document_id);
    assert_eq!(evidence["documents"][0]["path"], "target-evidence.md");
    assert_eq!(evidence["chunks"][0]["id"], chunk_id);
    assert_eq!(evidence["chunks"][0]["document_id"], document_id);
}

#[tokio::test]
async fn eval_lab_runs_multi_mode_experiment_with_gate() {
    let app = test_app();
    let upload_body = upload_text_file_with_target(
        &app,
        "platform-guide.md",
        "GPU Indexing\nLocal GPU workers refresh embeddings quickly.\n\nRetention Policy\nArchived invoices require finance approval.",
        "8",
    )
    .await;
    let document_id = upload_body["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");
    let chunks = get_json(&app, &format!("/api/v1/documents/{document_id}/chunks")).await;
    let chunks = chunks.as_array().expect("chunks");
    assert!(chunks.len() >= 2, "test fixture should produce two chunks");
    let chunk_id = chunks
        .iter()
        .find(|chunk| {
            chunk["text"]
                .as_str()
                .is_some_and(|text| text.contains("GPU workers"))
        })
        .and_then(|chunk| chunk["id"].as_str())
        .expect("gpu chunk id");
    let wrong_chunk_id = chunks
        .iter()
        .find(|chunk| {
            chunk["text"]
                .as_str()
                .is_some_and(|text| text.contains("finance"))
        })
        .and_then(|chunk| chunk["id"].as_str())
        .expect("finance chunk id");
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
            "expected_document_ids": []
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
            "expected_document_ids": []
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
    let failures = regressed["failures"].as_array().expect("failures");
    assert!(failures.iter().any(|failure| {
        failure["label"] == "correct_document_wrong_chunk" && failure["case_id"] == case_id
    }));

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
    upload_text_file_with_target(app, file_name, content, "40").await
}

async fn upload_text_file_with_target(
    app: &axum::Router,
    file_name: &str,
    content: &str,
    target_tokens: &str,
) -> Value {
    let response = app
        .clone()
        .oneshot(multipart_request(file_name, content, target_tokens))
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

fn multipart_request(file_name: &str, content: &str, target_tokens: &str) -> Request<Body> {
    let boundary = "CORPUSLAB_EVAL_LAB_TEST_BOUNDARY";
    let mut body = String::new();

    push_text_part(&mut body, boundary, "target_tokens", target_tokens);
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
