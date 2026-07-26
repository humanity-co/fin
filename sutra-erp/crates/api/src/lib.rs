//! SutraERP — REST API Layer
//!
//! Built on Axum, this crate provides versioned HTTP endpoints
//! with middleware for tenant extraction, authentication,
//! request ID propagation, and distributed tracing.

pub mod middleware;
pub mod routes;
pub mod router;
pub mod state;

// Re-exports
pub use middleware::{
    auth::auth_middleware,
    request_id::request_id_middleware,
    tenant::tenant_middleware,
    tracing::tracing_middleware,
};
pub use router::create_router;
pub use state::AppState;
