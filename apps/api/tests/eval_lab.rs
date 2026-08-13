mod support;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use rag_debugger_core::{ChunkId, DocumentId, RetrievalEvalCase};
use rag_debugger_storage::repository::{EvalRepository, SubmittedExpectedEvidence};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn test_app() -> axum::Router {
    support::authenticated_test_app().await.router
}

#[tokio::test]
async fn eval_lab_manages_datasets_and_cases() {
    let app = test_app().await;
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
async fn imported_traces_cannot_be_copied_into_eval_lab() {
    let app = test_app().await;
    let project = get_json(&app, "/api/v1/projects/current").await;
    let upload = upload_text_file(&app, "safe-evidence.md", "Authorized local evidence.").await;
    let chunk_id = upload["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("chunk id");
    let dataset = create_dataset(&app, "Private import guard").await;
    let dataset_id = dataset["id"].as_str().expect("dataset id");
    let marker = "FULL_LOCAL_QUERY_MUST_NOT_ESCAPE";
    let ingest = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/traces/ingest",
            json!({
                "schema_version": "1",
                "project_id": project["id"],
                "external_trace_id": "full-local-eval-guard",
                "privacy_mode": "full_local_only",
                "query": marker
            }),
        ))
        .await
        .expect("ingestion response");
    assert_eq!(ingest.status(), StatusCode::CREATED);
    let trace_id = json_body(ingest).await["trace_id"]
        .as_str()
        .expect("trace id")
        .to_owned();

    let inaccessible_trace = "00000000-0000-0000-0000-000000000099";
    let inaccessible = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
            json!({
                "query": "safe query",
                "expected_chunk_ids": [chunk_id],
                "source_trace_id": inaccessible_trace
            }),
        ))
        .await
        .expect("inaccessible source response");
    assert_eq!(inaccessible.status(), StatusCode::NOT_FOUND);
    assert!(!json_body(inaccessible)
        .await
        .to_string()
        .contains(inaccessible_trace));

    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
            json!({
                "query": marker,
                "expected_chunk_ids": [chunk_id],
                "source_trace_id": trace_id
            }),
        ))
        .await
        .expect("eval guard response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = json_body(response).await;
    assert_eq!(error["error"]["code"], "imported_trace_eval_not_permitted");
    assert!(!error.to_string().contains(marker));
    assert!(
        get_json(&app, &format!("/api/v1/eval-lab/datasets/{dataset_id}")).await["cases"]
            .as_array()
            .expect("cases")
            .is_empty()
    );

    let metadata_ingest = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/traces/ingest",
            json!({
                "schema_version": "1",
                "project_id": project["id"],
                "external_trace_id": "metadata-eval-guard",
                "privacy_mode": "metadata_only"
            }),
        ))
        .await
        .expect("metadata ingestion response");
    let metadata_trace_id = json_body(metadata_ingest).await["trace_id"]
        .as_str()
        .expect("metadata trace id")
        .to_owned();
    let metadata_response = app
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
            json!({
                "query": "separate safe query",
                "expected_chunk_ids": [chunk_id],
                "source_trace_id": metadata_trace_id
            }),
        ))
        .await
        .expect("metadata eval guard response");
    assert_eq!(metadata_response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json_body(metadata_response).await["error"]["code"],
        "imported_trace_eval_not_permitted"
    );
}

#[tokio::test]
async fn evidence_lookup_separates_requested_items_from_bounded_candidates() {
    let app = test_app().await;
    let _alpha = upload_text_file(&app, "aaa-indexing.md", "Indexing candidate alpha.").await;
    let beta = upload_text_file(&app, "bbb-indexing.md", "Indexing candidate beta.").await;
    let _gamma = upload_text_file(&app, "ccc-indexing.md", "Indexing candidate gamma.").await;
    let selected = upload_text_file(
        &app,
        "zzz-indexing.md",
        &format!("Indexing selected evidence {}", "é".repeat(400)),
    )
    .await;
    let selected_document_id = selected["documents"][0]["document"]["id"]
        .as_str()
        .expect("selected document id");
    let selected_chunk_id = selected["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("selected chunk id");
    let beta_document_id = beta["documents"][0]["document"]["id"]
        .as_str()
        .expect("beta document id");
    let beta_chunk_id = beta["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("beta chunk id");
    let missing_document_id = "018f7a2a-6e2e-7000-a000-000000000991";
    let missing_chunk_id = "018f7a2a-6e2e-7000-a000-000000000992";

    let request = json!({
        "query": "indexing",
        "document_ids": [selected_document_id, beta_document_id, selected_document_id, missing_document_id],
        "chunk_ids": [selected_chunk_id, beta_chunk_id, selected_chunk_id, missing_chunk_id],
        "document_limit": 2,
        "chunk_limit": 2,
        "include_chunks": true
    });
    let first = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        request.clone(),
    )
    .await;
    let second = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        request,
    )
    .await;
    assert_eq!(first, second, "evidence ordering must be stable");

    let documents = first["documents"].as_array().expect("documents");
    let chunks = first["chunks"].as_array().expect("chunks");
    assert_eq!(
        documents.len(),
        4,
        "two requested items plus two candidates"
    );
    assert_eq!(chunks.len(), 4, "two requested items plus two candidates");
    assert_eq!(documents[0]["id"], selected_document_id);
    assert_eq!(documents[1]["id"], beta_document_id);
    assert_eq!(chunks[0]["id"], selected_chunk_id);
    assert_eq!(chunks[1]["id"], beta_chunk_id);
    assert_eq!(documents[2]["path"], "aaa-indexing.md");
    assert_eq!(documents[3]["path"], "ccc-indexing.md");
    assert_eq!(
        documents
            .iter()
            .filter(|document| document["id"] == selected_document_id)
            .count(),
        1
    );
    assert_eq!(
        chunks
            .iter()
            .filter(|chunk| chunk["id"] == selected_chunk_id)
            .count(),
        1
    );
    assert_eq!(
        first["unresolved_document_ids"],
        json!([missing_document_id])
    );
    assert_eq!(first["unresolved_chunk_ids"], json!([missing_chunk_id]));

    let preview = chunks[0]["text_preview"].as_str().expect("chunk preview");
    assert_eq!(preview.chars().count(), 280);
    assert_eq!(chunks[0]["preview_truncated"], true);
    assert!(preview.is_char_boundary(preview.len()));

    let direct_chunk_only = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        json!({
            "document_ids": [],
            "chunk_ids": [selected_chunk_id],
            "document_limit": 0,
            "chunk_limit": 100,
            "include_chunks": false
        }),
    )
    .await;
    assert!(direct_chunk_only["documents"]
        .as_array()
        .expect("direct documents")
        .is_empty());
    assert_eq!(
        direct_chunk_only["chunks"]
            .as_array()
            .expect("direct chunks")
            .len(),
        1
    );
    assert_eq!(direct_chunk_only["chunks"][0]["id"], selected_chunk_id);

    let legacy_limit = request_json(
        &app,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        json!({ "query": "indexing", "limit": 1, "include_chunks": true }),
    )
    .await;
    assert_eq!(
        legacy_limit["documents"]
            .as_array()
            .expect("documents")
            .len(),
        1
    );
    assert_eq!(legacy_limit["chunks"].as_array().expect("chunks").len(), 1);
}

#[tokio::test]
async fn evidence_lookup_and_case_mutations_enforce_request_work_limits() {
    let app = test_app().await;
    let repeated_document_id = "018f7a2a-6e2e-7000-a000-000000000901";
    let repeated_chunk_id = "018f7a2a-6e2e-7000-a000-000000000902";

    let short_query = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/eval-lab/evidence/query",
            json!({ "query": "ab", "document_limit": 1 }),
        ))
        .await
        .expect("short-query response");
    assert_eq!(short_query.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(short_query).await["error"]["message"],
        "bad request: Enter at least 3 characters, paste an exact UUID, or leave blank to browse."
    );

    let oversized_lookup = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/eval-lab/evidence/query",
            json!({
                "document_ids": vec![repeated_document_id; 101],
                "document_limit": 0,
                "chunk_limit": 0
            }),
        ))
        .await
        .expect("oversized lookup response");
    assert_eq!(oversized_lookup.status(), StatusCode::BAD_REQUEST);

    let combined_lookup = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/eval-lab/evidence/query",
            json!({
                "document_ids": vec![repeated_document_id; 100],
                "chunk_ids": vec![repeated_chunk_id; 151],
                "document_limit": 0,
                "chunk_limit": 0
            }),
        ))
        .await
        .expect("combined lookup response");
    assert_eq!(combined_lookup.status(), StatusCode::BAD_REQUEST);

    let dataset = create_dataset(&app, "Bounded evidence mutations").await;
    let dataset_id = dataset["id"].as_str().expect("dataset id");
    let oversized_create = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
            json!({
                "query": "bounded create",
                "expected_document_ids": vec![repeated_document_id; 101]
            }),
        ))
        .await
        .expect("oversized create response");
    assert_eq!(oversized_create.status(), StatusCode::BAD_REQUEST);

    let upload = upload_text_file(&app, "bounded-case.md", "Bounded evidence case.").await;
    let document_id = upload["documents"][0]["document"]["id"]
        .as_str()
        .expect("document id");
    let eval_case = create_case(
        &app,
        dataset_id,
        json!({
            "query": "bounded update",
            "expected_document_ids": [document_id]
        }),
    )
    .await;
    let case_id = eval_case["id"].as_str().expect("case id");
    let oversized_update = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            &format!("/api/v1/eval-lab/cases/{case_id}"),
            json!({ "expected_chunk_ids": vec![repeated_chunk_id; 251] }),
        ))
        .await
        .expect("oversized update response");
    assert_eq!(oversized_update.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_case_repairs_legacy_stale_evidence_atomically() {
    let context = support::authenticated_test_app().await;
    let app = context.router;
    let store = context.store;
    let workspace_id = context.workspace_id;
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
    legacy.notes = Some("Original legacy note.".to_owned());
    store
        .update_retrieval_eval_case(
            workspace_id,
            legacy.clone(),
            SubmittedExpectedEvidence::default(),
        )
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
    assert_eq!(readable["cases"][0]["notes"], "Original legacy note.");

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
    assert_eq!(renamed["notes"], "Original legacy note.");

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

    let cleared_notes = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "notes": null }),
    )
    .await;
    assert_eq!(cleared_notes["notes"], Value::Null);
    assert_eq!(
        cleared_notes["expected_chunk_ids"][0],
        stale_chunk.0.to_string()
    );
    assert_eq!(
        cleared_notes["expected_document_ids"][0],
        stale_document.0.to_string()
    );

    let protected_notes = request_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{case_id}"),
        json!({ "notes": "Protected note." }),
    )
    .await;
    assert_eq!(protected_notes["notes"], "Protected note.");

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

    legacy.notes = Some("Protected note.".to_owned());
    store
        .update_retrieval_eval_case(workspace_id, legacy, SubmittedExpectedEvidence::default())
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
                "notes": null,
                "expected_document_ids": [stale_document.0, document_id]
            }),
        ))
        .await
        .expect("invalid stale document response");
    assert_eq!(invalid_document.status(), StatusCode::BAD_REQUEST);

    let unchanged = get_json(&app, &format!("/api/v1/eval-lab/datasets/{dataset_id}")).await;
    assert_eq!(unchanged["cases"][0]["name"], "Legacy case");
    assert_eq!(unchanged["cases"][0]["notes"], "Protected note.");
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
    let app = test_app().await;
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
    let app = test_app().await;
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
