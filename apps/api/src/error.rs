use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("unprocessable entity: {0}")]
    Unprocessable(String),
    #[error("service is not ready")]
    NotReady,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("storage error: {0}")]
    Storage(#[from] rag_debugger_storage::StorageError),
    #[error("unexpected service error")]
    Internal,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetails,
}

#[derive(Debug, Serialize)]
struct ErrorDetails {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                "bad_request",
                format!("bad request: {message}"),
            ),
            Self::Unauthorized(message) => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                format!("unauthorized: {message}"),
            ),
            Self::Forbidden(message) => (
                StatusCode::FORBIDDEN,
                "forbidden",
                format!("forbidden: {message}"),
            ),
            Self::Unprocessable(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable_entity",
                format!("unprocessable entity: {message}"),
            ),
            Self::NotReady => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_not_ready",
                "service is not ready".to_owned(),
            ),
            Self::NotFound(message) => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("not found: {message}"),
            ),
            Self::Storage(rag_debugger_storage::StorageError::Conflict(message)) => (
                StatusCode::CONFLICT,
                "conflict",
                format!("record already exists: {message}"),
            ),
            Self::Storage(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_error",
                "storage operation failed".to_owned(),
            ),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "unexpected service error".to_owned(),
            ),
        };

        let body = ErrorBody {
            error: ErrorDetails { code, message },
        };

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use serde_json::Value;

    use super::*;

    #[tokio::test]
    async fn serializes_stable_error_envelopes() {
        assert_error(
            ApiError::BadRequest("query is required".to_owned()),
            StatusCode::BAD_REQUEST,
            "bad_request",
            "bad request: query is required",
        )
        .await;
        assert_error(
            ApiError::Unauthorized("session missing".to_owned()),
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "unauthorized: session missing",
        )
        .await;
        assert_error(
            ApiError::NotFound("trace".to_owned()),
            StatusCode::NOT_FOUND,
            "not_found",
            "not found: trace",
        )
        .await;
        assert_error(
            ApiError::Storage(rag_debugger_storage::StorageError::Internal(
                "database credentials leaked here".to_owned(),
            )),
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage_error",
            "storage operation failed",
        )
        .await;
    }

    async fn assert_error(
        error: ApiError,
        expected_status: StatusCode,
        expected_code: &str,
        expected_message: &str,
    ) {
        let response = error.into_response();
        assert_eq!(response.status(), expected_status);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read error response");
        let body: Value = serde_json::from_slice(&bytes).expect("parse error response");
        assert_eq!(body["error"]["code"], expected_code);
        assert_eq!(body["error"]["message"], expected_message);
        assert_eq!(body["error"].as_object().map(|body| body.len()), Some(2));
    }
}
