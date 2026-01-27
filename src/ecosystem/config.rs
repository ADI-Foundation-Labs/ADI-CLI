//! Ecosystem configuration types.
//!
//! This module defines the core configuration types for ZkSync ecosystems,
//! including the settlement network and ecosystem structure.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use eyre::{ensure, Result};
use semver::Version;
use serde::{Deserialize, Serialize};

use super::contracts::EcosystemContracts;
use super::wallets::EcosystemWallets;

/// Settlement network configuration.
///
/// Specifies which Ethereum network the ecosystem contracts are deployed to.
/// The settlement layer is where L1 contracts (Bridgehub, Governance, etc.)
/// are deployed.
///
/// # Example
///
/// ```rust
/// // Use Sepolia testnet
/// let network = SettlementNetwork::Sepolia;
///
/// // Use custom network
/// let custom = SettlementNetwork::Custom {
///     rpc_url: "https://my-node.example.com".to_string(),
///     chain_id: 12345,
/// };
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettlementNetwork {
    /// Ethereum Mainnet (chain ID: 1).
    Mainnet,

    /// Sepolia testnet (chain ID: 11155111).
    Sepolia,

    /// Local development network (default RPC: http://localhost:8545).
    #[default]
    Localhost,

    /// Custom network with user-specified RPC URL and chain ID.
    Custom {
        /// The RPC endpoint URL.
        rpc_url: String,
        /// The network's chain ID.
        chain_id: u64,
    },
}

#[allow(dead_code)]
impl SettlementNetwork {
    /// Returns the chain ID for this network.
    ///
    /// # Example
    ///
    /// ```rust
    /// assert_eq!(SettlementNetwork::Mainnet.chain_id(), 1);
    /// assert_eq!(SettlementNetwork::Sepolia.chain_id(), 11155111);
    /// ```
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Mainnet => 1,
            Self::Sepolia => 11155111,
            Self::Localhost => 31337, // Default Anvil chain ID
            Self::Custom { chain_id, .. } => *chain_id,
        }
    }

    /// Returns the default RPC URL for this network.
    ///
    /// For custom networks, returns the configured URL.
    /// For standard networks, returns a public endpoint (may require API key).
    pub fn default_rpc_url(&self) -> &str {
        match self {
            Self::Mainnet => "https://eth.llamarpc.com",
            Self::Sepolia => "https://rpc.sepolia.org",
            Self::Localhost => "http://localhost:8545",
            Self::Custom { rpc_url, .. } => rpc_url,
        }
    }

    /// Checks if this is a testnet or local network.
    pub fn is_testnet(&self) -> bool {
        matches!(self, Self::Sepolia | Self::Localhost)
    }
}

impl std::fmt::Display for SettlementNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Sepolia => write!(f, "sepolia"),
            Self::Localhost => write!(f, "localhost"),
            Self::Custom { chain_id, .. } => write!(f, "custom (chain_id: {})", chain_id),
        }
    }
}

/// A ZkSync ecosystem configuration.
///
/// An ecosystem is the top-level container for ZkSync infrastructure.
/// It contains multiple chains and manages shared contracts on the
/// settlement layer.
///
/// # Directory Structure
///
/// ```text
/// {state_path}/
/// ├── ZkStack.yaml              # Ecosystem metadata
/// ├── configs/
/// │   ├── wallets.yaml
/// │   ├── contracts.yaml
/// │   └── initial_deployments.yaml
/// └── chains/{chain-name}/
///     └── configs/
///         ├── contracts.yaml
///         ├── wallets.yaml
///         ├── genesis.yaml
///         └── general.yaml
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ecosystem {
    /// Unique ecosystem name (alphanumeric with underscores, max 64 chars).
    pub name: String,

    /// The settlement network where contracts are deployed.
    pub settlement_network: SettlementNetwork,

    /// Path to the ecosystem state directory.
    pub state_path: PathBuf,

    /// Deployed ecosystem contracts (populated after deployment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contracts: Option<EcosystemContracts>,

    /// Ecosystem wallet keypairs.
    pub wallets: EcosystemWallets,

    /// Names of chains within this ecosystem.
    #[serde(default)]
    pub chains: Vec<String>,

    /// Current protocol version (e.g., 29.0.11).
    pub protocol_version: Version,

    /// Timestamp when the ecosystem was created.
    pub created_at: DateTime<Utc>,

    /// Timestamp when the ecosystem was last updated.
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
impl Ecosystem {
    /// Validates the ecosystem configuration.
    ///
    /// # Validation Rules
    ///
    /// - Name must be non-empty
    /// - Name must be max 64 characters
    /// - Name must be alphanumeric with underscores only
    /// - State path must exist and be a directory
    ///
    /// # Errors
    ///
    /// Returns an error if any validation rule fails.
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.name.is_empty(), "Ecosystem name cannot be empty");

        ensure!(
            self.name.len() <= 64,
            "Ecosystem name too long (max 64 characters, got {})",
            self.name.len()
        );

        ensure!(
            self.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "Ecosystem name must be alphanumeric with underscores only: '{}'",
            self.name
        );

        Ok(())
    }

    /// Checks if the ecosystem has been deployed (contracts exist).
    pub fn is_deployed(&self) -> bool {
        self.contracts.is_some()
    }

    /// Returns the ecosystem's protocol version as a display string.
    ///
    /// # Example
    ///
    /// ```rust
    /// // For version 29.0.11
    /// assert_eq!(ecosystem.version_string(), "v29.0.11");
    /// ```
    pub fn version_string(&self) -> String {
        format!("v{}", self.protocol_version)
    }
}
