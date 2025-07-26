use crate::database::entities::aliases as alias_entity;
use crate::error::Error;
use sea_orm::*;

pub struct AliasManager {
    db: DatabaseConnection,
}

impl AliasManager {
    /// Creates a new alias manager with a database connection
    pub fn new(database: DatabaseConnection) -> Self {
        Self { db: database }
    }

    /// Creates a new alias
    pub async fn create_alias(
        &self,
        name: &str,
        author: &str,
        commands: &str,
    ) -> Result<(), Error> {
        let alias = alias_entity::ActiveModel::new_for_insert(
            name.to_string(),
            author.to_string(),
            commands.to_string(),
        );

        alias_entity::Entity::insert(alias)
            .exec(&self.db)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    Error::InvalidArgument(format!("Alias '{}' already exists", name))
                } else {
                    Error::DatabaseError(format!("Failed to create alias: {}", e))
                }
            })?;

        Ok(())
    }

    /// Gets an alias by name
    pub async fn get_alias(&self, name: &str) -> Result<Option<alias_entity::Model>, Error> {
        alias_entity::Entity::find_by_id(name)
            .one(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to get alias: {}", e)))
    }

    /// Lists all aliases
    pub async fn list_aliases(&self) -> Result<Vec<alias_entity::Model>, Error> {
        alias_entity::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to list aliases: {}", e)))
    }

    /// Deletes an alias by name
    pub async fn delete_alias(&self, name: &str) -> Result<bool, Error> {
        let result = alias_entity::Entity::delete_by_id(name)
            .exec(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to delete alias: {}", e)))?;

        Ok(result.rows_affected > 0)
    }

    /// Checks if an alias exists
    pub async fn alias_exists(&self, name: &str) -> Result<bool, Error> {
        let count = alias_entity::Entity::find_by_id(name)
            .count(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to check alias existence: {}", e)))?;

        Ok(count > 0)
    }

    /// Lists aliases with pagination
    pub async fn list_aliases_paginated(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<Vec<alias_entity::Model>, Error> {
        let offset = page * per_page;

        let aliases = alias_entity::Entity::find()
            .order_by_asc(alias_entity::Column::Name)
            .offset(offset)
            .limit(per_page)
            .all(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to list aliases: {}", e)))?;

        Ok(aliases)
    }

    /// Counts total number of aliases
    pub async fn count_aliases(&self) -> Result<u64, Error> {
        let count = alias_entity::Entity::find()
            .count(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to count aliases: {}", e)))?;

        Ok(count)
    }

    /// Searches aliases by name or commands
    pub async fn search_aliases(
        &self,
        search_term: &str,
        page: u64,
        per_page: u64,
    ) -> Result<Vec<alias_entity::Model>, Error> {
        let offset = page * per_page;
        let search_pattern = format!("%{}%", search_term);

        let aliases = alias_entity::Entity::find()
            .filter(
                alias_entity::Column::Name
                    .like(&search_pattern)
                    .or(alias_entity::Column::Commands.like(&search_pattern)),
            )
            .order_by_asc(alias_entity::Column::Name)
            .offset(offset)
            .limit(per_page)
            .all(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to search aliases: {}", e)))?;

        Ok(aliases)
    }

    /// Counts aliases matching search term
    pub async fn count_search_aliases(&self, search_term: &str) -> Result<u64, Error> {
        let search_pattern = format!("%{}%", search_term);

        let count = alias_entity::Entity::find()
            .filter(
                alias_entity::Column::Name
                    .like(&search_pattern)
                    .or(alias_entity::Column::Commands.like(&search_pattern)),
            )
            .count(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to count search results: {}", e)))?;

        Ok(count)
    }

    /// Finds aliases that contain a specific sound code in their commands
    pub async fn find_aliases_containing_sound(
        &self,
        sound_code: &str,
    ) -> Result<Vec<alias_entity::Model>, Error> {
        // Search for the sound code in various formats used in aliases
        let search_patterns = vec![
            sound_code.to_lowercase(), // Lowercase version
            sound_code.to_uppercase(), // Uppercase version
        ];

        let mut found_aliases = Vec::new();

        for pattern in search_patterns {
            let aliases = alias_entity::Entity::find()
                .filter(alias_entity::Column::Commands.like(&format!("%{}%", pattern)))
                .all(&self.db)
                .await
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to search aliases for sound code: {}", e))
                })?;

            for alias in aliases {
                // Avoid duplicates
                if !found_aliases
                    .iter()
                    .any(|a: &alias_entity::Model| a.name == alias.name)
                {
                    found_aliases.push(alias);
                }
            }
        }

        Ok(found_aliases)
    }
}
