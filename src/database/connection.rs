use crate::error::Error;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Database connection manager
pub struct DatabaseManager {
    pool: DbPool,
}

impl DatabaseManager {
    /// Creates a new database manager with a connection pool to the SQLite database
    pub async fn new(database_path: &Path) -> Result<Self, Error> {
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::DatabaseError(format!("Failed to create database directory: {}", e))
            })?;
        }

        let manager = SqliteConnectionManager::file(database_path);
        let pool = Pool::builder()
            .max_size(8)
            .build(manager)
            .map_err(|e| Error::DatabaseError(format!("Failed to create database pool: {}", e)))?;

        {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to configure database pragmas: {}", e))
                })?;
        }

        let manager = Self { pool };
        manager.migrate_all().await?;
        Ok(manager)
    }

    /// Creates a clone of the database pool for use in managers
    pub fn pool_clone(&self) -> DbPool {
        self.pool.clone()
    }

    /// Runs all database migrations
    async fn migrate_all(&self) -> Result<(), Error> {
        super::migrations::run_all_migrations(&self.pool).await
    }

    /// Checks if the database is healthy
    pub async fn health_check(&self) -> Result<(), Error> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<(), Error> {
            let conn = pool.get().map_err(|e| {
                Error::DatabaseError(format!("Database health check failed: {}", e))
            })?;
            conn.query_row("SELECT 1", [], |_row| Ok(())).map_err(|e| {
                Error::DatabaseError(format!("Database health check failed: {}", e))
            })?;
            Ok(())
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Database health check task failed: {}", e)))?
    }
}
