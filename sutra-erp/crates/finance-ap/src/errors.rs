//! AP-specific error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApError {
    #[error("vendor {0} not found")]
    VendorNotFound(String),

    #[error("purchase order {0} not found")]
    PONotFound(String),

    #[error("grn {0} not found")]
    GrnNotFound(String),

    #[error("invoice {0} not found")]
    InvoiceNotFound(String),

    #[error("payment {0} not found")]
    PaymentNotFound(String),

    #[error("vendor is blacklisted")]
    VendorBlacklisted,

    #[error("invalid state transition: {0}")]
    InvalidState(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("GL error: {0}")]
    GlError(#[from] sutra_finance_gl::errors::GlError),
}
