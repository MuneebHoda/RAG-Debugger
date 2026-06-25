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
        let (status, code) = match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::NotReady => (StatusCode::SERVICE_UNAVAILABLE, "service_not_ready"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, "storage_error"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = ErrorBody {
            error: ErrorDetails {
                code,
                message: self.to_string(),
            },
        };

        (status, Json(body)).into_response()
    }
}
