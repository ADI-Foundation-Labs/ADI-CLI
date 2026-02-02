//! Deployment configuration types.

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

/// Initial deployment configuration from configs/initial_deployments.yaml.
///
/// Contains deployment parameters for ecosystem initialization.
///
/// # Example YAML
/// ```yaml
/// create2_factory_salt: "0x2436103ccff8503733211abe2e768123796afb2450e3b16be12fc617f59d2088"
/// governance_min_delay: 0
/// max_number_of_chains: 100
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InitialDeployments {
    /// Create2 factory salt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create2_factory_salt: Option<B256>,

    /// Governance minimum delay (seconds).
    #[serde(default)]
    pub governance_min_delay: u64,

    /// Maximum number of chains.
    #[serde(default)]
    pub max_number_of_chains: u64,

    /// Diamond init batch overhead L1 gas.
    #[serde(default)]
    pub diamond_init_batch_overhead_l1_gas: u64,

    /// Diamond init max L2 gas per batch.
    #[serde(default)]
    pub diamond_init_max_l2_gas_per_batch: u64,

    /// Diamond init max pubdata per batch.
    #[serde(default)]
    pub diamond_init_max_pubdata_per_batch: u64,

    /// Diamond init minimal L2 gas price.
    #[serde(default)]
    pub diamond_init_minimal_l2_gas_price: u64,

    /// Diamond init priority tx max pubdata.
    #[serde(default)]
    pub diamond_init_priority_tx_max_pubdata: u64,

    /// Diamond init pubdata pricing mode.
    #[serde(default)]
    pub diamond_init_pubdata_pricing_mode: u64,

    /// Priority tx max gas limit.
    #[serde(default)]
    pub priority_tx_max_gas_limit: u64,

    /// Validator timelock execution delay.
    #[serde(default)]
    pub validator_timelock_execution_delay: u64,

    /// Token WETH address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_weth_address: Option<Address>,

    /// Bridgehub create new chain salt.
    #[serde(default)]
    pub bridgehub_create_new_chain_salt: u64,
}

/// ERC20 token configuration for deployment.
///
/// # Example YAML
/// ```yaml
/// - name: "DAI"
///   symbol: "DAI"
///   decimals: 18
///   implementation: "TestnetERC20Token.sol"
///   mint: "0x9000000000000000000000"
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Erc20Token {
    /// Token name.
    pub name: String,

    /// Token symbol.
    pub symbol: String,

    /// Token decimals.
    pub decimals: u8,

    /// Implementation contract file.
    pub implementation: String,

    /// Initial mint amount (as hex string for large numbers).
    pub mint: String,
}

impl Erc20Token {
    /// Creates a new ERC20 token configuration.
    pub fn new(
        name: impl Into<String>,
        symbol: impl Into<String>,
        decimals: u8,
        implementation: impl Into<String>,
        mint: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            symbol: symbol.into(),
            decimals,
            implementation: implementation.into(),
            mint: mint.into(),
        }
    }
}

/// ERC20 deployments configuration from configs/erc20_deployments.yaml.
///
/// # Example YAML
/// ```yaml
/// tokens:
///   - name: "DAI"
///     symbol: "DAI"
///     decimals: 18
///     implementation: "TestnetERC20Token.sol"
///     mint: "0x9000000000000000000000"
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Erc20Deployments {
    /// List of tokens to deploy.
    #[serde(default)]
    pub tokens: Vec<Erc20Token>,
}

impl Erc20Deployments {
    /// Creates an empty ERC20 deployments configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a token to the deployments.
    pub fn with_token(mut self, token: Erc20Token) -> Self {
        self.tokens.push(token);
        self
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_deployments_deserialize() {
        let yaml = r#"
create2_factory_salt: "0x2436103ccff8503733211abe2e768123796afb2450e3b16be12fc617f59d2088"
governance_min_delay: 0
max_number_of_chains: 100
priority_tx_max_gas_limit: 72000000
"#;
        let deployments: InitialDeployments = serde_yaml::from_str(yaml).unwrap();
        assert!(deployments.create2_factory_salt.is_some());
        assert_eq!(deployments.max_number_of_chains, 100);
        assert_eq!(deployments.priority_tx_max_gas_limit, 72000000);
    }

    #[test]
    fn test_erc20_deployments_deserialize() {
        let yaml = r#"
tokens:
  - name: "DAI"
    symbol: "DAI"
    decimals: 18
    implementation: "TestnetERC20Token.sol"
    mint: "0x9000000000000000000000"
  - name: "WBTC"
    symbol: "WBTC"
    decimals: 8
    implementation: "TestnetERC20Token.sol"
    mint: "0x100000000000000"
"#;
        let deployments: Erc20Deployments = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(deployments.tokens.len(), 2);
        assert_eq!(deployments.tokens.first().map(|t| &t.symbol), Some(&"DAI".to_string()));
    }
}
