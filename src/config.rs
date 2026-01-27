//! Application configuration with environment variable support.
//!
//! Configuration is loaded from multiple sources with the following priority (highest first):
//! 1. CLI flags (handled by Clap)
//! 2. Environment variables with `ADI_` prefix
//! 3. Config file at `~/.adi_cli/.adi.yml`
//! 4. Built-in defaults
//!
//! ## Environment Variables
//!
//! Nested configuration uses double underscore (`__`) as separator:
//!
//! | Variable                     | Config Path                |
//! |------------------------------|----------------------------|
//! | `ADI_STATE_DIR`              | `state_dir`                |
//! | `ADI_SETTLEMENT__RPC_URL`    | `settlement.rpc_url`       |
//! | `ADI_SETTLEMENT__GAS_PRICE`  | `settlement.gas_price`     |
//! | `ADI_FUNDER__PRIVATE_KEY`    | `funder.private_key`       |
//! | `ADI_FUNDER__CGT_ADDRESS`    | `funder.cgt_address`       |
//! | `ADI_ECOSYSTEM__NAME`        | `ecosystem.name`           |
//! | `ADI_ECOSYSTEM__CHAIN_NAME`  | `ecosystem.chain_name`     |
//! | `ADI_ECOSYSTEM__CHAIN_ID`    | `ecosystem.chain_id`       |
//! | `ADI_DOCKER__REGISTRY`       | `docker.registry`          |
//! | `ADI_DOCKER__IMAGE_NAME`     | `docker.image_name`        |

use crate::error::{Result, WrapErr};
use alloy_primitives::Address;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

pub const DEFAULT_CONFIG_FILE_NAME: &str = ".adi.yml";
pub const DEFAULT_STATE_DIR: &str = ".adi_cli/state";
pub const DEFAULT_SETTLEMENT_RPC_URL: &str = "http://localhost:8545";
pub const DEFAULT_DOCKER_REGISTRY: &str = "harbor.io/adi";
pub const DEFAULT_DOCKER_IMAGE_NAME: &str = "adi-toolkit";

/// Application configuration.
///
/// Loaded from config file, environment variables, and CLI flags.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// State directory path for storing ecosystem and chain data.
    /// Default: `~/.adi_cli/state`
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,

    /// Settlement layer configuration.
    #[serde(default)]
    pub settlement: SettlementConfig,

    /// Funder wallet configuration for auto-funding operations.
    /// Optional - if not set, wallets must be pre-funded manually.
    #[serde(default)]
    pub funder: Option<FunderConfig>,

    /// Ecosystem configuration.
    #[serde(default)]
    pub ecosystem: EcosystemConfig,

    /// Docker toolkit image configuration.
    #[serde(default)]
    pub docker: DockerConfig,
}

fn default_state_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(DEFAULT_STATE_DIR))
        .unwrap_or_else(|| PathBuf::from("/home/user").join(DEFAULT_STATE_DIR))
}

/// Settlement layer (L1) configuration.
///
/// Defines the connection to the settlement layer where ecosystem
/// and chain contracts are deployed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SettlementConfig {
    /// RPC endpoint URL for the settlement layer.
    /// Default: `http://localhost:8545`
    #[serde(default = "default_rpc_url")]
    pub rpc_url: String,

    /// Gas price in wei for transactions.
    /// If not set, gas price is determined automatically.
    #[serde(default)]
    pub gas_price: Option<u64>,
}

fn default_rpc_url() -> String {
    DEFAULT_SETTLEMENT_RPC_URL.to_string()
}

impl Default for SettlementConfig {
    fn default() -> Self {
        Self {
            rpc_url: default_rpc_url(),
            gas_price: None,
        }
    }
}

/// Funder wallet configuration for automatic wallet funding.
///
/// When configured, the CLI can automatically fund ecosystem and chain
/// wallets before deployment operations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunderConfig {
    /// Private key of the funder wallet.
    /// This wallet should have sufficient ETH (and CGT if applicable)
    /// to fund all ecosystem and chain wallets.
    #[serde(
        serialize_with = "serialize_secret_string",
        deserialize_with = "deserialize_secret_string"
    )]
    pub private_key: SecretString,

    /// Custom Gas Token (CGT) contract address on the settlement layer.
    /// Only required when the chain uses a custom base token (not ETH).
    /// When set, the funder will transfer both ETH and CGT to wallets.
    #[serde(default)]
    pub cgt_address: Option<Address>,
}

fn serialize_secret_string<S>(secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Serialize as the exposed secret value (for config file writing)
    serializer.serialize_str(secret.expose_secret())
}

fn deserialize_secret_string<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(SecretString::from(s))
}

/// Ecosystem configuration for default ecosystem parameters.
///
/// These values provide defaults for `adi init ecosystem` and other
/// ecosystem-related commands.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EcosystemConfig {
    /// Ecosystem name.
    /// Default: `adi_ecosystem`
    #[serde(default = "default_ecosystem_name")]
    pub name: String,

    /// Initial chain name within the ecosystem.
    /// Default: `adi`
    #[serde(default = "default_chain_name")]
    pub chain_name: String,

    /// Initial chain ID.
    /// Default: `222`
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
}

fn default_ecosystem_name() -> String {
    "adi_ecosystem".to_string()
}

fn default_chain_name() -> String {
    "adi".to_string()
}

fn default_chain_id() -> u64 {
    222
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            name: default_ecosystem_name(),
            chain_name: default_chain_name(),
            chain_id: default_chain_id(),
        }
    }
}

/// Docker toolkit image configuration.
///
/// The CLI orchestrates pre-built Docker toolkit images that contain
/// zkstack CLI, foundry-zksync, and era-contracts.
///
/// Image reference format: `{registry}/{image_name}:v{major}.{minor}.{patch}`
/// Example: `harbor.io/adi/adi-toolkit:v29.0.11`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerConfig {
    /// Docker registry URL.
    /// Default: `harbor.io/adi`
    #[serde(default = "default_registry")]
    pub registry: String,

    /// Toolkit image name.
    /// Default: `adi-toolkit`
    #[serde(default = "default_image_name")]
    pub image_name: String,
}

fn default_registry() -> String {
    DEFAULT_DOCKER_REGISTRY.to_string()
}

fn default_image_name() -> String {
    DEFAULT_DOCKER_IMAGE_NAME.to_string()
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            registry: default_registry(),
            image_name: default_image_name(),
        }
    }
}

impl DockerConfig {
    /// Build the full image reference for a given protocol version.
    ///
    /// # Arguments
    /// * `version` - Protocol version (e.g., "29.0.11")
    ///
    /// # Returns
    /// Full image reference (e.g., "harbor.io/adi/adi-toolkit:v29.0.11")
    #[allow(dead_code)] // Will be used by toolkit runner in later tasks
    pub fn image_reference(&self, version: &str) -> String {
        format!("{}/{}:v{}", self.registry, self.image_name, version)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config {
            state_dir: default_state_dir(),
            settlement: SettlementConfig::default(),
            funder: None,
            ecosystem: EcosystemConfig::default(),
            docker: DockerConfig::default(),
        };

        assert_eq!(config.settlement.rpc_url, DEFAULT_SETTLEMENT_RPC_URL);
        assert!(config.settlement.gas_price.is_none());
        assert!(config.funder.is_none());
        assert_eq!(config.ecosystem.name, "adi_ecosystem");
        assert_eq!(config.ecosystem.chain_name, "adi");
        assert_eq!(config.ecosystem.chain_id, 222);
        assert_eq!(config.docker.registry, DEFAULT_DOCKER_REGISTRY);
        assert_eq!(config.docker.image_name, DEFAULT_DOCKER_IMAGE_NAME);
    }

    #[test]
    fn test_docker_image_reference() {
        let docker = DockerConfig::default();
        let reference = docker.image_reference("29.0.11");
        assert_eq!(reference, "harbor.io/adi/adi-toolkit:v29.0.11");
    }

    #[test]
    fn test_settlement_config_default() {
        let settlement = SettlementConfig::default();
        assert_eq!(settlement.rpc_url, "http://localhost:8545");
        assert!(settlement.gas_price.is_none());
    }

    #[test]
    fn test_ecosystem_config_default() {
        let ecosystem = EcosystemConfig::default();
        assert_eq!(ecosystem.name, "adi_ecosystem");
        assert_eq!(ecosystem.chain_name, "adi");
        assert_eq!(ecosystem.chain_id, 222);
    }
}
