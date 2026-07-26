//! Redis connection pool.

use deadpool_redis::{Config, Pool, Runtime};

/// Create a Redis connection pool.
///
/// # Errors
///
/// Returns an error if the Redis URL is invalid.
pub fn create_redis_pool(redis_url: &str) -> Result<Pool, deadpool_redis::CreatePoolError> {
    let cfg = Config::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
}
