use crate::database::connection::DbPool;
use crate::error::Error;

/// Runs all database migrations
pub async fn run_all_migrations(pool: &DbPool) -> Result<(), Error> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || -> Result<(), Error> {
        let conn = pool
            .get()
            .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sounds (
                code TEXT PRIMARY KEY,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                source_url TEXT NULL,
                start_time TEXT NOT NULL,
                length REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS aliases (
                name TEXT PRIMARY KEY,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                commands TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS user_settings (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                setting_type TEXT NOT NULL,
                setting_value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_user_settings_username
            ON user_settings(username);
            ",
        )
        .map_err(|e| Error::DatabaseError(format!("Failed to run database migrations: {}", e)))?;

        info!("All database migrations completed successfully");
        Ok(())
    })
    .await
    .map_err(|e| Error::DatabaseError(format!("Migration task failed: {}", e)))?
}
