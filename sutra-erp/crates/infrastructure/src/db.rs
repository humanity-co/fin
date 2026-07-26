//! Database connection pool setup.

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Create a PostgreSQL connection pool.
///
/// # Errors
///
/// Returns an error if the database cannot be reached or the
/// connection string is invalid.
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20) // conservative default
        .connect(database_url)
        .await
}
