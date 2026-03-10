//! Default configuration types for CLI.
//!
//! These types define the structure of the CLI configuration file (`~/.adi.yml`).
//! They are separate from the domain types (`EcosystemConfig`, `ChainConfig`) which
//! are used for building zkstack commands.

use alloy_primitives::Address;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::types::{L1Network, ProverMode};

/// Predefined operator addresses for a chain.
///
/// These addresses override randomly generated operator addresses after init.
/// All fields are optional - only specified addresses are overridden.
/// Operators manage their own private keys externally.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct OperatorsDefaults {
    /// Operator address (receives commit/precommit/revert roles).
    #[serde(default)]
    pub operator: Option<Address>,

    /// Prove operator address (receives prover role).
    #[serde(default)]
    pub prove_operator: Option<Address>,

    /// Execute operator address (receives executor role).
    #[serde(default)]
    pub execute_operator: Option<Address>,
}

/// Per-chain funding configuration.
///
/// Defines ETH amounts to fund chain-level wallets (operators).
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct ChainFundingDefaults {
    /// Operator ETH amount.
    #[serde(default)]
    pub operator_eth: Option<f64>,

    /// Prove operator ETH amount.
    #[serde(default)]
    pub prove_operator_eth: Option<f64>,

    /// Execute operator ETH amount.
    #[serde(default)]
    pub execute_operator_eth: Option<f64>,
}

/// Per-chain ownership configuration.
///
/// Used for transferring/accepting chain ownership (separate from ecosystem ownership).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ChainOwnershipDefaults {
    /// Address to transfer chain ownership to (newChainOwner).
    #[serde(default)]
    pub new_owner: Option<Address>,

    /// Private key for accepting chain ownership.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub private_key: Option<SecretString>,
}

impl PartialEq for ChainOwnershipDefaults {
    fn eq(&self, other: &Self) -> bool {
        // Only compare new_owner (private_key is never serialized anyway)
        self.new_owner == other.new_owner
    }
}

/// Per-chain configuration defaults.
///
/// Contains all chain-level settings including operators, funding, and ownership.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ChainDefaults {
    /// Chain name (must be unique within the ecosystem).
    pub name: String,

    /// Chain ID (unique numeric identifier).
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,

    /// Prover mode.
    #[serde(default)]
    pub prover_mode: ProverMode,

    /// Base token address.
    #[serde(default)]
    pub base_token_address: Option<Address>,

    /// Base token price nominator.
    #[serde(default = "default_price_ratio")]
    pub base_token_price_nominator: u64,

    /// Base token price denominator.
    #[serde(default = "default_price_ratio")]
    pub base_token_price_denominator: u64,

    /// Enable EVM emulator.
    #[serde(default)]
    pub evm_emulator: bool,

    /// Use blob-based pubdata (EIP-4844).
    ///
    /// When `true`, uses blobs for pubdata (L2 chains settling on L1).
    /// When `false`, uses calldata for pubdata (L3 chains settling on L2).
    /// Default: `false` (calldata mode for L3 deployments)
    #[serde(default)]
    pub blobs: bool,

    /// Predefined operator addresses.
    #[serde(default)]
    pub operators: OperatorsDefaults,

    /// Chain-level funding configuration.
    #[serde(default)]
    pub funding: ChainFundingDefaults,

    /// Chain-level ownership configuration.
    #[serde(default)]
    pub ownership: ChainOwnershipDefaults,
}

fn default_chain_id() -> u64 {
    222
}

fn default_price_ratio() -> u64 {
    1
}

impl Default for ChainDefaults {
    fn default() -> Self {
        Self {
            name: "adi".to_string(),
            chain_id: default_chain_id(),
            prover_mode: ProverMode::default(),
            base_token_address: None,
            base_token_price_nominator: 1,
            base_token_price_denominator: 1,
            evm_emulator: false,
            blobs: false,
            operators: OperatorsDefaults::default(),
            funding: ChainFundingDefaults::default(),
            ownership: ChainOwnershipDefaults::default(),
        }
    }
}

impl ChainDefaults {
    /// Serialize to YAML, omitting fields with default values.
    ///
    /// Used for saving to config files to keep them minimal.
    #[must_use]
    pub fn to_minimal_yaml(&self) -> String {
        let defaults = Self::default();
        let mut lines = vec![format!("name: {}", self.name)];

        if self.chain_id != defaults.chain_id {
            lines.push(format!("chain_id: {}", self.chain_id));
        }
        if self.prover_mode != defaults.prover_mode {
            lines.push(format!("prover_mode: {}", self.prover_mode));
        }
        if let Some(addr) = &self.base_token_address {
            lines.push(format!("base_token_address: \"{}\"", addr));
        }
        if self.base_token_price_nominator != defaults.base_token_price_nominator {
            lines.push(format!(
                "base_token_price_nominator: {}",
                self.base_token_price_nominator
            ));
        }
        if self.base_token_price_denominator != defaults.base_token_price_denominator {
            lines.push(format!(
                "base_token_price_denominator: {}",
                self.base_token_price_denominator
            ));
        }
        if self.evm_emulator != defaults.evm_emulator {
            lines.push(format!("evm_emulator: {}", self.evm_emulator));
        }
        if self.blobs != defaults.blobs {
            lines.push(format!("blobs: {}", self.blobs));
        }
        if self.operators != defaults.operators {
            lines.push("operators:".to_string());
            if let Some(op) = &self.operators.operator {
                lines.push(format!("  operator: \"{}\"", op));
            }
            if let Some(op) = &self.operators.prove_operator {
                lines.push(format!("  prove_operator: \"{}\"", op));
            }
            if let Some(op) = &self.operators.execute_operator {
                lines.push(format!("  execute_operator: \"{}\"", op));
            }
        }
        if self.funding != defaults.funding {
            lines.push("funding:".to_string());
            if let Some(v) = self.funding.operator_eth {
                lines.push(format!("  operator_eth: {}", v));
            }
            if let Some(v) = self.funding.prove_operator_eth {
                lines.push(format!("  prove_operator_eth: {}", v));
            }
            if let Some(v) = self.funding.execute_operator_eth {
                lines.push(format!("  execute_operator_eth: {}", v));
            }
        }
        if self.ownership != defaults.ownership {
            if let Some(owner) = &self.ownership.new_owner {
                lines.push("ownership:".to_string());
                lines.push(format!("  new_owner: \"{}\"", owner));
            }
        }

        lines.join("\n")
    }
}

/// Ecosystem-level ownership configuration.
///
/// Used for transferring/accepting ecosystem ownership (separate from chain ownership).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EcosystemOwnershipDefaults {
    /// Address to transfer ecosystem ownership to (newEcosystemOwner).
    #[serde(default)]
    pub new_owner: Option<Address>,

    /// Private key for accepting ecosystem ownership.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub private_key: Option<SecretString>,
}

/// Ecosystem-level configuration defaults.
///
/// Contains ecosystem-wide settings and nested chain configurations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EcosystemDefaults {
    /// Ecosystem name.
    #[serde(default = "default_ecosystem_name")]
    pub name: String,

    /// L1 network (settlement layer).
    #[serde(default)]
    pub l1_network: L1Network,

    /// Settlement layer RPC URL.
    #[serde(default)]
    pub rpc_url: Option<Url>,

    /// Ecosystem-level ownership configuration.
    #[serde(default)]
    pub ownership: EcosystemOwnershipDefaults,

    /// Chain configurations.
    #[serde(default)]
    pub chains: Vec<ChainDefaults>,
}

fn default_ecosystem_name() -> String {
    "adi_ecosystem".to_string()
}

impl Default for EcosystemDefaults {
    fn default() -> Self {
        Self {
            name: default_ecosystem_name(),
            l1_network: L1Network::Sepolia,
            rpc_url: None,
            ownership: EcosystemOwnershipDefaults::default(),
            chains: Vec::new(),
        }
    }
}

impl EcosystemDefaults {
    /// Get a chain by name.
    #[must_use]
    pub fn get_chain(&self, name: &str) -> Option<&ChainDefaults> {
        self.chains.iter().find(|c| c.name == name)
    }

    /// Get a mutable reference to a chain by name.
    pub fn get_chain_mut(&mut self, name: &str) -> Option<&mut ChainDefaults> {
        self.chains.iter_mut().find(|c| c.name == name)
    }

    /// Get all chain names.
    #[must_use]
    pub fn chain_names(&self) -> Vec<&str> {
        self.chains.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get the first/default chain if available.
    #[must_use]
    pub fn default_chain(&self) -> Option<&ChainDefaults> {
        self.chains.first()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_defaults() {
        let chain = ChainDefaults::default();
        assert_eq!(chain.name, "adi");
        assert_eq!(chain.chain_id, 222);
        assert_eq!(chain.prover_mode, ProverMode::NoProofs);
        assert!(chain.base_token_address.is_none());
        assert!(!chain.evm_emulator);
        assert!(!chain.blobs);
    }

    #[test]
    fn test_ecosystem_defaults() {
        let ecosystem = EcosystemDefaults::default();
        assert_eq!(ecosystem.name, "adi_ecosystem");
        assert_eq!(ecosystem.l1_network, L1Network::Sepolia);
        assert!(ecosystem.chains.is_empty());
    }

    #[test]
    fn test_get_chain() {
        let mut ecosystem = EcosystemDefaults::default();
        ecosystem.chains.push(ChainDefaults {
            name: "chain_a".to_string(),
            chain_id: 100,
            ..Default::default()
        });
        ecosystem.chains.push(ChainDefaults {
            name: "chain_b".to_string(),
            chain_id: 200,
            ..Default::default()
        });

        assert!(ecosystem.get_chain("chain_a").is_some());
        assert_eq!(ecosystem.get_chain("chain_a").unwrap().chain_id, 100);
        assert!(ecosystem.get_chain("chain_b").is_some());
        assert!(ecosystem.get_chain("nonexistent").is_none());

        let names = ecosystem.chain_names();
        assert_eq!(names, vec!["chain_a", "chain_b"]);
    }

    #[test]
    fn test_deserialize_chain_defaults() {
        let yaml = r#"
name: my_chain
chain_id: 456
prover_mode: Gpu
operators:
  operator: "0x1234567890123456789012345678901234567890"
funding:
  operator_eth: 30.0
ownership:
  new_owner: "0xabcdef0123456789abcdef0123456789abcdef01"
"#;
        let chain: ChainDefaults = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(chain.name, "my_chain");
        assert_eq!(chain.chain_id, 456);
        assert_eq!(chain.prover_mode, ProverMode::Gpu);
        assert!(chain.operators.operator.is_some());
        assert_eq!(chain.funding.operator_eth, Some(30.0));
        assert!(chain.ownership.new_owner.is_some());
    }

    #[test]
    fn test_deserialize_ecosystem_defaults() {
        let yaml = r#"
name: my_ecosystem
l1_network: Sepolia
rpc_url: "https://sepolia.example.com"
ownership:
  new_owner: "0x1111111111111111111111111111111111111111"
chains:
  - name: chain_a
    chain_id: 222
    blobs: true
  - name: chain_b
    chain_id: 333
"#;
        let ecosystem: EcosystemDefaults = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ecosystem.name, "my_ecosystem");
        assert_eq!(ecosystem.l1_network, L1Network::Sepolia);
        assert!(ecosystem.ownership.new_owner.is_some());
        assert_eq!(ecosystem.chains.len(), 2);
        assert_eq!(ecosystem.chains[0].name, "chain_a");
        assert!(ecosystem.chains[0].blobs);
        assert_eq!(ecosystem.chains[1].name, "chain_b");
        assert!(!ecosystem.chains[1].blobs);
    }
}
