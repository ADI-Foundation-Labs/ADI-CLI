use crate::error::{Result, WrapErr};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE_NAME: &str = ".adi.yml";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {}

impl Config {
    /// Load configuration from file and environment variables.
    ///
    /// Configuration is loaded from multiple sources with the following priority:
    /// 1. Environment variables with `ADI_` prefix (highest)
    /// 2. Config file at `~/.adi_cli/.adi.yml`
    /// 3. Built-in defaults (lowest)
    ///
    /// # Errors
    /// Returns an error if configuration cannot be loaded or deserialized.
    pub fn new() -> Result<Self> {
        let config_path = path_with_home_dir(DEFAULT_CONFIG_FILE_NAME);
        let config_path = std::path::Path::new(&config_path);
        config::Config::builder()
            .add_source(config::File::from(config_path).required(false))
            .add_source(
                config::Environment::with_prefix("ADI")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()
            .wrap_err("Failed to build config")?
            .try_deserialize()
            .wrap_err("Failed to deserialize config")
    }
}

/// Expand a path relative to the user's home directory.
///
/// # Arguments
/// * `path` - Relative path to append to home directory
///
/// # Returns
/// Full path with home directory prefix
pub fn path_with_home_dir(path: &str) -> String {
    let home_dir = dirs::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|| "/home/user".to_string());
    format!("{home_dir}/{path}")
}
