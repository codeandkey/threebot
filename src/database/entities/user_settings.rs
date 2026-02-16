#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub id: String,
    pub username: String,
    pub setting_type: String,
    pub setting_value: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
