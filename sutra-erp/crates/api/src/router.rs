//! HTTP router construction.
//!
//! Layout (all under `/api/v1`):
//!
//! ```text
//! POST /auth/register                     — public (no auth)
//! POST /auth/login                        — public (no auth)
//! GET  /auth/me                           — authenticated
//! ── General Ledger ──
//! POST /gl/journals                       — create journal
//! POST /gl/journals/:id/post              — post journal
//! POST /gl/journals/:id/reverse           — reverse journal
//! GET  /gl/journals                       — list journals
//! GET  /gl/journals/:id                   — get journal
//! GET  /gl/trial-balance                  — trial balance
//! GET  /gl/accounts                       — COA tree
//! GET  /gl/accounts/:id/ledger            — account ledger
//! ── Accounts Receivable ──
//! POST /ar/students/:id/assess-fees       — assess student fees
//! POST /ar/payments                       — record fee payment
//! GET  /ar/students/:id/fees              — get student fees
//! GET  /ar/payments/receipts              — list receipts
//! GET  /ar/payments/receipts/:id          — get receipt
//! POST /ar/concessions                    — grant concession
//! POST /ar/scholarships                   — apply scholarship
//! PUT  /ar/scholarships/:id/verify        — verify scholarship
//! PUT  /ar/scholarships/:id/disburse      — record DBT disbursement
//! GET  /ar/scholarships/pending-verification — pending verification
//! POST /ar/refunds                        — initiate refund
//! PUT  /ar/refunds/:id/process            — process refund
//! ── Accounts Payable ──
//! POST /ap/vendors                        — onboard vendor
//! GET  /ap/vendors                        — list vendors
//! GET  /ap/vendors/:id                    — get vendor
//! POST /ap/purchase-orders                — create purchase order
//! PUT  /ap/purchase-orders/:id/issue      — issue PO
//! POST /ap/goods-receipts                 — record GRN
//! POST /ap/invoices                       — record vendor invoice
//! PUT  /ap/invoices/:id/match             — 3-way match
//! PUT  /ap/invoices/:id/post              — post to GL
//! POST /ap/payments                       — create vendor payment
//! PUT  /ap/payments/:id/process           — process payment
//! GET  /ap/tds/deductions                 — TDS register
//! ```

use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use sutra_auth::middleware::auth_layer;

use crate::routes::{auth::auth_routes, gl::gl_routes};
use crate::state::AppState;

/// Build the full application router.
pub fn create_router(state: Arc<AppState>) -> Router {
    // Public auth endpoints (no token required).
    let public_auth = Router::new()
        .route("/register", post(crate::routes::auth::register))
        .route("/login", post(crate::routes::auth::login));

    // Everything below requires a valid Bearer token. The auth layer runs
    // BEFORE permission checks: it builds the UserContext from the JWT and
    // injects it into request extensions; handlers/RBAC read it from there.
    let authed = Router::new()
        .nest("/auth", Router::new().route("/me", get(crate::routes::auth::me)))
        .nest("/gl", gl_routes())
        .nest("/ap", crate::routes::ap::ap_routes())
        .route_layer(middleware::from_fn(auth_layer));

    let api_routes = Router::new()
        .route("/health", get(health_check))
        .nest("/auth", public_auth)
        .merge(authed)
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
