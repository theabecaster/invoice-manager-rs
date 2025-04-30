use anyhow::Result;
use dotenvy::dotenv;
use serde::Deserialize;

/// Configuration for the application
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Database connection URL
    pub database_url: String,
}

impl Config {
    /// Load configuration from environment variables
    /// 
    /// This function will:
    /// 1. Load variables from .env file if it exists
    /// 2. Deserialize environment variables into Config struct
    pub fn load() -> Result<Self> {
        // Load .env file if it exists
        dotenv().ok();

        // Parse environment variables into Config struct
        let config = envy::from_env::<Config>()?;
        
        Ok(config)
    }

    /// Get a direct reference to the database URL
    pub fn database_url(&self) -> &str {
        &self.database_url
    }
}

/// Initialize environment variables and load configuration
pub fn init() -> Result<Config> {
    // Ensure .env file is loaded
    dotenv().ok();
    
    // Load the configuration
    let config = Config::load()?;
    
    Ok(config)
} 