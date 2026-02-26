use std::path::{Path, PathBuf};

use crate::error::{Result, WrapErr};
use adi_ecosystem::EcosystemConfig;
use adi_state::BackendType;
use alloy_primitives::Address;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

pub const DEFAULT_CONFIG_FILE_NAME: &str = ".adi.yml";
pub const DEFAULT_STATE_DIR: &str = ".adi_cli/state";
/// Environment variable for specifying config file path.
pub const CONFIG_ENV_VAR: &str = "ADI_CONFIG";

/// Default toolkit configuration values.
///
/// These can be overridden by CLI flags or environment variables.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ToolkitDefaults {
    /// Custom image tag override.
    /// When set, this overrides the protocol version-derived tag.
    /// Can be overridden with --image-tag or ADI__TOOLKIT__IMAGE_TAG env var.
    #[serde(default)]
    pub image_tag: Option<String>,
}

/// Default funding configuration values.
///
/// These can be overridden by CLI flags or environment variables.
/// Note: Token address is read from ecosystem metadata, not configured here.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FundingDefaults {
    /// RPC URL for settlement layer.
    /// Can be overridden with --rpc-url or ADI_RPC_URL env var.
    #[serde(default)]
    pub rpc_url: Option<Url>,

    /// Funder wallet private key.
    /// Prefer ADI_FUNDER_KEY env var for security.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub funder_key: Option<SecretString>,

    /// Deployer ETH amount (default: 1.0 ETH).
    #[serde(default)]
    pub deployer_eth: Option<f64>,

    /// Governor ETH amount (default: 1.0 ETH).
    #[serde(default)]
    pub governor_eth: Option<f64>,

    /// Governor custom gas token amount (default: 5.0 tokens).
    #[serde(default)]
    pub governor_cgt_units: Option<f64>,

    /// Operator ETH amount (default: 5.0 ETH).
    #[serde(default)]
    pub operator_eth: Option<f64>,

    /// Prove operator ETH amount (default: 5.0 ETH).
    #[serde(default)]
    pub prove_operator_eth: Option<f64>,

    /// Execute operator ETH amount (default: 5.0 ETH).
    #[serde(default)]
    pub execute_operator_eth: Option<f64>,
}

fn default_gas_multiplier() -> u64 {
    120
}

/// Default ownership transfer configuration values.
///
/// These can be overridden by CLI flags.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OwnershipDefaults {
    /// Address to transfer ownership to.
    /// Can be overridden with --new-owner flag.
    #[serde(default)]
    pub new_owner: Option<Address>,

    /// Private key for accepting ownership (new owner mode).
    /// Can be overridden with --private-key or ADI_PRIVATE_KEY env var.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub private_key: Option<SecretString>,
}

/// Default verification configuration values.
///
/// These can be overridden by CLI flags or environment variables.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct VerificationDefaults {
    /// Block explorer type (etherscan, blockscout, custom).
    /// Can be overridden with --explorer flag.
    #[serde(default)]
    pub explorer: Option<String>,

    /// Block explorer API URL.
    /// Can be overridden with --explorer-url or ADI_EXPLORER_API_URL env var.
    #[serde(default)]
    pub explorer_url: Option<Url>,

    /// Block explorer API key.
    /// Prefer ADI_EXPLORER_API_KEY env var for security.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub api_key: Option<SecretString>,
}

/// S3 synchronization configuration.
///
/// Enables syncing ecosystem state to S3-compatible storage.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct S3Config {
    /// Enable S3 synchronization.
    /// Default: `false`
    #[serde(default)]
    pub enabled: bool,

    /// Tenant identifier for S3 key prefix.
    /// Used as subfolder name in the bucket (e.g., "alice" → "alice/ecosystem.tar.gz").
    /// Required when S3 sync is enabled.
    #[serde(default)]
    pub tenant_id: Option<String>,

    /// S3 bucket name.
    /// Required when S3 sync is enabled.
    #[serde(default)]
    pub bucket: Option<String>,

    /// AWS region (e.g., "us-east-1").
    /// Default: `us-east-1`
    #[serde(default)]
    pub region: Option<String>,

    /// Custom S3 endpoint URL (for MinIO, LocalStack, etc.).
    /// When set, enables path-style addressing.
    #[serde(default)]
    pub endpoint_url: Option<Url>,

    /// AWS access key ID.
    /// Can be overridden with ADI__S3__ACCESS_KEY_ID or AWS_ACCESS_KEY_ID env var.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub access_key_id: Option<SecretString>,

    /// AWS secret access key.
    /// Can be overridden with ADI__S3__SECRET_ACCESS_KEY or AWS_SECRET_ACCESS_KEY env var.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub secret_access_key: Option<SecretString>,
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
    #[serde(default)]
    pub protocol_version: Option<String>,

    /// Default ecosystem configuration values.
    /// These can be overridden by CLI flags.
    #[serde(default)]
    pub ecosystem: EcosystemConfig,

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
    /// These can be overridden by CLI flags.
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
}

impl Config {
    /// Load configuration from file and environment variables.
    ///
    /// Configuration is loaded from multiple sources with the following priority:
    /// 1. Environment variables with `ADI__` prefix (highest)
    /// 2. CLI `--config` flag
    /// 3. `ADI_CONFIG` environment variable
    /// 4. Global config file at `~/.adi_cli/.adi.yml`
    /// 5. Built-in defaults (lowest)
    ///
    /// # Arguments
    /// * `config_path` - Optional path to config file from CLI `--config` flag
    ///
    /// # Errors
    /// Returns an error if configuration cannot be loaded or deserialized.
    /// If a config path is explicitly provided (via CLI or `ADI_CONFIG` env var)
    /// and the file doesn't exist, an error is returned.
    pub fn new(config_path: Option<&Path>) -> Result<Self> {
        // 1. Global config (lowest file priority)
        let global_config_path = path_with_home_dir(DEFAULT_CONFIG_FILE_NAME);
        let mut builder = config::Config::builder()
            .add_source(config::File::from(Path::new(&global_config_path)).required(false));

        // 2. ADI_CONFIG environment variable (higher priority)
        if let Ok(env_path) = std::env::var(CONFIG_ENV_VAR) {
            builder = builder.add_source(config::File::from(Path::new(&env_path)).required(true));
        }

        // 3. CLI --config flag (highest file priority)
        if let Some(path) = config_path {
            builder = builder.add_source(config::File::from(path).required(true));
        }

        // 4. Environment variables ADI__* (highest overall priority)
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
