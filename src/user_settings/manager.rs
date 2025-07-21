use crate::database::entities::user_settings::{self as user_settings_entity, SettingType};
use crate::error::Error;
use sea_orm::{
    DatabaseConnection, EntityTrait, Set, ColumnTrait, QueryFilter,
    ActiveModelTrait, PaginatorTrait,
};

#[derive(Clone)]
pub struct UserSettingsManager {
    db: DatabaseConnection,
}

impl UserSettingsManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Set a user setting (bind, greeting, farewell)
    pub async fn set_user_setting(&self, username: &str, setting_type: SettingType, value: &str) -> Result<(), Error> {
        let id = format!("{}:{}", username, setting_type.as_str());
        
        // Check if setting already exists
        let existing = user_settings_entity::Entity::find_by_id(&id)
            .one(&self.db)
            .await?;

        if let Some(existing_model) = existing {
            // Update existing setting
            let mut active_model: user_settings_entity::ActiveModel = existing_model.into();
            active_model.setting_value = Set(value.to_string());
            active_model.updated_at = Set(chrono::Utc::now());
            active_model.update(&self.db).await?;
        } else {
            // Create new setting
            let new_setting = user_settings_entity::ActiveModel::new_for_user_setting(
                username, 
                setting_type.as_str(), 
                value
            );
            new_setting.insert(&self.db).await?;
        }

        Ok(())
    }

    /// Get a user setting by type
    pub async fn get_user_setting(&self, username: &str, setting_type: SettingType) -> Result<Option<String>, Error> {
        let id = format!("{}:{}", username, setting_type.as_str());
        
        let setting = user_settings_entity::Entity::find_by_id(&id)
            .one(&self.db)
            .await?;

        Ok(setting.map(|s| s.setting_value))
    }

    /// Delete a user setting
    pub async fn delete_user_setting(&self, username: &str, setting_type: SettingType) -> Result<bool, Error> {
        let id = format!("{}:{}", username, setting_type.as_str());
        
        let result = user_settings_entity::Entity::delete_by_id(&id)
            .exec(&self.db)
            .await?;

        Ok(result.rows_affected > 0)
    }

    /// Check if a user has a specific setting
    pub async fn user_has_setting(&self, username: &str, setting_type: SettingType) -> Result<bool, Error> {
        let id = format!("{}:{}", username, setting_type.as_str());
        
        let count = user_settings_entity::Entity::find_by_id(&id)
            .count(&self.db)
            .await?;

        Ok(count > 0)
    }

    /// Get all settings for a user
    pub async fn get_all_user_settings(&self, username: &str) -> Result<Vec<user_settings_entity::Model>, Error> {
        let settings = user_settings_entity::Entity::find()
            .filter(user_settings_entity::Column::Username.eq(username))
            .all(&self.db)
            .await?;

        Ok(settings)
    }

    /// Convenience methods for specific setting types

    pub async fn set_bind(&self, username: &str, command: &str) -> Result<(), Error> {
        self.set_user_setting(username, SettingType::Bind, command).await
    }

    pub async fn get_bind(&self, username: &str) -> Result<Option<String>, Error> {
        self.get_user_setting(username, SettingType::Bind).await
    }

    pub async fn set_greeting(&self, username: &str, command: &str) -> Result<(), Error> {
        self.set_user_setting(username, SettingType::Greeting, command).await
    }

    pub async fn get_greeting(&self, username: &str) -> Result<Option<String>, Error> {
        self.get_user_setting(username, SettingType::Greeting).await
    }

    pub async fn set_farewell(&self, username: &str, command: &str) -> Result<(), Error> {
        self.set_user_setting(username, SettingType::Farewell, command).await
    }

    pub async fn get_farewell(&self, username: &str) -> Result<Option<String>, Error> {
        self.get_user_setting(username, SettingType::Farewell).await
    }

    pub async fn clear_greeting(&self, username: &str) -> Result<bool, Error> {
        self.delete_user_setting(username, SettingType::Greeting).await
    }

    pub async fn clear_farewell(&self, username: &str) -> Result<bool, Error> {
        self.delete_user_setting(username, SettingType::Farewell).await
    }
}
