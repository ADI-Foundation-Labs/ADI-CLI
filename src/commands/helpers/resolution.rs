//! Functions for resolving CLI arguments from user input or config defaults.
//!
//! Every config value that maps to a CLI argument MUST be resolved through a
//! function in this module (and have a row in the precedence matrix test below),
//! so a flag is never *required* when the value is already present in config.
//! Commands should call these helpers instead of reaching into `context.config()`
//! with bespoke fallback ladders.

use adi_ecosystem::verification::{ExplorerConfig, ExplorerType};
use alloy_primitives::Address;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::config::Config;
use crate::error::Result;

/// Resolve ecosystem name from optional arg or config.
pub fn resolve_ecosystem_name(arg_value: Option<&String>, config: &Config) -> Result<String> {
    arg_value
        .cloned()
        .or_else(|| Some(config.ecosystem.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem name required: use --ecosystem-name or set in config")
        })
}

/// Resolve chain name from optional arg or config.
///
/// Falls back to the first chain in `ecosystem.chains[]` if available.
pub fn resolve_chain_name(arg_value: Option<&String>, config: &Config) -> Result<String> {
    arg_value
        .cloned()
        .or_else(|| config.ecosystem.default_chain().map(|c| c.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| eyre::eyre!("Chain name required: use --chain or set in config"))
}

/// Resolve RPC URL from optional arg or config.
///
/// Priority: CLI arg > ecosystem.rpc_url > funding.rpc_url (backward compat)
pub fn resolve_rpc_url(arg_value: Option<&Url>, config: &Config) -> Result<Url> {
    arg_value
        .cloned()
        .or_else(|| config.ecosystem.rpc_url.clone())
        .or_else(|| config.funding.rpc_url.clone()) // backward compatibility
        .ok_or_else(|| {
            eyre::eyre!("RPC URL required: use --rpc-url or set ecosystem.rpc_url in config")
        })
}

/// Resolve ecosystem new owner from config.
///
/// Priority: CLI arg > ecosystem.ownership.new_owner > ownership.new_owner (deprecated)
pub fn resolve_ecosystem_new_owner(arg_value: Option<Address>, config: &Config) -> Result<Address> {
    arg_value
        .or(config.ecosystem.ownership.new_owner)
        .or(config.ownership.new_owner) // backward compatibility
        .ok_or_else(|| {
            eyre::eyre!(
                "Ecosystem new owner required: use --new-owner or set ecosystem.ownership.new_owner in config"
            )
        })
}

/// Resolve chain new owner from config.
///
/// Priority: CLI arg > chains[name].ownership.new_owner > ownership.new_owner (deprecated)
pub fn resolve_chain_new_owner(
    arg_value: Option<Address>,
    chain_name: &str,
    config: &Config,
) -> Result<Address> {
    arg_value
        .or_else(|| {
            config
                .ecosystem
                .get_chain(chain_name)
                .and_then(|c| c.ownership.new_owner)
        })
        .or(config.ownership.new_owner) // backward compatibility
        .ok_or_else(|| {
            eyre::eyre!(
                "Chain new owner required: use --new-owner or set ecosystem.chains[{}].ownership.new_owner in config",
                chain_name
            )
        })
}

/// Resolve protocol version from optional arg or config.
pub fn resolve_protocol_version(arg_value: Option<&String>, config: &Config) -> Result<String> {
    arg_value
        .cloned()
        .or_else(|| config.protocol_version.clone())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!(
                "Protocol version required: use --protocol-version or set protocol_version in config"
            )
        })
}

/// Resolve gas price multiplier from optional arg or config.
///
/// `config.gas_multiplier` always has a value (serde default 200).
pub fn resolve_gas_multiplier(arg_value: Option<u64>, config: &Config) -> u64 {
    arg_value.unwrap_or(config.gas_multiplier)
}

/// Resolve funder private key from optional arg or config.
///
/// Priority: CLI arg / `ADI_FUNDER_KEY` env (folded into the arg by clap) > `funding.funder_key`.
pub fn resolve_funder_key(arg_value: Option<&str>, config: &Config) -> Result<SecretString> {
    arg_value
        .map(|key| SecretString::from(key.to_owned()))
        .or_else(|| config.funding.funder_key.clone())
        .ok_or_else(|| {
            eyre::eyre!(
                "Funder key required: use --funder-key, set ADI_FUNDER_KEY, or funding.funder_key in config"
            )
        })
}

/// Resolve the new-owner private key from optional arg or config.
///
/// Priority: CLI arg / `ADI_PRIVATE_KEY` env, then `ecosystem.ownership.private_key`,
/// then `ownership.private_key` (deprecated). Returns `None` so callers can fall
/// back to governor/interactive modes; never prompts.
pub fn resolve_private_key(arg_value: Option<&str>, config: &Config) -> Option<SecretString> {
    arg_value
        .map(|key| SecretString::from(key.to_owned()))
        .or_else(|| config.ecosystem.ownership.private_key.clone())
        .or_else(|| config.ownership.private_key.clone()) // backward compatibility
}

/// Resolve a chain's new-owner private key from config.
///
/// Priority: `ecosystem.chains[].ownership.private_key`. Returns `None` when the
/// chain has no configured key so callers can fall back appropriately.
pub fn resolve_chain_private_key(config: &Config, chain_name: &str) -> Option<SecretString> {
    config
        .ecosystem
        .get_chain(chain_name)
        .and_then(|c| c.ownership.private_key.clone())
}

/// Resolve block explorer type from optional arg or config.
///
/// Priority: CLI arg > `verification.explorer` > `ExplorerType` default (etherscan).
pub fn resolve_explorer_type(arg_value: Option<ExplorerType>, config: &Config) -> ExplorerType {
    arg_value
        .or_else(|| {
            config
                .verification
                .explorer
                .as_ref()
                .and_then(|explorer| explorer.parse().ok())
        })
        .unwrap_or_default()
}

/// Resolve block explorer API key from optional arg or config.
///
/// Priority: CLI arg / `ADI_EXPLORER_API_KEY` env > `verification.api_key`.
pub fn resolve_explorer_api_key(arg_value: Option<&str>, config: &Config) -> Option<String> {
    arg_value.map(str::to_owned).or_else(|| {
        config
            .verification
            .api_key
            .as_ref()
            .map(|key| key.expose_secret().to_owned())
    })
}

/// Resolve block explorer API URL from optional arg, config, or built-in default.
///
/// Priority: CLI arg (validated) > `verification.explorer_url` >
/// `ExplorerConfig::default_api_url`. Errors when none resolve.
pub fn resolve_explorer_url(
    arg_value: Option<&Url>,
    explorer_type: ExplorerType,
    chain_id: u64,
    config: &Config,
) -> Result<Url> {
    if let Some(url) = arg_value {
        validate_blockscout_url(url, explorer_type)?;
        return Ok(url.clone());
    }

    if let Some(url) = config.verification.explorer_url.clone() {
        return Ok(url);
    }

    if let Some(url) = ExplorerConfig::default_api_url(explorer_type, chain_id) {
        return Ok(url);
    }

    if explorer_type == ExplorerType::Custom {
        return Err(eyre::eyre!(
            "Explorer URL required for custom explorer. Provide --explorer-url"
        ));
    }

    Err(eyre::eyre!(
        "No default explorer URL for chain ID {chain_id}. Provide --explorer-url"
    ))
}

/// Validate Blockscout URLs to catch common endpoint mistakes.
fn validate_blockscout_url(url: &Url, explorer_type: ExplorerType) -> Result<()> {
    if explorer_type != ExplorerType::Blockscout {
        return Ok(());
    }

    let url_str = url.as_str();
    if url_str.contains("/api/eth-rpc") {
        return Err(eyre::eyre!(
            "Invalid Blockscout URL: '/api/eth-rpc' is the JSON-RPC endpoint.\n\
             For contract verification, use the REST API endpoint instead.\n\
             Example: https://eth-sepolia.blockscout.com/api"
        ));
    }
    if url_str.contains("/api/v2") {
        return Err(eyre::eyre!(
            "Invalid Blockscout URL: '/api/v2' is the native REST API.\n\
             For contract verification, use the Etherscan-compatible endpoint.\n\
             Example: https://eth-sepolia.blockscout.com/api"
        ));
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn resolve_ecosystem_name_from_arg() {
        let arg = "my_eco".to_string();
        let config = default_config();
        let result = resolve_ecosystem_name(Some(&arg), &config).unwrap();
        assert_eq!(result, "my_eco");
    }

    #[test]
    fn resolve_ecosystem_name_from_config() {
        let mut config = default_config();
        config.ecosystem.name = "config_eco".to_string();
        let result = resolve_ecosystem_name(None, &config).unwrap();
        assert_eq!(result, "config_eco");
    }

    #[test]
    fn resolve_ecosystem_name_empty_returns_error() {
        let mut config = default_config();
        config.ecosystem.name = String::new();
        assert!(resolve_ecosystem_name(None, &config).is_err());
    }

    #[test]
    fn resolve_protocol_version_from_arg() {
        let arg = "v30.0.2".to_string();
        let config = default_config();
        let result = resolve_protocol_version(Some(&arg), &config).unwrap();
        assert_eq!(result, "v30.0.2");
    }

    #[test]
    fn resolve_protocol_version_from_config() {
        let config = default_config();
        let result = resolve_protocol_version(None, &config).unwrap();
        assert_eq!(result, "v0.30.1");
    }

    #[test]
    fn resolve_protocol_version_missing_returns_error() {
        let mut config = default_config();
        config.protocol_version = None;
        assert!(resolve_protocol_version(None, &config).is_err());
    }

    #[test]
    fn resolve_rpc_url_from_arg() {
        let url: Url = "http://localhost:8545".parse().unwrap();
        let config = default_config();
        let result = resolve_rpc_url(Some(&url), &config).unwrap();
        assert_eq!(result.as_str(), "http://localhost:8545/");
    }

    #[test]
    fn resolve_rpc_url_missing_returns_error() {
        let config = default_config();
        assert!(resolve_rpc_url(None, &config).is_err());
    }

    #[test]
    fn resolve_rpc_url_from_ecosystem_config() {
        let mut config = default_config();
        config.ecosystem.rpc_url = Some("https://eco.example.com".parse().unwrap());
        let result = resolve_rpc_url(None, &config).unwrap();
        assert_eq!(result.as_str(), "https://eco.example.com/");
    }

    #[test]
    fn resolve_rpc_url_from_funding_config_backcompat() {
        let mut config = default_config();
        config.funding.rpc_url = Some("https://funding.example.com".parse().unwrap());
        let result = resolve_rpc_url(None, &config).unwrap();
        assert_eq!(result.as_str(), "https://funding.example.com/");
    }

    #[test]
    fn resolve_gas_multiplier_prefers_arg() {
        let config = default_config();
        assert_eq!(resolve_gas_multiplier(Some(150), &config), 150);
    }

    #[test]
    fn resolve_gas_multiplier_falls_back_to_config() {
        let mut config = default_config();
        config.gas_multiplier = 175;
        assert_eq!(resolve_gas_multiplier(None, &config), 175);
    }

    #[test]
    fn resolve_funder_key_prefers_arg() {
        let config = default_config();
        let key = resolve_funder_key(Some("0xabc"), &config).unwrap();
        assert_eq!(key.expose_secret(), "0xabc");
    }

    #[test]
    fn resolve_funder_key_falls_back_to_config() {
        let mut config = default_config();
        config.funding.funder_key = Some(SecretString::from("0xdef".to_string()));
        let key = resolve_funder_key(None, &config).unwrap();
        assert_eq!(key.expose_secret(), "0xdef");
    }

    #[test]
    fn resolve_funder_key_missing_returns_error() {
        let config = default_config();
        assert!(resolve_funder_key(None, &config).is_err());
    }

    #[test]
    fn resolve_private_key_prefers_arg() {
        let config = default_config();
        let key = resolve_private_key(Some("0x111"), &config).unwrap();
        assert_eq!(key.expose_secret(), "0x111");
    }

    #[test]
    fn resolve_private_key_falls_back_to_config() {
        let mut config = default_config();
        config.ownership.private_key = Some(SecretString::from("0x222".to_string()));
        let key = resolve_private_key(None, &config).unwrap();
        assert_eq!(key.expose_secret(), "0x222");
    }

    #[test]
    fn resolve_private_key_missing_returns_none() {
        let config = default_config();
        assert!(resolve_private_key(None, &config).is_none());
    }

    #[test]
    fn resolve_explorer_type_arg_beats_config() {
        let mut config = default_config();
        config.verification.explorer = Some("etherscan".to_string());
        assert_eq!(
            resolve_explorer_type(Some(ExplorerType::Blockscout), &config),
            ExplorerType::Blockscout
        );
    }

    #[test]
    fn resolve_explorer_type_from_config() {
        let mut config = default_config();
        config.verification.explorer = Some("blockscout".to_string());
        assert_eq!(
            resolve_explorer_type(None, &config),
            ExplorerType::Blockscout
        );
    }

    #[test]
    fn resolve_explorer_type_defaults_when_unset() {
        let config = default_config();
        assert_eq!(
            resolve_explorer_type(None, &config),
            ExplorerType::default()
        );
    }

    #[test]
    fn resolve_explorer_api_key_prefers_arg() {
        let mut config = default_config();
        config.verification.api_key = Some(SecretString::from("config_key".to_string()));
        assert_eq!(
            resolve_explorer_api_key(Some("arg_key"), &config).as_deref(),
            Some("arg_key")
        );
    }

    #[test]
    fn resolve_explorer_api_key_from_config() {
        let mut config = default_config();
        config.verification.api_key = Some(SecretString::from("config_key".to_string()));
        assert_eq!(
            resolve_explorer_api_key(None, &config).as_deref(),
            Some("config_key")
        );
    }

    #[test]
    fn resolve_explorer_url_prefers_arg() {
        let config = default_config();
        let url: Url = "https://arg.example.com/api".parse().unwrap();
        let result = resolve_explorer_url(Some(&url), ExplorerType::Custom, 1, &config).unwrap();
        assert_eq!(result.as_str(), "https://arg.example.com/api");
    }

    #[test]
    fn resolve_explorer_url_from_config() {
        let mut config = default_config();
        config.verification.explorer_url = Some("https://cfg.example.com/api".parse().unwrap());
        let result = resolve_explorer_url(None, ExplorerType::Custom, 1, &config).unwrap();
        assert_eq!(result.as_str(), "https://cfg.example.com/api");
    }

    #[test]
    fn resolve_explorer_url_custom_without_value_errors() {
        let config = default_config();
        assert!(resolve_explorer_url(None, ExplorerType::Custom, 1, &config).is_err());
    }

    #[test]
    fn resolve_explorer_url_rejects_blockscout_v2_endpoint() {
        let config = default_config();
        let url: Url = "https://x.blockscout.com/api/v2".parse().unwrap();
        assert!(resolve_explorer_url(Some(&url), ExplorerType::Blockscout, 1, &config).is_err());
    }

    /// Precedence matrix: every config-backed value must resolve from config
    /// alone (arg absent). This locks in "a flag is never required when the
    /// value is in config" — the regression that motivated this module.
    #[test]
    fn config_alone_is_sufficient_for_every_value() {
        struct Case {
            name: &'static str,
            set: fn(&mut Config),
            resolves: fn(&Config) -> bool,
        }

        let cases = [
            Case {
                name: "rpc_url via ecosystem",
                set: |c| c.ecosystem.rpc_url = Some("https://e.example.com".parse().unwrap()),
                resolves: |c| resolve_rpc_url(None, c).is_ok(),
            },
            Case {
                name: "rpc_url via funding",
                set: |c| c.funding.rpc_url = Some("https://f.example.com".parse().unwrap()),
                resolves: |c| resolve_rpc_url(None, c).is_ok(),
            },
            Case {
                name: "ecosystem_name",
                set: |c| c.ecosystem.name = "eco".to_string(),
                resolves: |c| resolve_ecosystem_name(None, c).is_ok(),
            },
            Case {
                name: "chain_name",
                set: |c| {
                    c.ecosystem.chains.push(adi_ecosystem::ChainDefaults {
                        name: "chain_x".to_string(),
                        chain_id: 1,
                        ..Default::default()
                    });
                },
                resolves: |c| resolve_chain_name(None, c).is_ok(),
            },
            Case {
                name: "protocol_version",
                set: |c| c.protocol_version = Some("v1.0.0".to_string()),
                resolves: |c| resolve_protocol_version(None, c).is_ok(),
            },
            Case {
                name: "gas_multiplier",
                set: |c| c.gas_multiplier = 150,
                resolves: |c| resolve_gas_multiplier(None, c) == 150,
            },
            Case {
                name: "funder_key",
                set: |c| c.funding.funder_key = Some(SecretString::from("0xabc".to_string())),
                resolves: |c| resolve_funder_key(None, c).is_ok(),
            },
            Case {
                name: "private_key",
                set: |c| c.ownership.private_key = Some(SecretString::from("0xdef".to_string())),
                resolves: |c| resolve_private_key(None, c).is_some(),
            },
            Case {
                name: "new_owner",
                set: |c| c.ecosystem.ownership.new_owner = Some(Address::ZERO),
                resolves: |c| resolve_ecosystem_new_owner(None, c).is_ok(),
            },
            Case {
                name: "explorer_type",
                set: |c| c.verification.explorer = Some("blockscout".to_string()),
                resolves: |c| resolve_explorer_type(None, c) == ExplorerType::Blockscout,
            },
            Case {
                name: "explorer_url",
                set: |c| {
                    c.verification.explorer_url =
                        Some("https://x.example.com/api".parse().unwrap());
                },
                resolves: |c| resolve_explorer_url(None, ExplorerType::Custom, 1, c).is_ok(),
            },
            Case {
                name: "explorer api_key",
                set: |c| c.verification.api_key = Some(SecretString::from("k".to_string())),
                resolves: |c| resolve_explorer_api_key(None, c).is_some(),
            },
        ];

        for case in cases {
            let mut config = default_config();
            (case.set)(&mut config);
            assert!(
                (case.resolves)(&config),
                "config value '{}' did not resolve from config alone",
                case.name
            );
        }
    }
}
