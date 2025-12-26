// =============================================================================
// CONFIGURATION MODULE
// =============================================================================
// This module handles loading configuration from environment variables.
//
// LEARNING NOTES:
// - Environment variables are the standard way to configure containers
// - We parse them into a strongly-typed Config struct
// - This makes configuration errors obvious at startup, not runtime
// =============================================================================

use anyhow::{Context, Result};
use std::env;

// -----------------------------------------------------------------------------
// CONFIG STRUCT
// -----------------------------------------------------------------------------
// This struct holds all configuration values for the service.
// Each field corresponds to an environment variable.
//
// LEARNING NOTE:
// Using a struct instead of raw env::var() calls everywhere has benefits:
// 1. Type safety: PORT is u16, not String
// 2. Validation: Errors happen at startup, not later
// 3. Documentation: All config options are in one place
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP server port (default: 8002)
    pub port: u16,
    
    /// PostgreSQL connection URL
    /// Format: postgres://user:password@host:port/database
    pub database_url: String,
    
    /// Redis connection URL
    /// Format: redis://:password@host:port/db_number
    pub redis_url: String,
}

impl Config {
    // -------------------------------------------------------------------------
    // LOAD CONFIGURATION FROM ENVIRONMENT
    // -------------------------------------------------------------------------
    /// Creates a Config by reading environment variables.
    /// 
    /// # Returns
    /// - `Ok(Config)` if all required variables are set
    /// - `Err` if any required variable is missing
    ///
    /// # Example
    /// ```
    /// let config = Config::from_env()?;
    /// println!("Server will listen on port {}", config.port);
    /// ```
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            // -----------------------------------------------------------------
            // PORT
            // -----------------------------------------------------------------
            // Read PORT env var, default to "8002" if not set
            // Then parse the string to u16 (unsigned 16-bit integer)
            //
            // LEARNING NOTE:
            // .context() adds helpful error messages when parsing fails
            // Instead of "invalid digit", you get "Failed to parse PORT"
            port: env::var("PORT")
                .unwrap_or_else(|_| "8002".to_string())
                .parse()
                .context("Failed to parse PORT as a number")?,
            
            // -----------------------------------------------------------------
            // DATABASE_URL
            // -----------------------------------------------------------------
            // Required - no default value
            // .context() provides a clear error message if missing
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL environment variable is required")?,
            
            // -----------------------------------------------------------------
            // REDIS_URL
            // -----------------------------------------------------------------
            // Required - no default value
            redis_url: env::var("REDIS_URL")
                .context("REDIS_URL environment variable is required")?,
        })
    }
}

// =============================================================================
// TESTS
// =============================================================================
// Unit tests for the configuration module.
//
// LEARNING NOTE:
// In Rust, tests live in the same file as the code they test.
// The #[cfg(test)] attribute means this code only compiles during testing.
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_from_env() {
        // Set up test environment
        env::set_var("PORT", "9000");
        env::set_var("DATABASE_URL", "postgres://test:test@localhost/test");
        env::set_var("REDIS_URL", "redis://localhost:6379");

        // Load config
        let config = Config::from_env().expect("Failed to load config");

        // Verify values
        assert_eq!(config.port, 9000);
        assert!(config.database_url.contains("postgres://"));
        assert!(config.redis_url.contains("redis://"));

        // Clean up
        env::remove_var("PORT");
        env::remove_var("DATABASE_URL");
        env::remove_var("REDIS_URL");
    }
}
