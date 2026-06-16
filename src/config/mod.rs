mod types;

pub use types::{
    FeeAdjusterConfig, FundingDefaults, OperatorsConfig, OwnershipDefaults, S3Config,
    ToolkitDefaults, VaultDefaults, VerificationDefaults,
};

use std::path::{Path, PathBuf};

use crate::error::{Result, WrapErr};
use adi_ecosystem::EcosystemDefaults;
use adi_state::BackendType;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE_NAME: &str = ".adi.yml";
/// ADI home directory (under the user's home) shared by config and state.
pub const DEFAULT_HOME_DIR: &str = ".adi_cli";
pub const DEFAULT_STATE_DIR: &str = ".adi_cli/state";
/// Environment variable for specifying config file path.
pub const CONFIG_ENV_VAR: &str = "ADI_CONFIG";

/// Recognized top-level config keys (mirror of `Config`'s serialized fields).
/// Used to warn about misplaced keys silently dropped during deserialization.
const KNOWN_TOP_LEVEL_KEYS: &[&str] = &[
    "state_dir",
    "debug",
    "protocol_version",
    "ecosystem",
    "state_backend",
    "funding",
    "toolkit",
    "ownership",
    "verification",
    "gas_multiplier",
    "vault",
    "s3",
    "operators",
    "fee_adjuster",
];

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

    /// Default Vault configuration values.
    /// Used by the server-params --upload flow.
    #[serde(default)]
    pub vault: VaultDefaults,

    /// S3 synchronization configuration.
    /// Enables syncing ecosystem state to S3-compatible storage.
    #[serde(default)]
    pub s3: S3Config,

    /// Predefined operator addresses.
    /// **Deprecated**: Use `ecosystem.chains[].operators` instead.
    #[serde(default)]
    pub operators: OperatorsConfig,

    /// Fee adjuster post-deployment configuration.
    /// `enabled` defaults to `true` — set `fee_adjuster.enabled: false` in
    /// `.adi.yml` to skip `FeeAdjusterConfig` deployment.
    #[serde(default)]
    pub fee_adjuster: FeeAdjusterConfig,
}

/// Outcome of loading configuration: the parsed config, the file path it was
/// loaded from (for saving), and any non-fatal warnings to surface to the user.
pub struct LoadedConfig {
    /// Parsed and normalized configuration.
    pub config: Config,
    /// Effective config file path (used by commands that save back to it).
    pub path: PathBuf,
    /// Non-fatal warnings (misplaced keys, legacy location) to log.
    pub warnings: Vec<String>,
}

/// Load configuration from file and environment variables.
///
/// Configuration sources (mutually exclusive for files):
/// 1. CLI `--config` flag (if provided, used exclusively)
/// 2. `ADI_CONFIG` environment variable (if set, used exclusively)
/// 3. Unified config file at `~/.adi_cli/.adi.yml` (preferred default)
/// 4. Legacy config file at `~/.adi.yml` (fallback, deprecated)
///
/// Environment variables with `ADI__` prefix always override any file config.
///
/// # Errors
/// Returns an error if configuration cannot be loaded or deserialized.
/// If a config path is explicitly provided (via CLI or `ADI_CONFIG` env var)
/// and the file doesn't exist, an error is returned.
pub fn load(cli_path: Option<&Path>) -> Result<LoadedConfig> {
    let mut warnings = Vec::new();
    let path = resolve_config_path(cli_path, &mut warnings);

    // A missing file is only an error when the path was explicitly requested.
    let required = cli_path.is_some() || std::env::var(CONFIG_ENV_VAR).is_ok();

    // Surface keys placed at the wrong nesting level (serde drops them silently).
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            warnings.extend(detect_unknown_keys(&content));
        }
    }

    // Environment variables ADI__* always override (highest priority).
    let mut config: Config = config::Config::builder()
        .add_source(config::File::from(path.as_path()).required(required))
        .add_source(
            config::Environment::with_prefix("ADI")
                .separator("__")
                .try_parsing(true),
        )
        .build()
        .wrap_err("Failed to build config")?
        .try_deserialize()
        .wrap_err("Failed to deserialize config")?;

    // Expand ~ in state_dir to user's home directory.
    config.state_dir = expand_tilde(&config.state_dir);

    Ok(LoadedConfig {
        config,
        path,
        warnings,
    })
}

/// Resolve the effective config file path.
///
/// Priority: CLI `--config` > `ADI_CONFIG` > `~/.adi_cli/.adi.yml` > legacy `~/.adi.yml`.
/// When falling back to the legacy location, a deprecation warning is pushed.
pub fn resolve_config_path(cli_path: Option<&Path>, warnings: &mut Vec<String>) -> PathBuf {
    if let Some(path) = cli_path {
        return path.to_path_buf();
    }

    if let Ok(env_path) = std::env::var(CONFIG_ENV_VAR) {
        return PathBuf::from(env_path);
    }

    let unified = PathBuf::from(path_with_home_dir(&format!(
        "{DEFAULT_HOME_DIR}/{DEFAULT_CONFIG_FILE_NAME}"
    )));
    if unified.exists() {
        return unified;
    }

    let legacy = PathBuf::from(path_with_home_dir(DEFAULT_CONFIG_FILE_NAME));
    if legacy.exists() {
        warnings.push(format!(
            "Using legacy config at {}. Move it to {} to keep config and state in one directory.",
            legacy.display(),
            unified.display()
        ));
        return legacy;
    }

    // Neither exists yet: default to the unified location.
    unified
}

/// Detect top-level YAML keys that `Config` does not recognize.
///
/// Misplaced keys (e.g. a top-level `rpc_url` instead of `ecosystem.rpc_url`)
/// are silently ignored by serde; this surfaces them as actionable warnings.
fn detect_unknown_keys(content: &str) -> Vec<String> {
    serde_yaml::from_str::<std::collections::BTreeMap<String, serde_yaml::Value>>(content)
        .map(|map| {
            map.keys()
                .filter(|key| !KNOWN_TOP_LEVEL_KEYS.contains(&key.as_str()))
                .map(|key| unknown_key_warning(key))
                .collect()
        })
        .unwrap_or_default()
}

/// Build a warning for an unrecognized top-level key, hinting at the likely
/// intended nested location for common mistakes.
fn unknown_key_warning(key: &str) -> String {
    let hint = match key {
        "rpc_url" => " — did you mean 'ecosystem.rpc_url'?",
        "new_owner" => " — did you mean 'ecosystem.ownership.new_owner'?",
        "private_key" => " — did you mean 'ownership.private_key'?",
        "funder_key" => " — did you mean 'funding.funder_key'?",
        "explorer" | "explorer_url" | "api_key" => {
            " — did you mean to nest it under 'verification'?"
        }
        _ => "",
    };
    format!("Unknown config key '{key}' at top level{hint}")
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn detect_unknown_keys_flags_misplaced_rpc_url() {
        let yaml = "rpc_url: https://example.com\necosystem:\n  name: eco\n";
        let warnings = detect_unknown_keys(yaml);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("rpc_url"));
        assert!(warnings[0].contains("ecosystem.rpc_url"));
    }

    #[test]
    fn detect_unknown_keys_accepts_known_keys() {
        let yaml = "state_dir: ./.state\necosystem:\n  name: eco\nfunding:\n  deployer_eth: 1\n";
        assert!(detect_unknown_keys(yaml).is_empty());
    }

    #[test]
    fn detect_unknown_keys_handles_invalid_yaml() {
        assert!(detect_unknown_keys(": not valid").is_empty());
    }

    #[test]
    fn unknown_key_warning_includes_hint_for_known_mistakes() {
        assert!(unknown_key_warning("funder_key").contains("funding.funder_key"));
        assert!(unknown_key_warning("private_key").contains("ownership.private_key"));
    }

    #[test]
    fn unknown_key_warning_without_hint_for_arbitrary_key() {
        let warning = unknown_key_warning("totally_made_up");
        assert!(warning.contains("totally_made_up"));
        assert!(!warning.contains("did you mean"));
    }

    #[test]
    fn resolve_config_path_prefers_cli_arg() {
        let mut warnings = Vec::new();
        let cli = PathBuf::from("/tmp/custom.yml");
        let resolved = resolve_config_path(Some(cli.as_path()), &mut warnings);
        assert_eq!(resolved, cli);
        assert!(warnings.is_empty());
    }
}
