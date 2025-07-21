use crate::error::Error;
use sea_orm::*;
use std::path::Path;

/// Database connection manager
pub struct DatabaseManager {
    connection: DatabaseConnection,
}

impl DatabaseManager {
    /// Creates a new database manager with a connection to the SQLite database
    pub async fn new(database_path: &Path) -> Result<Self, Error> {
        // Ensure the database directory exists
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::DatabaseError(format!("Failed to create database directory: {}", e)))?;
        }

        // Create database URL and connect
        let db_url = format!("sqlite://{}?mode=rwc", database_path.display());
        let connection = Database::connect(&db_url)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to connect to database: {}", e)))?;

        let manager = Self { connection };

        // Run all migrations
        manager.migrate_all().await?;

        Ok(manager)
    }

    /// Gets a reference to the database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// Creates a clone of the database connection for use in managers
    pub fn connection_clone(&self) -> DatabaseConnection {
        self.connection.clone()
    }

    /// Runs all database migrations
    async fn migrate_all(&self) -> Result<(), Error> {
        super::migrations::run_all_migrations(&self.connection).await
    }

    /// Checks if the database is healthy
    pub async fn health_check(&self) -> Result<(), Error> {
        self.connection
            .ping()
            .await
            .map_err(|e| Error::DatabaseError(format!("Database health check failed: {}", e)))?;
        Ok(())
    }
}
