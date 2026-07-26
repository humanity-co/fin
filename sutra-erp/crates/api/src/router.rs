//! Axum router with all API routes.

use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::routes;
use crate::state::AppState;

/// Build the complete Axum router with all middleware and routes.
///
/// # Route Structure
///
/// ```text
/// GET  /health                        — health check
/// POST /api/v1/gl/journals            — create journal
/// POST /api/v1/gl/journals/:id/post   — post journal
/// POST /api/v1/gl/journals/:id/reverse — reverse journal
/// GET  /api/v1/gl/journals            — list journals
/// GET  /api/v1/gl/journals/:id        — get journal
/// GET  /api/v1/gl/trial-balance       — trial balance
/// GET  /api/v1/gl/accounts            — COA tree
/// GET  /api/v1/gl/accounts/:id/ledger — account ledger
/// ```
pub fn create_router(state: Arc<AppState>) -> Router {
    let api_routes = Router::new()
        .route("/health", get(health_check))
        .nest("/gl", routes::gl::gl_routes())
        .with_state(state);

    Router::new()
        .nest("/api/v1", api_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// Health check endpoint — returns 200 OK.
async fn health_check() -> &'static str {
    "OK"
}
