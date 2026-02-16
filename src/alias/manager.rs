use crate::database::connection::DbPool;
use crate::database::entities::aliases as alias_entity;
use crate::error::Error;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};

pub struct AliasManager {
    db: DbPool,
}

impl AliasManager {
    /// Creates a new alias manager with a database pool
    pub fn new(database: DbPool) -> Self {
        Self { db: database }
    }

    fn parse_created_at(value: &str) -> Result<DateTime<Utc>, Error> {
        chrono::DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                Error::DatabaseError(format!("Invalid alias timestamp '{}': {}", value, e))
            })
    }

    /// Creates a new alias
    pub async fn create_alias(
        &self,
        name: &str,
        author: &str,
        commands: &str,
    ) -> Result<(), Error> {
        let pool = self.db.clone();
        let name = name.to_string();
        let author = author.to_string();
        let commands = commands.to_string();

        tokio::task::spawn_blocking(move || -> Result<(), Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let created_at = Utc::now().to_rfc3339();
            let result = conn.execute(
                "INSERT INTO aliases (name, author, created_at, commands) VALUES (?1, ?2, ?3, ?4)",
                params![name, author, created_at, commands],
            );

            match result {
                Ok(_) => Ok(()),
                Err(e) => {
                    if e.to_string().contains("UNIQUE constraint failed") {
                        Err(Error::InvalidArgument("Alias already exists".to_string()))
                    } else {
                        Err(Error::DatabaseError(format!(
                            "Failed to create alias: {}",
                            e
                        )))
                    }
                }
            }
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias create task failed: {}", e)))?
    }

    /// Gets an alias by name
    pub async fn get_alias(&self, name: &str) -> Result<Option<alias_entity::Model>, Error> {
        let pool = self.db.clone();
        let name = name.to_string();

        tokio::task::spawn_blocking(move || -> Result<Option<alias_entity::Model>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;

            let row = conn
                .query_row(
                    "SELECT name, author, created_at, commands FROM aliases WHERE name = ?1",
                    params![name],
                    |row| {
                        let created_at: String = row.get(2)?;
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            created_at,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| Error::DatabaseError(format!("Failed to get alias: {}", e)))?;

            if let Some((name, author, created_at_raw, commands)) = row {
                Ok(Some(alias_entity::Model {
                    name,
                    author,
                    created_at: Self::parse_created_at(&created_at_raw)?,
                    commands,
                }))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias get task failed: {}", e)))?
    }

    /// Deletes an alias by name
    pub async fn delete_alias(&self, name: &str) -> Result<bool, Error> {
        let pool = self.db.clone();
        let name = name.to_string();

        tokio::task::spawn_blocking(move || -> Result<bool, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let rows = conn
                .execute("DELETE FROM aliases WHERE name = ?1", params![name])
                .map_err(|e| Error::DatabaseError(format!("Failed to delete alias: {}", e)))?;
            Ok(rows > 0)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias delete task failed: {}", e)))?
    }

    /// Lists aliases with pagination
    pub async fn list_aliases_paginated(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<Vec<alias_entity::Model>, Error> {
        let pool = self.db.clone();
        let offset = (page * per_page) as i64;
        let limit = per_page as i64;

        tokio::task::spawn_blocking(move || -> Result<Vec<alias_entity::Model>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let mut stmt = conn
                .prepare(
                    "SELECT name, author, created_at, commands
                     FROM aliases
                     ORDER BY name ASC
                     LIMIT ?1 OFFSET ?2",
                )
                .map_err(|e| Error::DatabaseError(format!("Failed to list aliases: {}", e)))?;

            let rows = stmt
                .query_map(params![limit, offset], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| Error::DatabaseError(format!("Failed to list aliases: {}", e)))?;

            let mut aliases = Vec::new();
            for row in rows {
                let (name, author, created_at_raw, commands) = row.map_err(|e| {
                    Error::DatabaseError(format!("Failed to read alias row: {}", e))
                })?;
                aliases.push(alias_entity::Model {
                    name,
                    author,
                    created_at: Self::parse_created_at(&created_at_raw)?,
                    commands,
                });
            }
            Ok(aliases)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias page task failed: {}", e)))?
    }

    /// Counts total number of aliases
    pub async fn count_aliases(&self) -> Result<u64, Error> {
        let pool = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<u64, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM aliases", [], |row| row.get(0))
                .map_err(|e| Error::DatabaseError(format!("Failed to count aliases: {}", e)))?;
            Ok(count as u64)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias count task failed: {}", e)))?
    }

    /// Searches aliases by name or commands
    pub async fn search_aliases(
        &self,
        search_term: &str,
        page: u64,
        per_page: u64,
    ) -> Result<Vec<alias_entity::Model>, Error> {
        let pool = self.db.clone();
        let search_pattern = format!("%{}%", search_term);
        let offset = (page * per_page) as i64;
        let limit = per_page as i64;

        tokio::task::spawn_blocking(move || -> Result<Vec<alias_entity::Model>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let mut stmt = conn
                .prepare(
                    "SELECT name, author, created_at, commands
                     FROM aliases
                     WHERE name LIKE ?1 OR commands LIKE ?1
                     ORDER BY name ASC
                     LIMIT ?2 OFFSET ?3",
                )
                .map_err(|e| Error::DatabaseError(format!("Failed to search aliases: {}", e)))?;

            let rows = stmt
                .query_map(params![search_pattern, limit, offset], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| Error::DatabaseError(format!("Failed to search aliases: {}", e)))?;

            let mut aliases = Vec::new();
            for row in rows {
                let (name, author, created_at_raw, commands) = row.map_err(|e| {
                    Error::DatabaseError(format!("Failed to read alias row: {}", e))
                })?;
                aliases.push(alias_entity::Model {
                    name,
                    author,
                    created_at: Self::parse_created_at(&created_at_raw)?,
                    commands,
                });
            }
            Ok(aliases)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias search task failed: {}", e)))?
    }

    /// Counts aliases matching search term
    pub async fn count_search_aliases(&self, search_term: &str) -> Result<u64, Error> {
        let pool = self.db.clone();
        let search_pattern = format!("%{}%", search_term);

        tokio::task::spawn_blocking(move || -> Result<u64, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM aliases WHERE name LIKE ?1 OR commands LIKE ?1",
                    params![search_pattern],
                    |row| row.get(0),
                )
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to count search results: {}", e))
                })?;
            Ok(count as u64)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias search count task failed: {}", e)))?
    }

    /// Lists alias names and command strings for bulk in-memory sound matching.
    pub async fn list_alias_names_and_commands(&self) -> Result<Vec<(String, String)>, Error> {
        let pool = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;

            let mut stmt = conn
                .prepare("SELECT name, commands FROM aliases")
                .map_err(|e| Error::DatabaseError(format!("Failed to list aliases: {}", e)))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| Error::DatabaseError(format!("Failed to list aliases: {}", e)))?;

            let mut aliases = Vec::new();
            for row in rows {
                aliases.push(row.map_err(|e| {
                    Error::DatabaseError(format!("Failed to read alias row: {}", e))
                })?);
            }

            Ok(aliases)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Alias list task failed: {}", e)))?
    }
}
