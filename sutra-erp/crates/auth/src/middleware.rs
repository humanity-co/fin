//! Axum auth middleware and the authenticated [`UserContext`] identity.
//!
//! [`auth_layer`] validates the `Authorization: Bearer <token>` header,
//! verifies the JWT, and injects a [`UserContext`] into the request
//! extensions. The RBAC permission layer reads that context (see
//! `sutra-rbac`), so this middleware must be applied **before** any
//! permission checks in the middleware stack.

use axum::{
    extract::{FromRequestParts, Request},
    http::{header::AUTHORIZATION, request::Parts, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::jwt::validate_token_identity;

/// The authenticated caller's identity.
///
/// Injected into request extensions by [`auth_layer`] and consumed by:
/// - the RBAC permission engine (`sutra-rbac::PermissionEngine::require`),
/// - route handlers via the `UserContext` extractor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// Authenticated user id (from the JWT `sub` claim).
    pub user_id: Uuid,
    /// Tenant the user belongs to (from the JWT `tenant_id` claim).
    pub tenant_id: Uuid,
    /// Role codes carried by the token (e.g. `["CFO", "Accountant"]`).
    pub roles: Vec<String>,
}

/// Axum middleware: authenticate the request from its Bearer token.
///
/// Returns `401` (with the standard error envelope) when the header is
/// missing, malformed, or the token is invalid/expired. On success the
/// [`UserContext`] is inserted into `request.extensions_mut()` and the next
/// layer runs.
pub async fn auth_layer(mut request: Request, next: Next) -> Response {
    let Some(header) = request.headers().get(AUTHORIZATION) else {
        return unauthorized("missing Authorization header");
    };
    let Some(token) = bearer_token(header) else {
        return unauthorized("malformed Authorization header — expected 'Bearer <token>'");
    };

    let (user_id, tenant_id, roles) = match validate_token_identity(token) {
        Ok(identity) => identity,
        Err(e) => {
            tracing::warn!(error = %e, "auth: rejected token");
            return unauthorized("invalid or expired token");
        }
    };

    request.extensions_mut().insert(UserContext {
        user_id,
        tenant_id,
        roles,
    });
    next.run(request).await
}

/// Extract the token from an `Authorization: Bearer <token>` header value.
fn bearer_token(header: &HeaderValue) -> Option<&str> {
    let value = header.to_str().ok()?;
    let mut parts = value.splitn(2, ' ');
    if parts.next()? != "Bearer" {
        return None;
    }
    let token = parts.next()?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// Standard `401` error envelope used across the API.
fn unauthorized(message: &'static str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "success": false,
            "error": message,
            "data": null,
        })),
    )
        .into_response()
}

/// Extractor for handlers: `async fn handler(user: UserContext) { ... }`.
///
/// Rejects with `401` when the request has not passed through
/// [`auth_layer`] (i.e. no authenticated context is present).
#[axum::async_trait]
impl<S> FromRequestParts<S> for UserContext
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<UserContext>()
            .cloned()
            .ok_or(AuthRejection)
    }
}

/// Rejection for the [`UserContext`] extractor when no auth context exists.
#[derive(Debug)]
pub struct AuthRejection;

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "success": false,
                "error": "unauthenticated",
                "data": null,
            })),
        )
            .into_response()
    }
}
