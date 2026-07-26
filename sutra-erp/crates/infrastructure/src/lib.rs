//! SutraERP — Infrastructure Layer
//!
//! Handles database connection pools, Redis connections,
//! configuration loading, migration running, and telemetry setup.

pub mod config;
pub mod db;
pub mod migration;
pub mod redis_cache;
pub mod telemetry;

// Re-exports
pub use config::AppConfig;
pub use db::create_pool;
pub use migration::run_migrations;
pub use redis_cache::create_redis_pool;
pub use telemetry::init_telemetry;
