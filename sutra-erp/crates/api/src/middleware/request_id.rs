//! Request ID middleware (stub).
//!
//! Ensures every request has a unique correlation ID for tracing
//! across services and through the event bus.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Middleware that assigns and propagates a request ID.
///
/// Currently a pass-through stub. Will read `X-Request-Id` header
/// or generate a new UUID and inject it into request extensions.
pub async fn request_id_middleware(
    request: Request,
    next: Next,
) -> Response {
    // TODO: Extract or generate X-Request-Id, inject into extensions
    let _ = request;
    next.run(request).await
}
