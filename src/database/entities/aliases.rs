use sea_orm::entity::prelude::*;
use sea_orm::Set; 
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "aliases")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub commands: String, // JSON array of commands or space-separated string
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Creates a new alias model
    pub fn new(
        name: String,
        author: String,
        commands: String,
    ) -> Self {
        Self {
            name,
            author,
            created_at: Utc::now(),
            commands,
        }
    }
}

impl ActiveModel {
    /// Creates a new ActiveModel for insertion
    pub fn new_for_insert(
        name: String,
        author: String,
        commands: String,
    ) -> Self {
        Self {
            name: Set(name),
            author: Set(author),
            created_at: Set(Utc::now()),
            commands: Set(commands),
        }
    }
}
