mod support;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use opentelemetry_proto::tonic::{
    collector::trace::v1::{ExportTraceServiceRequest, ExportTraceServiceResponse},
    common::v1::{any_value::Value as OtlpValue, AnyValue, KeyValue},
    trace::v1::{ResourceSpans, ScopeSpans, Span},
};
use prost::Message;
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn native_session_ingestion_is_private_idempotent_and_inspectable() {
    let app = support::authenticated_test_app().await;
    let project = json_response(
        app.router
            .clone()
            .oneshot(request(
                Method::GET,
                "/api/v1/projects/current",
                Body::empty(),
                None,
            ))
            .await
            .expect("project response"),
    )
    .await;
    let project_id = project["id"].as_str().expect("project id");
    let malformed = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/traces/ingest")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{SECRET_MALFORMED"))
                .expect("malformed native request"),
        )
        .await
        .expect("malformed native response");
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    let malformed = json_body(malformed).await;
    assert_eq!(malformed["error"]["code"], "malformed_json");
    assert!(!malformed.to_string().contains("SECRET_MALFORMED"));
    let payload = json!({
        "schema_version": "1",
        "project_id": project_id,
        "external_trace_id": "native-test-1",
        "privacy_mode": "metadata_only",
        "query": "SECRET_QUERY_MARKER",
        "answer": "SECRET_ANSWER_MARKER",
        "retrieval_mode": "hybrid",
        "top_k": 1,
        "retrieved_evidence": [{
            "external_chunk_id": "chunk-1",
            "document_label": "SECRET_PATH_MARKER",
            "rank": 1,
            "score": 0.9,
            "lexical_score": 0.8,
            "semantic_score": 0.9,
            "citation_label": "E1",
            "snippet": "SECRET_SNIPPET_MARKER"
        }],
        "spans": [],
        "failure_labels": ["weak_evidence"]
    });
    let first = app
        .router
        .clone()
        .oneshot(json_request("/api/v1/traces/ingest", &payload, None))
        .await
        .expect("first ingestion");
    let first_status = first.status();
    let first_bytes = to_bytes(first.into_body(), 1_048_576)
        .await
        .expect("first body");
    assert_eq!(
        first_status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&first_bytes)
    );
    let first: Value = serde_json::from_slice(&first_bytes).expect("first JSON");
    let trace_id = first["trace_id"].as_str().expect("trace id");
    assert_eq!(first["disposition"], "created");

    let retry = app
        .router
        .clone()
        .oneshot(json_request("/api/v1/traces/ingest", &payload, None))
        .await
        .expect("retry ingestion");
    assert_eq!(retry.status(), StatusCode::OK);
    assert_eq!(json_response(retry).await["disposition"], "unchanged");

    let detail = json_response(
        app.router
            .clone()
            .oneshot(request(
                Method::GET,
                &format!("/api/v1/traces/{trace_id}"),
                Body::empty(),
                None,
            ))
            .await
            .expect("trace detail"),
    )
    .await;
    let serialized = detail.to_string();
    for marker in [
        "SECRET_QUERY_MARKER",
        "SECRET_ANSWER_MARKER",
        "SECRET_PATH_MARKER",
        "SECRET_SNIPPET_MARKER",
    ] {
        assert!(!serialized.contains(marker));
    }
    assert_eq!(detail["ingestion"]["source"], "native");
    assert_eq!(detail["ingestion"]["mapping_status"], "partially_mapped");

    let oversized = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/traces/ingest")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(vec![b'x'; 1_048_577]))
                .expect("oversized native request"),
        )
        .await
        .expect("oversized response");
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        json_body(oversized).await["error"]["code"],
        "payload_limit_exceeded"
    );

    let inaccessible_project = "00000000-0000-0000-0000-000000000099";
    let mut inaccessible = payload;
    inaccessible["project_id"] = json!(inaccessible_project);
    let response = app
        .router
        .oneshot(json_request("/api/v1/traces/ingest", &inaccessible, None))
        .await
        .expect("inaccessible project response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(!json_body(response)
        .await
        .to_string()
        .contains(inaccessible_project));
}

#[tokio::test]
async fn trace_key_authorizes_protobuf_otlp_and_invalid_bearer_never_uses_session() {
    let app = support::authenticated_test_app().await;
    let project = json_response(
        app.router
            .clone()
            .oneshot(request(
                Method::GET,
                "/api/v1/projects/current",
                Body::empty(),
                None,
            ))
            .await
            .expect("project response"),
    )
    .await;
    let project_id = project["id"].as_str().expect("project id");
    let key = json_response(
        app.router
            .clone()
            .oneshot(json_request(
                "/api/v1/api-keys",
                &json!({"name":"OTLP test", "scopes":["trace_ingest"]}),
                None,
            ))
            .await
            .expect("key response"),
    )
    .await;
    let secret = key["secret"].as_str().expect("secret");

    let invalid = app
        .router
        .clone()
        .oneshot(json_request(
            "/api/v1/traces/ingest",
            &json!({"schema_version":"1","project_id":project_id,"external_trace_id":"invalid-key","privacy_mode":"metadata_only"}),
            Some("Bearer invalid"),
        ))
        .await
        .expect("invalid bearer response");
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);

    let request_body = hex::decode(include_str!("fixtures/otlp-python-sdk-1.41.1.pb.hex").trim())
        .expect("versioned OTLP fixture");
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/otel/v1/traces")
                .header(header::CONTENT_TYPE, "application/x-protobuf")
                .header(header::AUTHORIZATION, format!("Bearer {secret}"))
                .header("x-corpuslab-project-id", project_id)
                .body(Body::from(request_body))
                .expect("OTLP request"),
        )
        .await
        .expect("OTLP response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/x-protobuf"
    );
    let body = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("OTLP body");
    let decoded = ExportTraceServiceResponse::decode(body).expect("OTLP response protobuf");
    assert!(decoded.partial_success.is_none());
    let fixture_traces = json_response(
        app.router
            .clone()
            .oneshot(request(Method::GET, "/api/v1/traces", Body::empty(), None))
            .await
            .expect("fixture trace list"),
    )
    .await;
    let fixture_trace_id = fixture_traces[0]["id"].as_str().expect("fixture trace id");
    let fixture_detail = json_response(
        app.router
            .clone()
            .oneshot(request(
                Method::GET,
                &format!("/api/v1/traces/{fixture_trace_id}"),
                Body::empty(),
                None,
            ))
            .await
            .expect("fixture trace detail"),
    )
    .await;
    assert_eq!(
        fixture_detail["ingestion"]["service_name"],
        "corpuslab-rag-demo"
    );
    assert_eq!(
        fixture_detail["ingestion"]["instrumentation_scope_name"],
        "corpuslab.trace-ingestion.example"
    );

    let mut partial_request = otlp_request();
    let invalid_span = Span {
        trace_id: Vec::new(),
        span_id: vec![3; 8],
        ..Span::default()
    };
    partial_request.resource_spans[0].scope_spans[0]
        .spans
        .push(invalid_span);
    let partial = app
        .router
        .clone()
        .oneshot(otlp_http_request(
            partial_request.encode_to_vec(),
            secret,
            project_id,
            None,
        ))
        .await
        .expect("partial OTLP response");
    assert_eq!(partial.status(), StatusCode::OK);
    let partial = ExportTraceServiceResponse::decode(
        to_bytes(partial.into_body(), 1_048_576)
            .await
            .expect("partial body"),
    )
    .expect("partial response protobuf");
    assert_eq!(
        partial
            .partial_success
            .expect("partial success details")
            .rejected_spans,
        1
    );

    let empty = app
        .router
        .clone()
        .oneshot(otlp_http_request(
            ExportTraceServiceRequest::default().encode_to_vec(),
            secret,
            project_id,
            None,
        ))
        .await
        .expect("empty OTLP response");
    assert_eq!(empty.status(), StatusCode::OK);

    let traces = json_response(
        app.router
            .oneshot(request(Method::GET, "/api/v1/traces", Body::empty(), None))
            .await
            .expect("trace list"),
    )
    .await;
    assert_eq!(traces[0]["ingestion_source"], "otlp_http");
    assert_eq!(traces[0]["mapping_status"], "partially_mapped");
}

#[tokio::test]
async fn ingestion_rejects_wrong_scopes_revoked_keys_and_unsupported_transports() {
    let app = support::authenticated_test_app().await;
    let project = json_response(
        app.router
            .clone()
            .oneshot(request(
                Method::GET,
                "/api/v1/projects/current",
                Body::empty(),
                None,
            ))
            .await
            .expect("project response"),
    )
    .await;
    let project_id = project["id"].as_str().expect("project id");
    let wrong_scope = json_response(
        app.router
            .clone()
            .oneshot(json_request(
                "/api/v1/api-keys",
                &json!({"name":"CI only", "scopes":["ci_eval_runs"]}),
                None,
            ))
            .await
            .expect("wrong-scope key"),
    )
    .await;
    let native = json!({
        "schema_version":"1",
        "project_id":project_id,
        "external_trace_id":"auth-check",
        "privacy_mode":"metadata_only"
    });
    let denied = app
        .router
        .clone()
        .oneshot(json_request(
            "/api/v1/traces/ingest",
            &native,
            Some(&format!(
                "Bearer {}",
                wrong_scope["secret"].as_str().expect("wrong-scope secret")
            )),
        ))
        .await
        .expect("scope denial");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let trace_key = json_response(
        app.router
            .clone()
            .oneshot(json_request(
                "/api/v1/api-keys",
                &json!({"name":"revoked collector", "scopes":["trace_ingest"]}),
                None,
            ))
            .await
            .expect("trace key"),
    )
    .await;
    let trace_key_id = trace_key["api_key"]["id"].as_str().expect("key id");
    let trace_secret = trace_key["secret"].as_str().expect("trace secret");
    let revoked = app
        .router
        .clone()
        .oneshot(request(
            Method::DELETE,
            &format!("/api/v1/api-keys/{trace_key_id}"),
            Body::empty(),
            None,
        ))
        .await
        .expect("revoke key");
    assert_eq!(revoked.status(), StatusCode::OK);
    let denied = app
        .router
        .clone()
        .oneshot(json_request(
            "/api/v1/traces/ingest",
            &native,
            Some(&format!("Bearer {trace_secret}")),
        ))
        .await
        .expect("revoked denial");
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let active = json_response(
        app.router
            .clone()
            .oneshot(json_request(
                "/api/v1/api-keys",
                &json!({"name":"active collector", "scopes":["trace_ingest"]}),
                None,
            ))
            .await
            .expect("active key"),
    )
    .await;
    let active_secret = active["secret"].as_str().expect("active secret");
    let session_only = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/otel/v1/traces")
                .header(header::CONTENT_TYPE, "application/x-protobuf")
                .header("x-corpuslab-project-id", project_id)
                .body(Body::from(
                    ExportTraceServiceRequest::default().encode_to_vec(),
                ))
                .expect("session-only OTLP request"),
        )
        .await
        .expect("session-only response");
    assert_eq!(session_only.status(), StatusCode::UNAUTHORIZED);
    let unsupported_json = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/otel/v1/traces")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {active_secret}"))
                .header("x-corpuslab-project-id", project_id)
                .body(Body::from("{}"))
                .expect("JSON OTLP request"),
        )
        .await
        .expect("unsupported JSON response");
    assert_eq!(
        unsupported_json.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        unsupported_json.headers()[header::CONTENT_TYPE],
        "application/x-protobuf"
    );

    let malformed = app
        .router
        .clone()
        .oneshot(otlp_http_request(
            vec![0xff],
            active_secret,
            project_id,
            None,
        ))
        .await
        .expect("malformed protobuf response");
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        malformed.headers()[header::CONTENT_TYPE],
        "application/x-protobuf"
    );

    let compressed = app
        .router
        .clone()
        .oneshot(otlp_http_request(
            ExportTraceServiceRequest::default().encode_to_vec(),
            active_secret,
            project_id,
            Some("gzip"),
        ))
        .await
        .expect("compressed response");
    assert_eq!(compressed.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let oversized = app
        .router
        .oneshot(otlp_http_request(
            vec![0; 1_048_577],
            active_secret,
            project_id,
            None,
        ))
        .await
        .expect("oversized OTLP response");
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        oversized.headers()[header::CONTENT_TYPE],
        "application/x-protobuf"
    );
}

fn otlp_request() -> ExportTraceServiceRequest {
    ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: None,
            scope_spans: vec![ScopeSpans {
                scope: None,
                schema_url: String::new(),
                spans: vec![Span {
                    trace_id: vec![1; 16],
                    span_id: vec![2; 8],
                    name: "SECRET_SPAN_NAME".to_owned(),
                    start_time_unix_nano: 1_800_000_000_000_000_000,
                    end_time_unix_nano: 1_800_000_000_010_000_000,
                    attributes: vec![
                        attribute("corpuslab.operation", "retrieval"),
                        attribute("corpuslab.query", "SECRET_OTLP_QUERY"),
                        attribute("corpuslab.evidence.external_chunk_id", "chunk-otlp"),
                    ],
                    ..Span::default()
                }],
            }],
            schema_url: String::new(),
        }],
    }
}

fn attribute(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.to_owned(),
        value: Some(AnyValue {
            value: Some(OtlpValue::StringValue(value.to_owned())),
        }),
        key_strindex: 0,
    }
}

fn otlp_http_request(
    body: Vec<u8>,
    secret: &str,
    project_id: &str,
    content_encoding: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/otel/v1/traces")
        .header(header::CONTENT_TYPE, "application/x-protobuf")
        .header(header::AUTHORIZATION, format!("Bearer {secret}"))
        .header("x-corpuslab-project-id", project_id);
    if let Some(value) = content_encoding {
        builder = builder.header(header::CONTENT_ENCODING, value);
    }
    builder.body(Body::from(body)).expect("OTLP request")
}

fn json_request(uri: &str, value: &Value, authorization: Option<&str>) -> Request<Body> {
    request(
        Method::POST,
        uri,
        Body::from(value.to_string()),
        authorization,
    )
}

fn request(method: Method, uri: &str, body: Body, authorization: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(&method).uri(uri);
    if method == Method::POST {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(value) = authorization {
        builder = builder.header(header::AUTHORIZATION, value);
    }
    builder.body(body).expect("request")
}

async fn json_response(response: axum::response::Response) -> Value {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("body");
    assert!(
        status.is_success(),
        "unexpected response {status}: {}",
        String::from_utf8_lossy(&bytes)
    );
    serde_json::from_slice(&bytes).expect("JSON response")
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("JSON response")
}
