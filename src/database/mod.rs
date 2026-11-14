use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;
use std::time::Duration;
use crate::errors::AppError;

pub type DbPool = SqlitePool;

pub async fn create_connection_pool(database_url: &str) -> Result<DbPool, AppError> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| AppError::Database(format!("Invalid database connection string: {}", e)))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .connect_with(options)
        .await
        .map_err(|e| AppError::Database(format!("Failed to connect to SQLite database: {}", e)))?;

    tracing::info!("SQLite connection pool established successfully.");
    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to run database migrations: {}", e)))?;

    tracing::info!("Database schema migrations applied successfully.");
    Ok(())
}
