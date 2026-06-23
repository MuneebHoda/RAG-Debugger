pub mod postgres;
pub mod repository;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("record not found")]
    NotFound,
    #[error("storage operation is not implemented yet: {0}")]
    NotImplemented(&'static str),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
