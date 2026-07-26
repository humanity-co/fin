//! Tenant extraction middleware (stub).
//!
//! In production, this extracts `X-Tenant-Id` from the request header
//! and injects it into the request extensions for downstream handlers.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Middleware that extracts the tenant ID from the request.
///
/// Currently a pass-through stub. Will validate the tenant header
/// and inject `TenantId` into request extensions.
pub async fn tenant_middleware(
    request: Request,
    next: Next,
) -> Response {
    // TODO: Extract X-Tenant-Id header, validate, inject into extensions
    let _ = request;
    next.run(request).await
}
