use std::path::PathBuf;

use crate::error::{Result, WrapErr};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE_NAME: &str = ".adi.yml";
pub const DEFAULT_STATE_DIR: &str = ".adi_cli/state";
pub const LOCAL_CONFIG_FILE_NAME: &str = ".adi.yml";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// State directory path for storing ecosystem and chain data.
    /// Default: `~/.adi_cli/state`
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,
}

impl Config {
    /// Load configuration from file and environment variables.
    ///
    /// Configuration is loaded from multiple sources with the following priority:
    /// 1. Environment variables with `ADI_` prefix (highest)
    /// 2. Local config file at `./.adi.yml` (current directory)
    /// 3. Global config file at `~/.adi_cli/.adi.yml`
    /// 4. Built-in defaults (lowest)
    ///
    /// Local config values override global config values, allowing project-specific
    /// configuration for development and testing.
    ///
    /// # Errors
    /// Returns an error if configuration cannot be loaded or deserialized.
    pub fn new() -> Result<Self> {
        let global_config_path = path_with_home_dir(DEFAULT_CONFIG_FILE_NAME);

        let mut builder = config::Config::builder()
            // 1. Global config (lowest file priority)
            .add_source(
                config::File::from(std::path::Path::new(&global_config_path)).required(false),
            );

        // 2. Local config (higher priority, overrides global)
        if let Ok(current_dir) = std::env::current_dir() {
            let local_config_path = current_dir.join(LOCAL_CONFIG_FILE_NAME);
            builder = builder.add_source(config::File::from(local_config_path).required(false));
        }

        // 3. Environment variables (highest priority)
        builder
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

fn default_state_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(DEFAULT_STATE_DIR))
        .unwrap_or_else(|| PathBuf::from("/home/user").join(DEFAULT_STATE_DIR))
}
