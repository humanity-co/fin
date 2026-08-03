use crate::engine::PermissionEngine;
use axum::{extract::FromRequestParts, http::{request::Parts, StatusCode}, response::{IntoResponse, Response, Json}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeGrant { pub scope_type: String, pub scope_id: Option<Uuid> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext { pub user_id: Uuid, pub tenant_id: Uuid, pub scopes: Vec<ScopeGrant> }
#[derive(Debug, Serialize)]
pub struct PermissionDenied { pub error: &'static str, pub permission: String }
impl IntoResponse for PermissionDenied { fn into_response(self) -> Response { (StatusCode::FORBIDDEN, Json(self)).into_response() } }
#[derive(Clone, Debug)]
pub struct RequirePermission { pub code: String }
impl RequirePermission { pub fn new(code: impl Into<String>) -> Self { Self { code: code.into() } } }
#[axum::async_trait]
impl<S> FromRequestParts<S> for RequirePermission where S: Send + Sync, Arc<PermissionEngine>: axum::extract::FromRef<S> {
 type Rejection = PermissionDenied;
 async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
  let code = parts.extensions.get::<RequirePermission>().map(|g| g.code.clone()).ok_or(PermissionDenied { error: "permission guard not configured", permission: String::new() })?;
  let ctx = parts.extensions.get::<UserContext>().cloned().ok_or(PermissionDenied { error: "unauthenticated", permission: code.clone() })?;
  let engine = Arc::<PermissionEngine>::from_ref(state);
  if engine.has_permission(ctx.user_id, ctx.tenant_id, &code, "GLOBAL", None).await.unwrap_or(false) { Ok(Self { code }) } else { tracing::warn!(user_id=%ctx.user_id, permission=%code, "permission denied"); Err(PermissionDenied { error: "permission denied", permission: code }) }
 }
}
