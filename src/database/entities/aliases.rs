use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub name: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub commands: String,
}
