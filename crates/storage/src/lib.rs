pub mod memory;
pub mod postgres;
pub mod repository;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("record not found")]
    NotFound,
    #[error("selected evidence is unavailable")]
    UnavailableEvidence,
    #[error("record already exists: {0}")]
    Conflict(String),
    #[error("invalid stored data: {0}")]
    InvalidData(String),
    #[error("trace merge failed: {0}")]
    TraceMerge(#[from] rag_debugger_core::TraceMergeError),
    #[error("internal storage error: {0}")]
    Internal(String),
    #[error("storage operation is not implemented yet: {0}")]
    NotImplemented(&'static str),
    #[error("database migration {0} has not been applied")]
    PendingMigration(i64),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
