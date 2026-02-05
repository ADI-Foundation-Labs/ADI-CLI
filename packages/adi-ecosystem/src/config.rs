//! Ecosystem configuration types.

use adi_types::ETH_TOKEN_ADDRESS;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

use crate::types::{L1Network, ProverMode};

/// Configuration for ecosystem creation.
///
/// This configuration is used to build zkstack ecosystem create command arguments.
/// It can be loaded from config files or constructed from CLI arguments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Ecosystem name.
    pub name: String,

    /// L1 network.
    pub l1_network: L1Network,

    /// Initial chain name.
    pub chain_name: String,

    /// Initial chain ID.
    pub chain_id: u64,

    /// Prover mode.
    pub prover_mode: ProverMode,

    /// Base token address.
    #[serde(default = "default_base_token_address")]
    pub base_token_address: Address,

    /// Base token price nominator.
    pub base_token_price_nominator: u64,

    /// Base token price denominator.
    pub base_token_price_denominator: u64,

    /// Enable EVM emulator.
    pub evm_emulator: bool,
}

fn default_base_token_address() -> Address {
    ETH_TOKEN_ADDRESS
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            name: "adi_ecosystem".to_string(),
            l1_network: L1Network::default(),
            chain_name: "adi".to_string(),
            chain_id: 270,
            prover_mode: ProverMode::default(),
            base_token_address: ETH_TOKEN_ADDRESS,
            base_token_price_nominator: 1,
            base_token_price_denominator: 1,
            evm_emulator: false,
        }
    }
}

/// Builder for EcosystemConfig.
#[derive(Default)]
pub struct EcosystemConfigBuilder {
    config: EcosystemConfig,
}

impl EcosystemConfigBuilder {
    /// Create a new builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set ecosystem name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set L1 network.
    #[must_use]
    pub fn l1_network(mut self, network: L1Network) -> Self {
        self.config.l1_network = network;
        self
    }

    /// Set chain name.
    #[must_use]
    pub fn chain_name(mut self, name: impl Into<String>) -> Self {
        self.config.chain_name = name.into();
        self
    }

    /// Set chain ID.
    #[must_use]
    pub fn chain_id(mut self, id: u64) -> Self {
        self.config.chain_id = id;
        self
    }

    /// Set prover mode.
    #[must_use]
    pub fn prover_mode(mut self, mode: ProverMode) -> Self {
        self.config.prover_mode = mode;
        self
    }

    /// Set base token address.
    #[must_use]
    pub fn base_token_address(mut self, address: Address) -> Self {
        self.config.base_token_address = address;
        self
    }

    /// Set base token price nominator.
    #[must_use]
    pub fn base_token_price_nominator(mut self, value: u64) -> Self {
        self.config.base_token_price_nominator = value;
        self
    }

    /// Set base token price denominator.
    #[must_use]
    pub fn base_token_price_denominator(mut self, value: u64) -> Self {
        self.config.base_token_price_denominator = value;
        self
    }

    /// Set EVM emulator flag.
    #[must_use]
    pub fn evm_emulator(mut self, enabled: bool) -> Self {
        self.config.evm_emulator = enabled;
        self
    }

    /// Build the config.
    #[must_use]
    pub fn build(self) -> EcosystemConfig {
        self.config
    }
}

impl EcosystemConfig {
    /// Create a new builder.
    #[must_use]
    pub fn builder() -> EcosystemConfigBuilder {
        EcosystemConfigBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EcosystemConfig::default();
        assert_eq!(config.name, "adi_ecosystem");
        assert_eq!(config.l1_network, L1Network::Localhost);
        assert_eq!(config.chain_name, "adi");
        assert_eq!(config.chain_id, 270);
        assert_eq!(config.prover_mode, ProverMode::NoProofs);
        assert_eq!(config.base_token_address, ETH_TOKEN_ADDRESS);
        assert!(!config.evm_emulator);
    }

    #[test]
    fn test_builder() {
        let config = EcosystemConfig::builder()
            .name("my_ecosystem")
            .l1_network(L1Network::Sepolia)
            .chain_name("my_chain")
            .chain_id(123)
            .prover_mode(ProverMode::Gpu)
            .evm_emulator(true)
            .build();

        assert_eq!(config.name, "my_ecosystem");
        assert_eq!(config.l1_network, L1Network::Sepolia);
        assert_eq!(config.chain_name, "my_chain");
        assert_eq!(config.chain_id, 123);
        assert_eq!(config.prover_mode, ProverMode::Gpu);
        assert!(config.evm_emulator);
    }
}
