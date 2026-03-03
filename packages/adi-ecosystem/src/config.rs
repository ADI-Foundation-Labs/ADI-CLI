//! Ecosystem configuration types.

use adi_types::ETH_TOKEN_ADDRESS;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::types::{L1Network, ProverMode};

/// Validate that L2/L3 chain ID does not conflict with the settlement layer chain ID.
///
/// # Arguments
///
/// * `chain_id` - The L2/L3 chain ID to validate.
/// * `settlement_chain_id` - The actual settlement layer chain ID from RPC.
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with descriptive message if chain IDs match.
///
/// # Example
///
/// ```rust
/// use adi_ecosystem::validate_chain_id;
///
/// // Valid: L2 chain ID differs from settlement
/// assert!(validate_chain_id(270, 1).is_ok());
///
/// // Invalid: L2 chain ID matches settlement
/// assert!(validate_chain_id(1, 1).is_err());
/// ```
pub fn validate_chain_id(chain_id: u64, settlement_chain_id: u64) -> Result<(), String> {
    if chain_id == settlement_chain_id {
        return Err(format!(
            "Chain ID {} conflicts with settlement layer chain ID {}. \
             L2/L3 chains must have a unique chain ID different from the settlement layer.",
            chain_id, settlement_chain_id
        ));
    }
    Ok(())
}

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

    /// Deploy as L3 chain (uses calldata DA instead of blobs).
    ///
    /// When enabled, the deployment will configure the chain to use
    /// calldata-based pubdata instead of EIP-4844 blobs. Required for
    /// L3 chains deploying on L2 settlement layers.
    #[serde(default)]
    pub l3: bool,

    /// Settlement layer RPC URL.
    #[serde(default)]
    pub rpc_url: Option<Url>,
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
            l3: false,
            rpc_url: None,
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

    /// Set L3 deployment flag.
    #[must_use]
    pub fn l3(mut self, enabled: bool) -> Self {
        self.config.l3 = enabled;
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

/// Configuration for chain creation within an existing ecosystem.
///
/// This configuration is used to build zkstack chain create command arguments.
/// Unlike [`EcosystemConfig`], this does not include ecosystem-level settings
/// like L1 network, as those are inherited from the existing ecosystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain name (must be unique within the ecosystem).
    pub name: String,

    /// Chain ID (unique numeric identifier).
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

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            name: "chain".to_string(),
            chain_id: 271,
            prover_mode: ProverMode::default(),
            base_token_address: ETH_TOKEN_ADDRESS,
            base_token_price_nominator: 1,
            base_token_price_denominator: 1,
            evm_emulator: false,
        }
    }
}

/// Builder for ChainConfig.
#[derive(Default)]
pub struct ChainConfigBuilder {
    config: ChainConfig,
}

impl ChainConfigBuilder {
    /// Create a new builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set chain name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
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
    pub fn build(self) -> ChainConfig {
        self.config
    }
}

impl ChainConfig {
    /// Create a new builder.
    #[must_use]
    pub fn builder() -> ChainConfigBuilder {
        ChainConfigBuilder::new()
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
        assert!(!config.l3);
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

    #[test]
    fn test_chain_config_default() {
        let config = ChainConfig::default();
        assert_eq!(config.name, "chain");
        assert_eq!(config.chain_id, 271);
        assert_eq!(config.prover_mode, ProverMode::NoProofs);
        assert_eq!(config.base_token_address, ETH_TOKEN_ADDRESS);
        assert_eq!(config.base_token_price_nominator, 1);
        assert_eq!(config.base_token_price_denominator, 1);
        assert!(!config.evm_emulator);
    }

    #[test]
    fn test_chain_config_builder() {
        let config = ChainConfig::builder()
            .name("my_chain")
            .chain_id(456)
            .prover_mode(ProverMode::Gpu)
            .evm_emulator(true)
            .build();

        assert_eq!(config.name, "my_chain");
        assert_eq!(config.chain_id, 456);
        assert_eq!(config.prover_mode, ProverMode::Gpu);
        assert!(config.evm_emulator);
    }

    #[test]
    fn test_validate_chain_id_conflict() {
        // Same chain IDs should fail
        let result = validate_chain_id(1, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("conflicts"));

        // Sepolia chain ID conflict
        let result = validate_chain_id(11155111, 11155111);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_chain_id_valid() {
        // Different chain IDs should pass
        assert!(validate_chain_id(270, 1).is_ok());
        assert!(validate_chain_id(271, 1).is_ok());
        assert!(validate_chain_id(270, 11155111).is_ok());
    }
}
