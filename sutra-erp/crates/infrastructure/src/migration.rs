//! Migration runner.

use sqlx::PgPool;

/// Run all pending database migrations.
///
/// Migrations are read from the `migrations/` directory at the
/// workspace root.
///
/// # Errors
///
/// Returns an error if the migration cannot be applied.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    // The migrations directory is relative to the workspace root.
    // In development, this is `sutra-erp/migrations/`.
    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("migrations")).await?;
    migrator.run(pool).await?;
    Ok(())
}
