mod support;

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use rag_debugger_api::{app, auth, state::AppState};
use rag_debugger_core::{
    Organization, OrganizationId, User, UserId, Workspace, WorkspaceId, WorkspaceRole,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    repository::{AuthRepository, ProjectRepository},
};
use serde_json::{json, Value};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

struct WorkspaceSession {
    workspace_id: WorkspaceId,
    cookie: String,
}

#[tokio::test]
async fn evidence_and_eval_resources_are_isolated_by_workspace() {
    let store = Arc::new(MemoryStore::default());
    let config = support::test_config();
    let workspace_a = create_workspace_session(store.as_ref(), &config, "Workspace Alpha").await;
    let workspace_b =
        create_workspace_session(store.as_ref(), &config, "Workspace Beta Secret").await;
    let app = app(AppState::new(config, store));

    let beta_upload = upload(
        &app,
        &workspace_b.cookie,
        "beta-private-policy.md",
        "# Private Retention Section\nBeta-only retention evidence marker.",
    )
    .await;
    let beta_document_id = beta_upload["documents"][0]["document"]["id"]
        .as_str()
        .expect("beta document id");
    let beta_chunk_id = beta_upload["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("beta chunk id");

    let alpha_upload = upload(
        &app,
        &workspace_a.cookie,
        "alpha-public-guide.md",
        "# Account Recovery\nAlpha recovery links expire after fifteen minutes.",
    )
    .await;
    let alpha_document_id = alpha_upload["documents"][0]["document"]["id"]
        .as_str()
        .expect("alpha document id");
    let alpha_chunk_id = alpha_upload["documents"][0]["preview_chunks"][0]["id"]
        .as_str()
        .expect("alpha chunk id");

    for query in [
        "beta-private-policy",
        "Workspace Beta Secret",
        "Private Retention Section",
        "Beta-only retention evidence",
    ] {
        let response = json_response(
            &app,
            &workspace_a.cookie,
            Method::POST,
            "/api/v1/eval-lab/evidence/query",
            json!({"query": query, "include_chunks": true}),
        )
        .await;
        assert_eq!(response.0, StatusCode::OK);
        assert!(
            response.1["documents"]
                .as_array()
                .expect("documents")
                .is_empty(),
            "workspace A must not search workspace B documents for {query}"
        );
        assert!(
            response.1["chunks"].as_array().expect("chunks").is_empty(),
            "workspace A must not search workspace B chunks for {query}"
        );
    }

    let inaccessible = json_response(
        &app,
        &workspace_a.cookie,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        json!({
            "document_ids": [beta_document_id],
            "chunk_ids": [beta_chunk_id],
            "document_limit": 0,
            "chunk_limit": 0,
            "include_chunks": true
        }),
    )
    .await;
    assert_eq!(inaccessible.0, StatusCode::OK);
    assert_eq!(
        inaccessible.1["unresolved_document_ids"],
        json!([beta_document_id])
    );
    assert_eq!(
        inaccessible.1["unresolved_chunk_ids"],
        json!([beta_chunk_id])
    );
    assert!(inaccessible.1["documents"]
        .as_array()
        .expect("documents")
        .is_empty());
    assert!(inaccessible.1["chunks"]
        .as_array()
        .expect("chunks")
        .is_empty());

    let missing_document_id = Uuid::now_v7().to_string();
    let missing_chunk_id = Uuid::now_v7().to_string();
    let nonexistent = json_response(
        &app,
        &workspace_a.cookie,
        Method::POST,
        "/api/v1/eval-lab/evidence/query",
        json!({
            "document_ids": [missing_document_id],
            "chunk_ids": [missing_chunk_id],
            "document_limit": 0,
            "chunk_limit": 0,
            "include_chunks": true
        }),
    )
    .await;
    assert_eq!(nonexistent.0, inaccessible.0);
    assert_eq!(
        inaccessible.1["documents"].as_array().map(Vec::len),
        nonexistent.1["documents"].as_array().map(Vec::len)
    );
    assert_eq!(
        inaccessible.1["chunks"].as_array().map(Vec::len),
        nonexistent.1["chunks"].as_array().map(Vec::len)
    );

    let alpha_dataset = create_dataset(&app, &workspace_a.cookie, "Alpha quality").await;
    let beta_dataset = create_dataset(&app, &workspace_b.cookie, "Beta quality").await;
    let alpha_dataset_id = alpha_dataset["id"].as_str().expect("alpha dataset id");
    let beta_dataset_id = beta_dataset["id"].as_str().expect("beta dataset id");
    let beta_case = create_case(
        &app,
        &workspace_b.cookie,
        beta_dataset_id,
        json!({
            "name": "Private retention",
            "query": "beta retention",
            "expected_document_ids": [beta_document_id],
            "expected_chunk_ids": [beta_chunk_id]
        }),
    )
    .await;
    let beta_case_id = beta_case["id"].as_str().expect("beta case id");
    let alpha_case = create_case(
        &app,
        &workspace_a.cookie,
        alpha_dataset_id,
        json!({
            "name": "Recovery evidence",
            "query": "account recovery links",
            "expected_document_ids": [alpha_document_id],
            "expected_chunk_ids": [alpha_chunk_id]
        }),
    )
    .await;
    let alpha_case_id = alpha_case["id"].as_str().expect("alpha case id");

    let beta_experiment = json_response(
        &app,
        &workspace_b.cookie,
        Method::POST,
        "/api/v1/eval-lab/experiments",
        json!({
            "dataset_id": beta_dataset_id,
            "name": "Beta experiment",
            "modes": ["lexical"],
            "top_k": 5
        }),
    )
    .await;
    assert_eq!(beta_experiment.0, StatusCode::OK);
    let beta_experiment_id = beta_experiment.1["id"]
        .as_str()
        .expect("beta experiment id");

    for uri in [
        format!("/api/v1/eval-lab/datasets/{beta_dataset_id}"),
        format!("/api/v1/eval-lab/experiments/{beta_experiment_id}"),
    ] {
        let response = empty_response(&app, &workspace_a.cookie, Method::GET, &uri).await;
        assert_eq!(response.0, StatusCode::NOT_FOUND);
        assert_eq!(response.1["error"]["code"], "not_found");
    }

    let cross_case_update = json_response(
        &app,
        &workspace_a.cookie,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{beta_case_id}"),
        json!({"name": "Must remain private"}),
    )
    .await;
    assert_eq!(cross_case_update.0, StatusCode::NOT_FOUND);
    let cross_case_delete = empty_response(
        &app,
        &workspace_a.cookie,
        Method::DELETE,
        &format!("/api/v1/eval-lab/cases/{beta_case_id}"),
    )
    .await;
    assert_eq!(cross_case_delete.0, StatusCode::NOT_FOUND);

    let cross_run = json_response(
        &app,
        &workspace_a.cookie,
        Method::POST,
        "/api/v1/eval-lab/experiments",
        json!({"dataset_id": beta_dataset_id, "modes": ["lexical"], "top_k": 5}),
    )
    .await;
    assert_eq!(cross_run.0, StatusCode::NOT_FOUND);

    let cross_create = json_response(
        &app,
        &workspace_a.cookie,
        Method::POST,
        &format!("/api/v1/eval-lab/datasets/{alpha_dataset_id}/cases"),
        json!({
            "name": "Cross workspace evidence",
            "query": "private beta evidence",
            "expected_document_ids": [beta_document_id],
            "expected_chunk_ids": [beta_chunk_id]
        }),
    )
    .await;
    let missing_create = json_response(
        &app,
        &workspace_a.cookie,
        Method::POST,
        &format!("/api/v1/eval-lab/datasets/{alpha_dataset_id}/cases"),
        json!({
            "name": "Missing evidence",
            "query": "missing evidence",
            "expected_document_ids": [Uuid::now_v7()],
            "expected_chunk_ids": [Uuid::now_v7()]
        }),
    )
    .await;
    assert_eq!(cross_create.0, StatusCode::BAD_REQUEST);
    assert_eq!(cross_create.0, missing_create.0);
    assert_eq!(
        cross_create.1["error"]["message"],
        missing_create.1["error"]["message"]
    );

    let cross_update = json_response(
        &app,
        &workspace_a.cookie,
        Method::PATCH,
        &format!("/api/v1/eval-lab/cases/{alpha_case_id}"),
        json!({
            "name": "Must stay atomic",
            "expected_document_ids": [beta_document_id],
            "expected_chunk_ids": [beta_chunk_id]
        }),
    )
    .await;
    assert_eq!(cross_update.0, StatusCode::BAD_REQUEST);
    let alpha_detail = empty_response(
        &app,
        &workspace_a.cookie,
        Method::GET,
        &format!("/api/v1/eval-lab/datasets/{alpha_dataset_id}"),
    )
    .await;
    assert_eq!(alpha_detail.0, StatusCode::OK);
    let persisted = alpha_detail.1["cases"]
        .as_array()
        .expect("alpha cases")
        .iter()
        .find(|case| case["id"] == alpha_case_id)
        .expect("persisted alpha case");
    assert_eq!(persisted["name"], "Recovery evidence");
    assert_eq!(
        persisted["expected_document_ids"],
        json!([alpha_document_id])
    );
    assert_eq!(persisted["expected_chunk_ids"], json!([alpha_chunk_id]));

    assert_ne!(workspace_a.workspace_id, workspace_b.workspace_id);
}

async fn create_workspace_session(
    store: &MemoryStore,
    config: &rag_debugger_api::config::ApiConfig,
    name: &str,
) -> WorkspaceSession {
    let now = OffsetDateTime::now_utc();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: format!("{name} organization"),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: name.to_owned(),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: format!("{}@example.test", Uuid::now_v7()),
        name: format!("{name} user"),
        created_at: now,
    };
    let authenticated = store
        .create_user_workspace(
            organization,
            workspace.clone(),
            user,
            WorkspaceRole::Owner,
            "unused-test-password-hash".to_owned(),
        )
        .await
        .expect("create test workspace");
    store
        .ensure_default_project(workspace.id)
        .await
        .expect("create workspace project");
    let session = auth::create_session(store, &authenticated, &config.auth)
        .await
        .expect("create test session");
    WorkspaceSession {
        workspace_id: workspace.id,
        cookie: format!("{}={}", session.name, session.value),
    }
}

async fn create_dataset(app: &axum::Router, cookie: &str, name: &str) -> Value {
    let response = json_response(
        app,
        cookie,
        Method::POST,
        "/api/v1/eval-lab/datasets",
        json!({"name": name}),
    )
    .await;
    assert_eq!(response.0, StatusCode::OK);
    response.1
}

async fn create_case(app: &axum::Router, cookie: &str, dataset_id: &str, body: Value) -> Value {
    let response = json_response(
        app,
        cookie,
        Method::POST,
        &format!("/api/v1/eval-lab/datasets/{dataset_id}/cases"),
        body,
    )
    .await;
    assert_eq!(response.0, StatusCode::OK);
    response.1
}

async fn upload(app: &axum::Router, cookie: &str, file_name: &str, content: &str) -> Value {
    let boundary = "CORPUSLAB_WORKSPACE_ISOLATION_BOUNDARY";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"files[]\"; filename=\"{file_name}\"\r\nContent-Type: text/markdown\r\n\r\n{content}\r\n--{boundary}--\r\n"
    );
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sources/files")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::COOKIE, cookie)
        .body(Body::from(body))
        .expect("upload request");
    let response = app.clone().oneshot(request).await.expect("upload response");
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn json_response(
    app: &axum::Router,
    cookie: &str,
    method: Method,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_string()))
        .expect("JSON request");
    response_json(app, request).await
}

async fn empty_response(
    app: &axum::Router,
    cookie: &str,
    method: Method,
    uri: &str,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .expect("empty request");
    response_json(app, request).await
}

async fn response_json(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("API response");
    let status = response.status();
    (status, json_body(response).await)
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("JSON response")
}
