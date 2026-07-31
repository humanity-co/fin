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
/// GET  /health                                    — health check
/// ── General Ledger ──
/// POST /api/v1/gl/journals                        — create journal
/// POST /api/v1/gl/journals/:id/post               — post journal
/// POST /api/v1/gl/journals/:id/reverse             — reverse journal
/// GET  /api/v1/gl/journals                        — list journals
/// GET  /api/v1/gl/journals/:id                    — get journal
/// GET  /api/v1/gl/trial-balance                   — trial balance
/// GET  /api/v1/gl/accounts                        — COA tree
/// GET  /api/v1/gl/accounts/:id/ledger             — account ledger
/// ── Accounts Receivable ──
/// POST /api/v1/ar/students/:id/assess-fees        — assess student fees
/// POST /api/v1/ar/payments                        — record fee payment
/// GET  /api/v1/ar/students/:id/fees               — get student fees
/// GET  /api/v1/ar/payments/receipts               — list receipts
/// GET  /api/v1/ar/payments/receipts/:id           — get receipt
/// POST /api/v1/ar/concessions                     — grant concession
/// POST /api/v1/ar/scholarships                    — apply scholarship
/// PUT  /api/v1/ar/scholarships/:id/verify         — verify scholarship
/// PUT  /api/v1/ar/scholarships/:id/disburse       — record DBT disbursement
/// GET  /api/v1/ar/scholarships/pending-verification — pending verification
/// POST /api/v1/ar/refunds                         — initiate refund
/// PUT  /api/v1/ar/refunds/:id/process             — process refund
/// ── Accounts Payable ──
/// POST /api/v1/ap/vendors                         — onboard vendor
/// GET  /api/v1/ap/vendors                         — list vendors
/// GET  /api/v1/ap/vendors/:id                     — get vendor
/// POST /api/v1/ap/purchase-orders                 — create purchase order
/// PUT  /api/v1/ap/purchase-orders/:id/issue       — issue PO
/// POST /api/v1/ap/goods-receipts                  — record GRN
/// POST /api/v1/ap/invoices                        — record vendor invoice
/// PUT  /api/v1/ap/invoices/:id/match              — 3-way match
/// PUT  /api/v1/ap/invoices/:id/post               — post to GL
/// POST /api/v1/ap/payments                        — create vendor payment
/// PUT  /api/v1/ap/payments/:id/process            — process payment
/// GET  /api/v1/ap/tds/deductions                  — TDS register
/// ```
pub fn create_router(state: Arc<AppState>) -> Router {
    let api_routes = Router::new()
        .route("/health", get(health_check))
        .nest("/gl", routes::gl::gl_routes())
        .nest("/ar", routes::ar::ar_routes())
        .nest("/ap", routes::ap::ap_routes())
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
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory
