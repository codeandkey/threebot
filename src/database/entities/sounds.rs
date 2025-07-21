use sea_orm::entity::prelude::*;
use sea_orm::Set; 
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sounds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub code: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub source_url: Option<String>,
    pub start_time: String,
    pub length: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Creates a new sound model
    pub fn new(
        code: String,
        author: String,
        source_url: Option<String>,
        start_time: String,
        length: f64,
    ) -> Self {
        Self {
            code,
            author,
            created_at: Utc::now(),
            source_url,
            start_time,
            length,
        }
    }
}

impl ActiveModel {
    /// Creates a new ActiveModel for insertion
    pub fn new_for_insert(
        code: String,
        author: String,
        source_url: Option<String>,
        start_time: String,
        length: f64,
    ) -> Self {
        Self {
            code: Set(code),
            author: Set(author),
            created_at: Set(Utc::now()),
            source_url: Set(source_url),
            start_time: Set(start_time),
            length: Set(length),
        }
    }
}
