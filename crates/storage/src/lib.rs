pub mod memory;
pub mod postgres;
pub mod repository;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("record not found")]
    NotFound,
    #[error("invalid stored data: {0}")]
    InvalidData(String),
    #[error("internal storage error: {0}")]
    Internal(String),
    #[error("storage operation is not implemented yet: {0}")]
    NotImplemented(&'static str),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
