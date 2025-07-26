use sea_orm::Set;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String, // Format: "username:setting_type" e.g., "john:bind", "john:greeting"
    pub username: String,
    pub setting_type: String,  // "bind", "greeting", "farewell"
    pub setting_value: String, // The actual command/sound to execute
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    pub fn new_for_user_setting(username: &str, setting_type: &str, setting_value: &str) -> Self {
        let now = chrono::Utc::now();
        let id = format!("{}:{}", username, setting_type);

        Self {
            id: Set(id),
            username: Set(username.to_string()),
            setting_type: Set(setting_type.to_string()),
            setting_value: Set(setting_value.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }
}

// Enum for setting types to ensure consistency
#[derive(Debug, Clone, PartialEq)]
pub enum SettingType {
    Bind,
    Greeting,
    Farewell,
}

impl SettingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingType::Bind => "bind",
            SettingType::Greeting => "greeting",
            SettingType::Farewell => "farewell",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bind" => Some(SettingType::Bind),
            "greeting" => Some(SettingType::Greeting),
            "farewell" => Some(SettingType::Farewell),
            _ => None,
        }
    }
}
