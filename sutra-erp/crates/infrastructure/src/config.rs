//! Application configuration loaded from environment variables.

use std::net::SocketAddr;

/// Application configuration loaded from environment variables and .env.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// PostgreSQL connection string.
    pub database_url: String,
    /// Redis connection string.
    pub redis_url: String,
    /// Host to bind the HTTP server to.
    pub host: String,
    /// Port to bind the HTTP server to.
    pub port: u16,
    /// RUST_LOG directive.
    pub rust_log: String,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// Reads a `.env` file if present, then falls back to hardcoded defaults
    /// for local development.
    pub fn from_env() -> Self {
        // Try to load .env (ignore if not found)
        let _ = dotenvy::dotenv();

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://sutraerp:sutraerp@localhost:5432/sutraerp".into());
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".into());
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse()
            .unwrap_or(3000);
        let rust_log = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info,sutra_erp=debug".into());

        AppConfig {
            database_url,
            redis_url,
            host,
            port,
            rust_log,
        }
    }

    /// Construct a `SocketAddr` from the host and port.
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("invalid host:port in config")
    }

    /// Get the database connection string as a `&str`.
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// Get the Redis connection string as a `&str`.
    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }
}
