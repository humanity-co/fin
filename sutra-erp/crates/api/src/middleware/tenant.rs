use axum::{extract::Request, middleware::Next, response::{Response, IntoResponse}, http::StatusCode};
use sutra_core::TenantId;
use uuid::Uuid;
/// Extract and validate tenant identity. Database active-state validation belongs at auth boundary.
pub async fn tenant_middleware(mut request: Request, next: Next) -> Response {
 let Some(value)=request.headers().get("X-Tenant-Id") else { return (StatusCode::BAD_REQUEST,"missing X-Tenant-Id").into_response(); };
 let Ok(raw)=value.to_str().ok().and_then(|s|Uuid::parse_str(s).ok()).ok_or(()) else { return (StatusCode::BAD_REQUEST,"invalid X-Tenant-Id").into_response(); };
 request.extensions_mut().insert(TenantId::from_uuid(raw)); next.run(request).await
}
