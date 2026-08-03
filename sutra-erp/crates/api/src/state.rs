//! Application state shared across all request handlers.

use sqlx::PgPool;
use std::sync::Arc;
use sutra_rbac::PermissionEngine;

/// The global application state available to all handlers via Axum's
/// `State<Arc<AppState>>` extractor.
#[derive(Debug, Clone)]
pub struct AppState {
    /// PostgreSQL connection pool.
    pub db: PgPool,
    /// Redis connection pool (optional — will be None if Redis is not configured).
    pub redis: Option<deadpool_redis::Pool>,
    /// Tenant-aware permission engine.
    pub permission_engine: Arc<PermissionEngine>,
}

impl AppState {
    /// Create a new AppState with the given pools.
    pub fn new(db: PgPool, redis: Option<deadpool_redis::Pool>) -> Self {
        let permission_engine = Arc::new(PermissionEngine::new(db.clone(), redis.clone()));
        AppState { db, redis, permission_engine }
    }
}
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory
