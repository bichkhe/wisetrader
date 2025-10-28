use sqlx::MySqlPool;
use sea_orm::{Database, DatabaseConnection};
use anyhow::Result;
use tracing::info;

pub async fn get_pool(database_url: &str) -> Result<MySqlPool> {
    info!("Connecting to database at: {}", database_url);
    let pool = MySqlPool::connect(database_url).await?;
    Ok(pool)
}

pub async fn get_db_connection(database_url: &str) -> Result<DatabaseConnection> {
    info!("Connecting to database via Sea-ORM at: {}", database_url);
    let db = Database::connect(database_url).await?;
    Ok(db)
}

pub type DbPool = MySqlPool;
