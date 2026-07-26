//! Authentication middleware (stub).
//!
//! In production, this validates JWT tokens and injects
//! the authenticated user's claims into request extensions.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Middleware that validates authentication.
///
/// Currently a pass-through stub. Will validate JWT/Bearer tokens
/// and inject user ID + roles into request extensions.
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Response {
    // TODO: Extract Authorization header, validate JWT, inject claims
    let _ = request;
    next.run(request).await
}
