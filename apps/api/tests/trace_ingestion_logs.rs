mod support;

use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use opentelemetry_proto::tonic::{
    collector::trace::v1::ExportTraceServiceRequest,
    common::v1::{any_value::Value as OtlpValue, AnyValue, KeyValue},
    trace::v1::{ResourceSpans, ScopeSpans, Span},
};
use prost::Message;
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test(flavor = "current_thread")]
async fn operational_logs_exclude_credentials_and_otlp_content() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let output = captured.clone();
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .without_time()
            .with_max_level(tracing::Level::INFO)
            .with_writer(move || CapturedWriter(output.clone()))
            .finish(),
    )
    .expect("install isolated test log capture");

    let (router, secret, project_id) = otlp_test_context().await;
    let markers = [
        "PRIVATE_QUERY_MARKER",
        "PRIVATE_PROMPT_MARKER",
        "PRIVATE_ANSWER_MARKER",
        "PRIVATE_SNIPPET_MARKER",
        "PRIVATE_SPAN_NAME_MARKER",
    ];
    let mut request = otlp_request();
    let span = &mut request.resource_spans[0].scope_spans[0].spans[0];
    span.name = markers[4].to_owned();
    span.attributes.extend([
        attribute("corpuslab.query", markers[0]),
        attribute("corpuslab.prompt", markers[1]),
        attribute("corpuslab.answer", markers[2]),
        attribute("corpuslab.evidence.snippet", markers[3]),
    ]);
    let response = router
        .oneshot(otlp_http_request(
            request.encode_to_vec(),
            &secret,
            &project_id,
        ))
        .await
        .expect("captured OTLP response");

    assert_eq!(response.status(), StatusCode::OK);
    let logs =
        String::from_utf8(captured.lock().expect("capture lock").clone()).expect("UTF-8 logs");
    assert!(logs.contains("trace ingestion completed"));
    assert!(!logs.contains(&secret));
    for marker in markers {
        assert!(!logs.contains(marker), "logs contained {marker}");
    }
}

async fn otlp_test_context() -> (axum::Router, String, String) {
    let app = support::authenticated_test_app().await;
    let project = json_response(
        app.router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/projects/current")
                    .body(Body::empty())
                    .expect("project request"),
            )
            .await
            .expect("project response"),
    )
    .await;
    let key = json_response(
        app.router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/api-keys")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({"name":"log capture", "scopes":["trace_ingest"]}).to_string(),
                    ))
                    .expect("key request"),
            )
            .await
            .expect("key response"),
    )
    .await;
    (
        app.router,
        key["secret"].as_str().expect("secret").to_owned(),
        project["id"].as_str().expect("project ID").to_owned(),
    )
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
                    name: "PRIVATE_SPAN_NAME_MARKER".to_owned(),
                    start_time_unix_nano: 1_800_000_000_000_000_000,
                    end_time_unix_nano: 1_800_000_000_010_000_000,
                    attributes: vec![attribute("corpuslab.operation", "retrieval")],
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

fn otlp_http_request(body: Vec<u8>, secret: &str, project_id: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/api/v1/otel/v1/traces")
        .header(header::CONTENT_TYPE, "application/x-protobuf")
        .header(header::AUTHORIZATION, format!("Bearer {secret}"))
        .header("x-corpuslab-project-id", project_id)
        .body(Body::from(body))
        .expect("OTLP request")
}

async fn json_response(response: axum::response::Response) -> Value {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("response body");
    assert!(
        status.is_success(),
        "unexpected response {status}: {}",
        String::from_utf8_lossy(&bytes)
    );
    serde_json::from_slice(&bytes).expect("JSON response")
}

struct CapturedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("capture lock").write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
