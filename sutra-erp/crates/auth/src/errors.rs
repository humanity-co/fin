//! Error types for the authentication kernel.

use thiserror::Error;

/// Errors produced by password hashing, JWT handling, and the auth middleware.
#[derive(Debug, Error)]
pub enum AuthError {
    /// `JWT_SECRET` is not configured in the environment.
    #[error("JWT_SECRET environment variable is not set")]
    MissingSecret,
    /// Password hashing / verification failed.
    #[error("password hashing error: {0}")]
    Hashing(String),
    /// The supplied token could not be decoded or validated.
    #[error("invalid token: {0}")]
    InvalidToken(String),
    /// The token signature is valid but the token has expired.
    #[error("token has expired")]
    TokenExpired,
    /// The token does not carry a well-formed subject / tenant claim.
    #[error("token claim error: {0}")]
    InvalidClaim(String),
}

/// Convenience alias for auth operations.
pub type AuthResult<T> = Result<T, AuthError>;
