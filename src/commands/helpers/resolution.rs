//! Functions for resolving CLI arguments from user input or config defaults.

use alloy_primitives::Address;
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
}
