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
        let (status, code) = match &self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            Self::Unprocessable(_) => (StatusCode::UNPROCESSABLE_ENTITY, "unprocessable_entity"),
            Self::NotReady => (StatusCode::SERVICE_UNAVAILABLE, "service_not_ready"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Storage(rag_debugger_storage::StorageError::Conflict(_)) => {
                (StatusCode::CONFLICT, "conflict")
            }
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
