use crate::error::Error;
use sea_orm::*;

/// Runs all database migrations
pub async fn run_all_migrations(db: &DatabaseConnection) -> Result<(), Error> {
    migrate_sounds_table(db).await?;
    migrate_aliases_table(db).await?;
    info!("All database migrations completed successfully");
    Ok(())
}

/// Migrates the sounds table
async fn migrate_sounds_table(db: &DatabaseConnection) -> Result<(), Error> {
    use sea_orm::Schema;
    use super::entities::sounds;

    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

    // Create the sounds table if it doesn't exist
    let stmt = schema.create_table_from_entity(sounds::Entity);
    
    match db.execute(builder.build(&stmt)).await {
        Ok(_) => {
            info!("Sounds table migration completed successfully");
            Ok(())
        }
        Err(e) => {
            // Ignore "table already exists" errors
            if e.to_string().contains("already exists") {
                info!("Sounds table already exists");
                Ok(())
            } else {
                Err(Error::DatabaseError(format!("Failed to create sounds table: {}", e)))
            }
        }
    }
}

/// Migrates the aliases table
async fn migrate_aliases_table(db: &DatabaseConnection) -> Result<(), Error> {
    use sea_orm::Schema;
    use super::entities::aliases;

    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

    // Create the aliases table if it doesn't exist
    let stmt = schema.create_table_from_entity(aliases::Entity);
    
    match db.execute(builder.build(&stmt)).await {
        Ok(_) => {
            info!("Aliases table migration completed successfully");
            Ok(())
        }
        Err(e) => {
            // Ignore "table already exists" errors
            if e.to_string().contains("already exists") {
                info!("Aliases table already exists");
                Ok(())
            } else {
                Err(Error::DatabaseError(format!("Failed to create aliases table: {}", e)))
            }
        }
    }
}
