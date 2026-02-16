use crate::database::connection::DbPool;
use crate::database::entities::user_settings::{self as user_settings_entity, SettingType};
use crate::error::Error;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};

#[derive(Clone)]
pub struct UserSettingsManager {
    db: DbPool,
}

impl UserSettingsManager {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, Error> {
        chrono::DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                Error::DatabaseError(format!(
                    "Invalid user settings timestamp '{}': {}",
                    value, e
                ))
            })
    }

    /// Set a user setting (bind, greeting, farewell)
    pub async fn set_user_setting(
        &self,
        username: &str,
        setting_type: SettingType,
        value: &str,
    ) -> Result<(), Error> {
        let pool = self.db.clone();
        let username = username.to_string();
        let setting_type = setting_type.as_str().to_string();
        let value = value.to_string();

        tokio::task::spawn_blocking(move || -> Result<(), Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let id = format!("{}:{}", username, setting_type);
            let now = Utc::now().to_rfc3339();

            let existing: Option<String> = conn
                .query_row(
                    "SELECT id FROM user_settings WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to check existing user setting: {}", e))
                })?;

            if existing.is_some() {
                conn.execute(
                    "UPDATE user_settings SET setting_value = ?1, updated_at = ?2 WHERE id = ?3",
                    params![value, now, id],
                )
                .map_err(|e| Error::DatabaseError(format!("Failed to update user setting: {}", e)))?;
            } else {
                conn.execute(
                    "INSERT INTO user_settings (id, username, setting_type, setting_value, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![id, username, setting_type, value, now, now],
                )
                .map_err(|e| Error::DatabaseError(format!("Failed to insert user setting: {}", e)))?;
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Set user setting task failed: {}", e)))?
    }

    /// Get a user setting by type
    pub async fn get_user_setting(
        &self,
        username: &str,
        setting_type: SettingType,
    ) -> Result<Option<String>, Error> {
        let pool = self.db.clone();
        let id = format!("{}:{}", username, setting_type.as_str());

        tokio::task::spawn_blocking(move || -> Result<Option<String>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;

            conn.query_row(
                "SELECT setting_value FROM user_settings WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Error::DatabaseError(format!("Failed to get user setting: {}", e)))
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Get user setting task failed: {}", e)))?
    }

    /// Delete a user setting
    pub async fn delete_user_setting(
        &self,
        username: &str,
        setting_type: SettingType,
    ) -> Result<bool, Error> {
        let pool = self.db.clone();
        let id = format!("{}:{}", username, setting_type.as_str());

        tokio::task::spawn_blocking(move || -> Result<bool, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let rows = conn
                .execute("DELETE FROM user_settings WHERE id = ?1", params![id])
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to delete user setting: {}", e))
                })?;
            Ok(rows > 0)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Delete user setting task failed: {}", e)))?
    }

    /// Check if a user has a specific setting
    pub async fn user_has_setting(
        &self,
        username: &str,
        setting_type: SettingType,
    ) -> Result<bool, Error> {
        let pool = self.db.clone();
        let id = format!("{}:{}", username, setting_type.as_str());

        tokio::task::spawn_blocking(move || -> Result<bool, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM user_settings WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to check user setting existence: {}", e))
                })?;
            Ok(count > 0)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("Has user setting task failed: {}", e)))?
    }

    /// Get all settings for a user
    pub async fn get_all_user_settings(
        &self,
        username: &str,
    ) -> Result<Vec<user_settings_entity::Model>, Error> {
        let pool = self.db.clone();
        let username = username.to_string();

        tokio::task::spawn_blocking(move || -> Result<Vec<user_settings_entity::Model>, Error> {
            let conn = pool
                .get()
                .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, username, setting_type, setting_value, created_at, updated_at
                     FROM user_settings WHERE username = ?1",
                )
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to prepare user settings query: {}", e))
                })?;

            let rows = stmt
                .query_map(params![username], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                })
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to query user settings: {}", e))
                })?;

            let mut settings = Vec::new();
            for row in rows {
                let (id, username, setting_type, setting_value, created_at_raw, updated_at_raw) =
                    row.map_err(|e| {
                        Error::DatabaseError(format!("Failed to read user setting row: {}", e))
                    })?;
                settings.push(user_settings_entity::Model {
                    id,
                    username,
                    setting_type,
                    setting_value,
                    created_at: Self::parse_timestamp(&created_at_raw)?,
                    updated_at: Self::parse_timestamp(&updated_at_raw)?,
                });
            }
            Ok(settings)
        })
        .await
        .map_err(|e| Error::DatabaseError(format!("List user settings task failed: {}", e)))?
    }

    /// Convenience methods for specific setting types
    pub async fn set_bind(&self, username: &str, command: &str) -> Result<(), Error> {
        self.set_user_setting(username, SettingType::Bind, command)
            .await
    }

    pub async fn get_bind(&self, username: &str) -> Result<Option<String>, Error> {
        self.get_user_setting(username, SettingType::Bind).await
    }

    pub async fn set_greeting(&self, username: &str, command: &str) -> Result<(), Error> {
        self.set_user_setting(username, SettingType::Greeting, command)
            .await
    }

    pub async fn get_greeting(&self, username: &str) -> Result<Option<String>, Error> {
        self.get_user_setting(username, SettingType::Greeting).await
    }

    pub async fn set_farewell(&self, username: &str, command: &str) -> Result<(), Error> {
        self.set_user_setting(username, SettingType::Farewell, command)
            .await
    }

    pub async fn get_farewell(&self, username: &str) -> Result<Option<String>, Error> {
        self.get_user_setting(username, SettingType::Farewell).await
    }

    pub async fn clear_greeting(&self, username: &str) -> Result<bool, Error> {
        self.delete_user_setting(username, SettingType::Greeting)
            .await
    }

    pub async fn clear_farewell(&self, username: &str) -> Result<bool, Error> {
        self.delete_user_setting(username, SettingType::Farewell)
            .await
    }
}
