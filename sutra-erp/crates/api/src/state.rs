//! Application state shared across all request handlers.

use sqlx::PgPool;
use std::sync::Arc;

/// The global application state available to all handlers via Axum's
/// `State<Arc<AppState>>` extractor.
#[derive(Debug, Clone)]
pub struct AppState {
    /// PostgreSQL connection pool.
    pub db: PgPool,
    /// Redis connection pool (optional — will be None if Redis is not configured).
    pub redis: Option<deadpool_redis::Pool>,
}

impl AppState {
    /// Create a new AppState with the given pools.
    pub fn new(db: PgPool, redis: Option<deadpool_redis::Pool>) -> Self {
        AppState { db, redis }
    }
}
