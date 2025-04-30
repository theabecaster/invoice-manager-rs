use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;

/// Database connection pool
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new Database instance with a connection pool
    pub async fn new(config: &Config) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(config.database_url())
            .await?;
            
        Ok(Self { pool })
    }
    
    /// Get a reference to the connection pool
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Initialize the database connection pool
pub async fn init(config: &Config) -> Result<Database> {
    let db = Database::new(config).await?;
    
    // Run migrations if you want to automatically migrate the database
    // sqlx::migrate!().run(db.get_pool()).await?;
    
    Ok(db)
} 