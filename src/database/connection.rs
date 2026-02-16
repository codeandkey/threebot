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
        Ok(manager)
    }

    /// Creates a clone of the database pool for use in managers
    pub fn pool_clone(&self) -> DbPool {
        self.pool.clone()
    }
}
