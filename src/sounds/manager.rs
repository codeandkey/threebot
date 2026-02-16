use crate::database::connection::DbPool;
use crate::database::entities::sounds as sound_entity;
use crate::error::Error;
use crate::sounds::{SoundFile, validate_sound_code};
use chrono::Utc;
use rand::Rng;
use rusqlite::{OptionalExtension, params};
use std::path::{Path, PathBuf};

/// High-level manager for sound operations
pub struct SoundsManager {
    database: DbPool,
    sounds_dir: PathBuf,
}

impl SoundsManager {
    /// Creates a new SoundsManager from a database pool
    pub fn new(database: DbPool, sounds_dir: PathBuf) -> Result<Self, Error> {
        std::fs::create_dir_all(&sounds_dir).map_err(|e| {
            Error::DatabaseError(format!("Failed to create sounds directory: {}", e))
        })?;

        Ok(SoundsManager {
            database,
            sounds_dir,
        })
    }

    fn row_to_model(row: &rusqlite::Row<'_>) -> Result<sound_entity::Model, rusqlite::Error> {
        Ok(sound_entity::Model {
            code: row.get(0)?,
            author: row.get(1)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            source_url: row.get(3)?,
            start_time: row.get(4)?,
            length: row.get(5)?,
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
                format!(
                    "{}:{:02}:{:02}.{}",
                    hours,
                    minutes,
                    secs,
                    (fractional * 10.0) as u32
                )
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
        let pool = self.database.clone();
        let metadata = tokio::task::spawn_blocking(move || -> Result<Option<sound_entity::Model>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            conn.query_row(
                "SELECT code, author, created_at, source_url, start_time, length FROM sounds WHERE code = ?1",
                params![code_upper],
                Self::row_to_model,
            )
            .optional()
            .map_err(|e| Error::DatabaseError(format!("Failed to query sound: {}", e)))
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Get sound task failed: {}", e)))??;

        if metadata.is_none() {
            return Ok(None);
        }

        let mut sound_file = SoundFile::new(code.to_uppercase(), &self.sounds_dir);
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
        let sound_file = SoundFile::new(code_upper.clone(), &self.sounds_dir);
        if !sound_file.exists() {
            return Err(Error::InvalidInput(format!(
                "Sound file does not exist: {}",
                sound_file.file_path.display()
            )));
        }

        let start_time_str = Self::format_timestamp(start_time);
        let created_at = Utc::now().to_rfc3339();
        let pool = self.database.clone();

        tokio::task::spawn_blocking(move || -> Result<(), Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            conn.execute(
                "INSERT INTO sounds (code, author, created_at, source_url, start_time, length)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    code_upper,
                    author,
                    created_at,
                    source_url,
                    start_time_str,
                    length
                ],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to insert sound: {}", e)))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Add sound task failed: {}", e)))??;

        info!("Added sound with code: {}", code);
        Ok(())
    }

    /// Removes a sound from the database and deletes the file from disk
    pub async fn remove_sound(&self, code: &str) -> Result<(), Error> {
        if !validate_sound_code(code) {
            return Err(Error::InvalidInput(format!("Invalid sound code: {}", code)));
        }

        let code_upper = code.to_uppercase();

        let sound_file = match self.get_sound(&code_upper).await? {
            Some(sound) => sound,
            None => return Err(Error::InvalidInput(format!("Sound not found: {}", code))),
        };

        if sound_file.exists() {
            if let Err(e) = std::fs::remove_file(&sound_file.file_path) {
                warn!(
                    "Failed to delete sound file {:?}: {}",
                    sound_file.file_path, e
                );
                return Err(Error::IOError(e));
            }
            info!("Deleted sound file: {:?}", sound_file.file_path);
        } else {
            warn!(
                "Sound file {:?} does not exist on disk",
                sound_file.file_path
            );
        }

        let pool = self.database.clone();
        let rows_affected = tokio::task::spawn_blocking(move || -> Result<usize, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            conn.execute("DELETE FROM sounds WHERE code = ?1", params![code_upper])
                .map_err(|e| Error::DatabaseError(format!("Failed to delete sound: {}", e)))
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Remove sound task failed: {}", e)))??;
        if rows_affected == 0 {
            return Err(Error::InvalidInput(format!(
                "Sound not found in database: {}",
                code
            )));
        }

        info!("Removed sound with code: {}", code);
        Ok(())
    }

    /// Lists all sounds in the database, ordered by most recently created
    pub async fn list_sounds(&self) -> Result<Vec<sound_entity::Model>, Error> {
        let pool = self.database.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<sound_entity::Model>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let mut stmt = conn
                .prepare(
                    "SELECT code, author, created_at, source_url, start_time, length
                     FROM sounds
                     ORDER BY created_at DESC",
                )
                .map_err(|e| Error::DatabaseError(format!("Failed to list sounds: {}", e)))?;

            let rows = stmt
                .query_map([], Self::row_to_model)
                .map_err(|e| Error::DatabaseError(format!("Failed to list sounds: {}", e)))?;

            let mut sounds = Vec::new();
            for row in rows {
                sounds.push(row.map_err(|e| {
                    Error::DatabaseError(format!("Failed to read sound row: {}", e))
                })?);
            }
            Ok(sounds)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("List sounds task failed: {}", e)))?
    }

    /// Gets a random sound from the database
    pub async fn get_random_sound(&self) -> Result<Option<SoundFile>, Error> {
        let pool = self.database.clone();
        let count = tokio::task::spawn_blocking(move || -> Result<i64, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            conn.query_row("SELECT COUNT(*) FROM sounds", [], |row| row.get(0))
                .map_err(|e| Error::DatabaseError(format!("Failed to count sounds: {}", e)))
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Count sounds task failed: {}", e)))??;

        if count == 0 {
            return Ok(None);
        }

        let offset = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..count)
        };

        let pool = self.database.clone();
        let model =
            tokio::task::spawn_blocking(move || -> Result<Option<sound_entity::Model>, Error> {
                let conn = pool
                    .get()
                    .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
                conn.query_row(
                    "SELECT code, author, created_at, source_url, start_time, length
                 FROM sounds
                 LIMIT 1 OFFSET ?1",
                    params![offset],
                    Self::row_to_model,
                )
                .optional()
                .map_err(|e| Error::DatabaseError(format!("Failed to get random sound: {}", e)))
            })
            .await
            .map_err(|e| Error::DatabaseError(format!("Random sound task failed: {}", e)))??;

        if let Some(metadata) = model {
            let mut sound_file = SoundFile::new(metadata.code.clone(), &self.sounds_dir);
            sound_file.metadata = Some(metadata);
            Ok(Some(sound_file))
        } else {
            Ok(None)
        }
    }

    /// Gets the sounds directory path
    pub fn sounds_dir(&self) -> &Path {
        &self.sounds_dir
    }
}
