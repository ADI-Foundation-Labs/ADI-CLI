use alloy_primitives::Address;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

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

    /// Deployer ETH amount.
    /// Default: `100`
    #[serde(default = "default_deployer_eth")]
    pub deployer_eth: Option<f64>,

    /// Governor ETH amount.
    /// Default: `40`
    #[serde(default = "default_governor_eth")]
    pub governor_eth: Option<f64>,

    /// Governor custom gas token amount.
    /// Default: `5`
    #[serde(default = "default_governor_cgt_units")]
    pub governor_cgt_units: Option<f64>,
}

fn default_deployer_eth() -> Option<f64> {
    Some(100.0)
}

fn default_governor_eth() -> Option<f64> {
    Some(40.0)
}

fn default_governor_cgt_units() -> Option<f64> {
    Some(5.0)
}

fn default_s3_bucket() -> Option<String> {
    Some("adi-state".to_string())
}

fn default_s3_region() -> Option<String> {
    Some("us-east-1".to_string())
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

/// Predefined operator addresses.
///
/// These addresses override randomly generated operator addresses after init.
/// All fields are optional - only specified addresses are overridden.
/// Operators manage their own private keys externally.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OperatorsConfig {
    /// Operator address (receives commit/precommit/revert roles).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<Address>,

    /// Prove operator address (receives prover role).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prove_operator: Option<Address>,

    /// Execute operator address (receives executor role).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execute_operator: Option<Address>,
}

/// Default Vault configuration values.
///
/// These can be overridden by environment variables.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct VaultDefaults {
    /// Vault base URL (without trailing slash).
    /// Default: `https://vault.dev.internal.adifoundation.ai`
    /// Can be overridden with `ADI__VAULT__API_URL` env var.
    #[serde(default)]
    pub api_url: Option<Url>,
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
    /// Default: `adi-state`
    #[serde(default = "default_s3_bucket")]
    pub bucket: Option<String>,

    /// AWS region (e.g., "us-east-1").
    /// Default: `us-east-1`
    #[serde(default = "default_s3_region")]
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
