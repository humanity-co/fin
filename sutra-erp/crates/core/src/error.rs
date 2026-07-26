//! Domain error hierarchy for SutraERP.
//!
//! Every module uses this shared error type. Module-specific errors
//! extend `DomainError` through the `Internal` variant or by embedding
//! a domain-specific error enum inside `Validation`.

use std::fmt;
use thiserror::Error;

/// The unified domain error type for SutraERP.
///
/// All operations return `DomainResult<T> = Result<T, DomainError>`.
#[derive(Debug, Error)]
pub enum DomainError {
    /// Input validation failed (field- or aggregate-level).
    #[error("validation error: {0}")]
    Validation(String),

    /// The requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// The caller is not authorized for this operation.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// A conflict was detected (optimistic lock, duplicate, state machine).
    #[error("conflict: {0}")]
    Conflict(String),

    /// An unexpected internal error occurred.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Convenience type alias.
pub type DomainResult<T> = Result<T, DomainError>;

impl DomainError {
    /// Create a validation error.
    pub fn validation(msg: impl fmt::Display) -> Self {
        DomainError::Validation(msg.to_string())
    }

    /// Create a not-found error.
    pub fn not_found(msg: impl fmt::Display) -> Self {
        DomainError::NotFound(msg.to_string())
    }

    /// Create an unauthorized error.
    pub fn unauthorized(msg: impl fmt::Display) -> Self {
        DomainError::Unauthorized(msg.to_string())
    }

    /// Create a conflict error.
    pub fn conflict(msg: impl fmt::Display) -> Self {
        DomainError::Conflict(msg.to_string())
    }

    /// Create an internal error.
    pub fn internal(msg: impl fmt::Display) -> Self {
        DomainError::Internal(msg.to_string())
    }
}
