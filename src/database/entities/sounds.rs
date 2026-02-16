use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub code: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub source_url: Option<String>,
    pub start_time: String,
    pub length: f64,
}
