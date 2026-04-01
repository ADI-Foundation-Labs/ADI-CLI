mod types;

pub use types::{
    FundingDefaults, OperatorsConfig, OwnershipDefaults, S3Config, ToolkitDefaults,
    VerificationDefaults,
};

use std::path::{Path, PathBuf};

use crate::error::{Result, WrapErr};
use adi_ecosystem::EcosystemDefaults;
use adi_state::BackendType;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE_NAME: &str = ".adi.yml";
pub const DEFAULT_STATE_DIR: &str = ".adi_cli/state";
/// Environment variable for specifying config file path.
pub const CONFIG_ENV_VAR: &str = "ADI_CONFIG";

fn default_gas_multiplier() -> u64 {
    200
}

fn default_protocol_version() -> Option<String> {
    Some("v0.30.1".to_string())
}

/// Get the default state directory path.
pub fn default_state_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(DEFAULT_STATE_DIR))
        .unwrap_or_else(|| PathBuf::from("/home/user").join(DEFAULT_STATE_DIR))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// State directory path for storing ecosystem and chain data.
    /// Default: `~/.adi_cli/state`
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,

    /// Enable debug logging.
    /// Default: `false`
    #[serde(default)]
    pub debug: bool,

    /// Default protocol version for toolkit Docker image.
    /// Used by init, add, and deploy commands when --protocol-version is not provided.
    /// Can be overridden with --protocol-version or ADI__PROTOCOL_VERSION env var.
    /// Default: `v0.30.1`
    #[serde(default = "default_protocol_version")]
    pub protocol_version: Option<String>,

    /// Default ecosystem configuration values.
    /// Includes nested chain configurations with operators, funding, and ownership.
    #[serde(default)]
    pub ecosystem: EcosystemDefaults,

    /// State backend type for persistence.
    /// Default: `filesystem`
    #[serde(default)]
    pub state_backend: BackendType,

    /// Default funding configuration values.
    /// These can be overridden by CLI flags.
    #[serde(default)]
    pub funding: FundingDefaults,

    /// Default toolkit configuration values.
    /// These can be overridden by CLI flags.
    #[serde(default)]
    pub toolkit: ToolkitDefaults,

    /// Default ownership transfer configuration values.
    /// **Deprecated**: Use `ecosystem.ownership` for ecosystem-level
    /// and `ecosystem.chains[].ownership` for chain-level ownership.
    #[serde(default)]
    pub ownership: OwnershipDefaults,

    /// Default verification configuration values.
    /// These can be overridden by CLI flags.
    #[serde(default)]
    pub verification: VerificationDefaults,

    /// Gas price multiplier percentage (default: 120 = 20% buffer).
    /// Applied to all on-chain transactions.
    /// Can be overridden with --gas-multiplier flag.
    #[serde(default = "default_gas_multiplier")]
    pub gas_multiplier: u64,

    /// S3 synchronization configuration.
    /// Enables syncing ecosystem state to S3-compatible storage.
    #[serde(default)]
    pub s3: S3Config,

    /// Predefined operator addresses.
    /// **Deprecated**: Use `ecosystem.chains[].operators` instead.
    #[serde(default)]
    pub operators: OperatorsConfig,
}

impl Config {
    /// Load configuration from file and environment variables.
    ///
    /// Configuration sources (mutually exclusive for files):
    /// 1. CLI `--config` flag (if provided, used exclusively)
    /// 2. `ADI_CONFIG` environment variable (if set, used exclusively)
    /// 3. Global config file at `~/.adi.yml` (fallback)
    ///
    /// Environment variables with `ADI__` prefix always override any file config.
    ///
    /// # Arguments
    /// * `config_path` - Optional path to config file from CLI `--config` flag
    ///
    /// # Errors
    /// Returns an error if configuration cannot be loaded or deserialized.
    /// If a config path is explicitly provided (via CLI or `ADI_CONFIG` env var)
    /// and the file doesn't exist, an error is returned.
    pub fn new(config_path: Option<&Path>) -> Result<Self> {
        let mut builder = config::Config::builder();

        // Determine which config file to use (mutually exclusive, not merged)
        // Priority: CLI --config > ADI_CONFIG env var > global ~/.adi.yml
        if let Some(path) = config_path {
            // CLI --config flag takes highest priority
            builder = builder.add_source(config::File::from(path).required(true));
        } else if let Ok(env_path) = std::env::var(CONFIG_ENV_VAR) {
            // ADI_CONFIG environment variable
            builder = builder.add_source(config::File::from(Path::new(&env_path)).required(true));
        } else {
            // Fall back to global config
            let global_config_path = path_with_home_dir(DEFAULT_CONFIG_FILE_NAME);
            builder = builder
                .add_source(config::File::from(Path::new(&global_config_path)).required(false));
        }

        // Environment variables ADI__* always override (highest priority)
        let mut config: Self = builder
            .add_source(
                config::Environment::with_prefix("ADI")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()
            .wrap_err("Failed to build config")?
            .try_deserialize()
            .wrap_err("Failed to deserialize config")?;

        // Expand ~ in state_dir to user's home directory
        config.state_dir = expand_tilde(&config.state_dir);

        Ok(config)
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

/// Expand tilde (~) to home directory in a path.
///
/// If the path starts with `~`, it is replaced with the user's home directory.
/// Otherwise, the path is returned unchanged.
fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}
