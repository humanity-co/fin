//! AR-specific error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArError {
    #[error("fee structure {0} not found")]
    FeeStructureNotFound(String),

    #[error("payment receipt {0} not found")]
    ReceiptNotFound(String),

    #[error("scholarship {0} not found")]
    ScholarshipNotFound(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
