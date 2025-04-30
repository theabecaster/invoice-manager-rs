mod config;
mod db;
mod models;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = config::init()?;
    println!("Database URL: {}", config.database_url());
    
    // Initialize database connection
    let db = db::init(&config).await?;
    println!("Database connection established successfully");
    
    // Simple test query to verify connection is working
    let pool = db.get_pool();
    let result = sqlx::query_scalar!("SELECT 1 + 1 as sum")
        .fetch_one(pool)
        .await?;
    
    println!("Test query result: {:?}", result);
    
    Ok(())
}
