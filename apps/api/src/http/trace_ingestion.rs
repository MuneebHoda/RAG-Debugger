use std::time::Instant;

use axum::{
    body::Bytes,
    extract::{rejection::BytesRejection, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTracePartialSuccess, ExportTraceServiceResponse,
};
use prost::Message;
use rag_debugger_core::{
    ApiKeyScope, NativeTraceIngestionRequest, ProjectId, TraceIngestionDisposition,
    TraceIngestionResponse, WorkspaceId,
};
use rag_debugger_rag::imported_trace::{add_diagnosis, build_native_trace, ImportValidationError};
use serde_json::Value;
use uuid::Uuid;

use super::otlp;
use crate::{auth, error::ApiError, state::AppState};

pub const MAX_BODY_BYTES: usize = 1_048_576;

pub async fn ingest_native(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let started = Instant::now();
    let body = match body {
        Ok(body) => body,
        Err(_) => {
            tracing::info!(
                source = "native",
                outcome = "rejected",
                rejection_code = "payload_limit_exceeded",
                accepted_requests = 0_u32,
                rejected_requests = 1_u32,
                accepted_spans = 0_u32,
                rejected_spans = 0_u32,
                payload_bytes = 0_u64,
                parse_mapping_latency_ms = started.elapsed().as_millis() as u64,
                "trace ingestion rejected"
            );
            return coded(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_limit_exceeded",
                "trace ingestion body exceeds 1 MiB",
            )
            .into_response();
        }
    };
    let result = ingest_native_inner(&state, &headers, &body).await;
    match result {
        Ok((status, response)) => {
            tracing::info!(
                source = "native",
                outcome = "accepted",
                accepted_requests = 1_u32,
                rejected_requests = 0_u32,
                accepted_spans = response.accepted_span_count,
                rejected_spans = 0_u32,
                payload_bytes = body.len() as u64,
                parse_mapping_latency_ms = started.elapsed().as_millis() as u64,
                mapping_status = ?response.mapping_status,
                "trace ingestion completed"
            );
            (status, Json(response)).into_response()
        }
        Err(error) => {
            tracing::info!(
                source = "native",
                outcome = "rejected",
                rejection_code = native_rejection_code(&error),
                accepted_requests = 0_u32,
                rejected_requests = 1_u32,
                accepted_spans = 0_u32,
                rejected_spans = 0_u32,
                payload_bytes = body.len() as u64,
                parse_mapping_latency_ms = started.elapsed().as_millis() as u64,
                "trace ingestion rejected"
            );
            error.into_response()
        }
    }
}

async fn ingest_native_inner(
    state: &AppState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(StatusCode, TraceIngestionResponse), ApiError> {
    require_content_type(headers, "application/json")?;
    require_identity_encoding(headers)?;
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let workspace_id = authenticate_native(state, headers).await?;
    let value: Value = serde_json::from_slice(body).map_err(|_| {
        coded(
            StatusCode::BAD_REQUEST,
            "malformed_json",
            "request body is not valid JSON",
        )
    })?;
    validate_json_shape(&value, 0)?;
    let request: NativeTraceIngestionRequest = serde_json::from_value(value).map_err(|_| {
        coded(
            StatusCode::BAD_REQUEST,
            "invalid_native_trace",
            "native trace does not match schema version 1",
        )
    })?;
    let project = repository
        .get_project(workspace_id, request.project_id)
        .await
        .map_err(|error| match error {
            rag_debugger_storage::StorageError::NotFound => coded(
                StatusCode::NOT_FOUND,
                "project_not_found",
                "project was not found",
            ),
            other => ApiError::Storage(other),
        })?;
    let trace = build_native_trace(
        &project,
        request,
        &state.config().product.retrieval,
        &state.config().product.debugger,
    )
    .map_err(validation_error)?;
    let accepted_span_count = trace
        .ingestion
        .as_ref()
        .map_or(0, |metadata| metadata.spans.len() as u32);
    let mut saved = repository
        .upsert_imported_trace(workspace_id, trace)
        .await
        .map_err(import_storage_error)?;
    let mut diagnosed = saved.trace.clone();
    add_diagnosis(
        &mut diagnosed,
        &state.config().product.retrieval,
        &state.config().product.debugger,
    );
    if diagnosed != saved.trace {
        saved.trace = repository
            .upsert_imported_trace(workspace_id, diagnosed)
            .await
            .map_err(import_storage_error)?
            .trace;
    }
    let metadata = saved.trace.ingestion.as_ref().ok_or(ApiError::Internal)?;
    let status = if saved.disposition == TraceIngestionDisposition::Created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        TraceIngestionResponse {
            trace_id: saved.trace.id,
            external_trace_id: metadata.external_trace_id.clone(),
            disposition: saved.disposition,
            mapping_status: metadata.mapping_status,
            accepted_span_count,
            limitations: metadata.limitations.clone(),
        },
    ))
}

fn native_rejection_code(error: &ApiError) -> &'static str {
    match error {
        ApiError::Coded { code, .. } => code,
        ApiError::Unauthorized(_) => "unauthorized",
        ApiError::Forbidden(_) => "insufficient_scope",
        ApiError::NotReady => "service_not_ready",
        ApiError::Storage(_) => "storage_error",
        _ => "internal_error",
    }
}

async fn authenticate_native(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<WorkspaceId, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    if headers.contains_key(header::AUTHORIZATION) {
        return Ok(auth::authenticate_api_key(
            repository.as_ref(),
            headers,
            ApiKeyScope::TraceIngest,
        )
        .await?
        .workspace_id);
    }
    Ok(
        auth::authenticate_session(repository.as_ref(), headers, &state.config().auth)
            .await?
            .workspace
            .id,
    )
}

pub(crate) fn require_content_type(
    headers: &HeaderMap,
    expected: &'static str,
) -> Result<(), ApiError> {
    let matches = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case(expected));
    if !matches {
        return Err(coded(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_content_type",
            "request content type is not supported",
        ));
    }
    Ok(())
}

pub(crate) fn require_identity_encoding(headers: &HeaderMap) -> Result<(), ApiError> {
    let supported = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_none_or(|value| value.eq_ignore_ascii_case("identity"));
    if !supported {
        return Err(coded(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_content_encoding",
            "compressed trace ingestion is not supported",
        ));
    }
    Ok(())
}

fn validate_json_shape(value: &Value, depth: usize) -> Result<(), ApiError> {
    if depth > 8 {
        return Err(coded(
            StatusCode::BAD_REQUEST,
            "nested_value_limit_exceeded",
            "JSON nesting exceeds the supported limit",
        ));
    }
    match value {
        Value::Array(values) => {
            if values.len() > 256 {
                return Err(coded(
                    StatusCode::BAD_REQUEST,
                    "collection_limit_exceeded",
                    "JSON collection exceeds the supported limit",
                ));
            }
            for value in values {
                validate_json_shape(value, depth + 1)?;
            }
        }
        Value::Object(values) => {
            if values.len() > 32 && depth > 1 {
                return Err(coded(
                    StatusCode::BAD_REQUEST,
                    "attribute_limit_exceeded",
                    "JSON object exceeds the supported attribute limit",
                ));
            }
            for value in values.values() {
                validate_json_shape(value, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validation_error(error: ImportValidationError) -> ApiError {
    coded(
        StatusCode::BAD_REQUEST,
        error.code(),
        "native trace validation failed",
    )
}

fn import_storage_error(error: rag_debugger_storage::StorageError) -> ApiError {
    match error {
        rag_debugger_storage::StorageError::Conflict(_) => coded(
            StatusCode::CONFLICT,
            "import_identity_conflict",
            "trace import identity conflicts with stored schema or privacy mode",
        ),
        rag_debugger_storage::StorageError::NotFound => coded(
            StatusCode::NOT_FOUND,
            "project_not_found",
            "project was not found",
        ),
        other => ApiError::Storage(other),
    }
}

pub(crate) const fn coded(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> ApiError {
    ApiError::Coded {
        status,
        code,
        message,
    }
}

pub async fn ingest_otlp(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let started = Instant::now();
    let body = match body {
        Ok(body) => body,
        Err(_) => {
            tracing::info!(
                source = "otlp_http",
                outcome = "rejected",
                rejection_code = "payload_limit_exceeded",
                accepted_requests = 0_u32,
                rejected_requests = 1_u32,
                accepted_spans = 0_u32,
                rejected_spans = 0_u32,
                payload_bytes = 0_u64,
                parse_mapping_latency_ms = started.elapsed().as_millis() as u64,
                "trace ingestion rejected"
            );
            return OtlpRejection::new(StatusCode::PAYLOAD_TOO_LARGE, "payload_limit_exceeded")
                .into_response();
        }
    };
    let result = ingest_otlp_inner(&state, &headers, &body).await;
    match result {
        Ok((response, accepted_spans, rejected_spans, mapping_counts)) => {
            tracing::info!(
                source = "otlp_http",
                outcome = "accepted",
                accepted_requests = 1_u32,
                rejected_requests = 0_u32,
                accepted_spans,
                rejected_spans,
                payload_bytes = body.len() as u64,
                parse_mapping_latency_ms = started.elapsed().as_millis() as u64,
                complete_traces = mapping_counts.0,
                partially_mapped_traces = mapping_counts.1,
                "trace ingestion completed"
            );
            protobuf_response(StatusCode::OK, response.encode_to_vec())
        }
        Err(error) => {
            tracing::info!(
                source = "otlp_http",
                outcome = "rejected",
                rejection_code = error.code,
                accepted_requests = 0_u32,
                rejected_requests = 1_u32,
                accepted_spans = 0_u32,
                rejected_spans = 0_u32,
                payload_bytes = body.len() as u64,
                parse_mapping_latency_ms = started.elapsed().as_millis() as u64,
                "trace ingestion rejected"
            );
            error.into_response()
        }
    }
}

async fn ingest_otlp_inner(
    state: &AppState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(ExportTraceServiceResponse, u32, u32, (u32, u32)), OtlpRejection> {
    require_content_type(headers, "application/x-protobuf").map_err(OtlpRejection::from_api)?;
    require_identity_encoding(headers).map_err(OtlpRejection::from_api)?;
    let repository = state
        .repository()
        .ok_or_else(|| OtlpRejection::new(StatusCode::SERVICE_UNAVAILABLE, "service_not_ready"))?;
    let api_key =
        auth::authenticate_api_key(repository.as_ref(), headers, ApiKeyScope::TraceIngest)
            .await
            .map_err(OtlpRejection::from_api)?;
    let project_id = project_id_header(headers).map_err(OtlpRejection::from_api)?;
    let project = repository
        .get_project(api_key.workspace_id, project_id)
        .await
        .map_err(|error| match error {
            rag_debugger_storage::StorageError::NotFound => {
                OtlpRejection::new(StatusCode::NOT_FOUND, "project_not_found")
            }
            _ => OtlpRejection::new(StatusCode::INTERNAL_SERVER_ERROR, "storage_error"),
        })?;
    let batch = otlp::decode_and_map(
        body,
        &project,
        &state.config().product.retrieval,
        &state.config().product.debugger,
    )
    .map_err(|error| OtlpRejection::new(StatusCode::BAD_REQUEST, error.code))?;
    let mut accepted_spans = 0_u32;
    let mut complete = 0_u32;
    let mut partial = 0_u32;
    for trace in batch.traces {
        let metadata = trace.ingestion.as_ref().ok_or_else(|| {
            OtlpRejection::new(StatusCode::INTERNAL_SERVER_ERROR, "mapping_error")
        })?;
        accepted_spans = accepted_spans.saturating_add(metadata.spans.len() as u32);
        match metadata.mapping_status {
            rag_debugger_core::TraceMappingStatus::Complete => complete += 1,
            rag_debugger_core::TraceMappingStatus::PartiallyMapped => partial += 1,
        }
        repository
            .upsert_imported_trace(api_key.workspace_id, trace)
            .await
            .map_err(|error| match error {
                rag_debugger_storage::StorageError::Conflict(_) => {
                    OtlpRejection::new(StatusCode::CONFLICT, "import_identity_conflict")
                }
                _ => OtlpRejection::new(StatusCode::INTERNAL_SERVER_ERROR, "storage_error"),
            })?;
    }
    let rejected = u32::try_from(batch.rejected_spans).unwrap_or(u32::MAX);
    let partial_success = (batch.rejected_spans > 0).then(|| ExportTracePartialSuccess {
        rejected_spans: batch.rejected_spans,
        error_message: batch
            .rejection_reasons
            .iter()
            .map(|(code, count)| format!("{code}:{count}"))
            .collect::<Vec<_>>()
            .join(","),
    });
    Ok((
        ExportTraceServiceResponse { partial_success },
        accepted_spans,
        rejected,
        (complete, partial),
    ))
}

#[derive(Debug)]
struct OtlpRejection {
    status: StatusCode,
    code: &'static str,
}

impl OtlpRejection {
    const fn new(status: StatusCode, code: &'static str) -> Self {
        Self { status, code }
    }

    fn from_api(error: ApiError) -> Self {
        match error {
            ApiError::Coded { status, code, .. } => Self::new(status, code),
            ApiError::Unauthorized(_) => Self::new(StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::Forbidden(_) => Self::new(StatusCode::FORBIDDEN, "insufficient_scope"),
            ApiError::NotReady => Self::new(StatusCode::SERVICE_UNAVAILABLE, "service_not_ready"),
            _ => Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }
}

#[derive(Clone, PartialEq, Message)]
struct RpcStatus {
    #[prost(int32, tag = "1")]
    code: i32,
    #[prost(string, tag = "2")]
    message: String,
}

impl IntoResponse for OtlpRejection {
    fn into_response(self) -> Response {
        let rpc_code = match self.status {
            StatusCode::BAD_REQUEST => 3,
            StatusCode::UNAUTHORIZED => 16,
            StatusCode::FORBIDDEN => 7,
            StatusCode::NOT_FOUND => 5,
            StatusCode::CONFLICT => 10,
            StatusCode::PAYLOAD_TOO_LARGE => 8,
            StatusCode::UNSUPPORTED_MEDIA_TYPE | StatusCode::NOT_IMPLEMENTED => 12,
            StatusCode::SERVICE_UNAVAILABLE => 14,
            _ => 13,
        };
        protobuf_response(
            self.status,
            RpcStatus {
                code: rpc_code,
                message: self.code.to_owned(),
            }
            .encode_to_vec(),
        )
    }
}

fn protobuf_response(status: StatusCode, body: Vec<u8>) -> Response {
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-protobuf"),
        )],
        body,
    )
        .into_response()
}

pub(crate) fn project_id_header(headers: &HeaderMap) -> Result<ProjectId, ApiError> {
    let raw = headers
        .get("x-corpuslab-project-id")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            coded(
                StatusCode::BAD_REQUEST,
                "project_header_required",
                "x-corpuslab-project-id is required",
            )
        })?;
    Uuid::parse_str(raw).map(ProjectId).map_err(|_| {
        coded(
            StatusCode::BAD_REQUEST,
            "invalid_project_id",
            "project ID is invalid",
        )
    })
}
