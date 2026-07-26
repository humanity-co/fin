//! Tracing middleware (stub).
//!
//! Wraps every request in a tracing span with request metadata.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Middleware that creates a tracing span for each request.
///
/// Currently a pass-through stub. Will create spans with method, path,
/// tenant ID, and request ID.
pub async fn tracing_middleware(
    request: Request,
    next: Next,
) -> Response {
    // TODO: Create tracing span with request metadata
    let _ = request;
    next.run(request).await
}
