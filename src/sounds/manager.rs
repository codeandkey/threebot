use sea_orm::*;
use std::path::{Path, PathBuf};
use crate::error::Error;
use crate::sounds::{SoundFile, validate_sound_code};
use crate::database::entities::sounds as sound_entity;

/// High-level manager for sound operations
pub struct SoundsManager {
    database: DatabaseConnection,
    sounds_dir: PathBuf,
}

impl SoundsManager {
    /// Creates a new SoundsManager from a database connection
    pub fn new(database: DatabaseConnection, sounds_dir: PathBuf) -> Result<Self, Error> {
        // Ensure sounds directory exists
        std::fs::create_dir_all(&sounds_dir)
            .map_err(|e| Error::DatabaseError(format!("Failed to create sounds directory: {}", e)))?;

        Ok(SoundsManager {
            database,
            sounds_dir,
        })
    }

    /// Converts seconds to timestamp format (MM:SS or H:MM:SS)
    fn format_timestamp(seconds: f64) -> String {
        let total_seconds = seconds as u64;
        let fractional = seconds - total_seconds as f64;
        
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;
        
        if hours > 0 {
            if fractional > 0.0 {
                format!("{}:{:02}:{:02}.{}", hours, minutes, secs, (fractional * 10.0) as u32)
            } else {
                format!("{}:{:02}:{:02}", hours, minutes, secs)
            }
        } else if fractional > 0.0 {
            format!("{}:{:02}.{}", minutes, secs, (fractional * 10.0) as u32)
        } else {
            format!("{}:{:02}", minutes, secs)
        }
    }

    /// Gets a sound by its code
    pub async fn get_sound(&self, code: &str) -> Result<Option<SoundFile>, Error> {
        if !validate_sound_code(code) {
            return Err(Error::InvalidInput(format!("Invalid sound code: {}", code)));
        }

        let code_upper = code.to_uppercase();
        
        // Get metadata from database
        let metadata = sound_entity::Entity::find_by_id(&code_upper)
            .one(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to query sound: {}", e)))?;

        // If no metadata found, return None
        if metadata.is_none() {
            return Ok(None);
        }

        let mut sound_file = SoundFile::new(code_upper, &self.sounds_dir);
        sound_file.metadata = metadata;

        Ok(Some(sound_file))
    }

    /// Adds a new sound to the database
    pub async fn add_sound(
        &self,
        code: &str,
        author: String,
        source_url: Option<String>,
        start_time: f64,
        length: f64,
    ) -> Result<(), Error> {
        if !validate_sound_code(code) {
            return Err(Error::InvalidInput(format!("Invalid sound code: {}", code)));
        }

        let code_upper = code.to_uppercase();

        // Check if sound file exists
        let sound_file = SoundFile::new(code_upper.clone(), &self.sounds_dir);
        if !sound_file.exists() {
            return Err(Error::InvalidInput(format!("Sound file does not exist: {}", sound_file.file_path.display())));
        }

        // Convert start_time from seconds to timestamp format
        let start_time_str = Self::format_timestamp(start_time);

        // Create new sound model
        let new_sound = sound_entity::ActiveModel::new_for_insert(
            code_upper,
            author,
            source_url,
            start_time_str,
            length,
        );

        // Insert into database
        sound_entity::Entity::insert(new_sound)
            .exec(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to insert sound: {}", e)))?;

        info!("Added sound with code: {}", code);
        Ok(())
    }

    /// Updates an existing sound's metadata
    pub async fn update_sound(
        &self,
        code: &str,
        author: Option<String>,
        source_url: Option<Option<String>>,
        start_time: Option<f64>,
        length: Option<f64>,
    ) -> Result<(), Error> {
        if !validate_sound_code(code) {
            return Err(Error::InvalidInput(format!("Invalid sound code: {}", code)));
        }

        let code_upper = code.to_uppercase();

        // Find existing sound
        let existing_sound = sound_entity::Entity::find_by_id(&code_upper)
            .one(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to query sound: {}", e)))?;

        let Some(existing) = existing_sound else {
            return Err(Error::InvalidInput(format!("Sound not found: {}", code)));
        };

        // Create update model
        let mut sound_update: sound_entity::ActiveModel = existing.into();

        if let Some(author) = author {
            sound_update.author = Set(author);
        }
        if let Some(source_url) = source_url {
            sound_update.source_url = Set(source_url);
        }
        if let Some(start_time) = start_time {
            sound_update.start_time = Set(Self::format_timestamp(start_time));
        }
        if let Some(length) = length {
            sound_update.length = Set(length);
        }

        // Update in database
        sound_entity::Entity::update(sound_update)
            .exec(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update sound: {}", e)))?;

        info!("Updated sound with code: {}", code);
        Ok(())
    }

    /// Removes a sound from the database (but not from disk)
    pub async fn remove_sound(&self, code: &str) -> Result<(), Error> {
        if !validate_sound_code(code) {
            return Err(Error::InvalidInput(format!("Invalid sound code: {}", code)));
        }

        let code_upper = code.to_uppercase();

        let result = sound_entity::Entity::delete_by_id(&code_upper)
            .exec(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to delete sound: {}", e)))?;

        if result.rows_affected == 0 {
            return Err(Error::InvalidInput(format!("Sound not found: {}", code)));
        }

        info!("Removed sound with code: {}", code);
        Ok(())
    }

    /// Lists all sounds in the database, ordered by most recently created
    pub async fn list_sounds(&self) -> Result<Vec<sound_entity::Model>, Error> {
        let sounds = sound_entity::Entity::find()
            .order_by_desc(sound_entity::Column::CreatedAt)
            .all(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to list sounds: {}", e)))?;

        Ok(sounds)
    }

    /// Gets a random sound from the database
    pub async fn get_random_sound(&self) -> Result<Option<SoundFile>, Error> {
        use rand::Rng;
        
        // Get count of all sounds
        let count = sound_entity::Entity::find()
            .count(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to count sounds: {}", e)))?;

        if count == 0 {
            return Ok(None);
        }

        // Generate random offset - do this before any await to avoid Send issues
        let offset = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..count)
        };

        // Get random sound using offset
        let random_sound = sound_entity::Entity::find()
            .offset(offset)
            .limit(1)
            .one(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to get random sound: {}", e)))?;

        if let Some(metadata) = random_sound {
            let mut sound_file = SoundFile::new(metadata.code.clone(), &self.sounds_dir);
            sound_file.metadata = Some(metadata);
            Ok(Some(sound_file))
        } else {
            Ok(None)
        }
    }

    /// Lists sounds by author
    pub async fn list_sounds_by_author(&self, author: &str) -> Result<Vec<sound_entity::Model>, Error> {
        let sounds = sound_entity::Entity::find()
            .filter(sound_entity::Column::Author.eq(author))
            .order_by_asc(sound_entity::Column::Code)
            .all(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to list sounds by author: {}", e)))?;

        Ok(sounds)
    }

    /// Searches for sounds that have the given string in their source URL
    pub async fn search_sounds_by_source(&self, search_term: &str) -> Result<Vec<sound_entity::Model>, Error> {
        let sounds = sound_entity::Entity::find()
            .filter(sound_entity::Column::SourceUrl.contains(search_term))
            .order_by_asc(sound_entity::Column::Code)
            .all(&self.database)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to search sounds: {}", e)))?;

        Ok(sounds)
    }

    /// Gets the sounds directory path
    pub fn sounds_dir(&self) -> &Path {
        &self.sounds_dir
    }

    /// Scans the sounds directory for files and returns codes that exist on disk but not in database
    pub async fn scan_orphaned_files(&self) -> Result<Vec<String>, Error> {
        let mut orphaned = Vec::new();

        // Read the sounds directory
        let entries = std::fs::read_dir(&self.sounds_dir)
            .map_err(|e| Error::DatabaseError(format!("Failed to read sounds directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| Error::DatabaseError(format!("Failed to read directory entry: {}", e)))?;
            
            let path = entry.path();
            
            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            // Check if it's an MP3 file
            if let Some(extension) = path.extension() {
                if extension != "mp3" {
                    continue;
                }
            } else {
                continue;
            }

            // Extract the code (filename without extension)
            if let Some(file_stem) = path.file_stem() {
                if let Some(code_str) = file_stem.to_str() {
                    let code = code_str.to_uppercase();
                    
                    // Validate the code format
                    if !validate_sound_code(&code) {
                        continue;
                    }

                    // Check if it exists in database
                    let exists = sound_entity::Entity::find_by_id(&code)
                        .one(&self.database)
                        .await
                        .map_err(|e| Error::DatabaseError(format!("Failed to query sound: {}", e)))?
                        .is_some();

                    if !exists {
                        orphaned.push(code);
                    }
                }
            }
        }

        Ok(orphaned)
    }

    /// Gets database health status
    pub async fn health_check(&self) -> Result<(), Error> {
        self.database
            .ping()
            .await
            .map_err(|e| Error::DatabaseError(format!("Database health check failed: {}", e)))?;
        Ok(())
    }
}
