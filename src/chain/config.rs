//! Chain configuration types.
//!
//! This module defines the core configuration types for ZkSync chains
//! within an ecosystem.

use chrono::{DateTime, Utc};
use eyre::{ensure, Result};
use serde::{Deserialize, Serialize};

use alloy_primitives::Address;

use super::contracts::ChainContracts;
use super::wallets::ChainWallets;

/// Base token configuration for a chain.
///
/// The base token determines what asset is used as the native token on L2:
/// - `Eth`: ETH becomes the native token (address: 0x0...001)
/// - `Custom`: An ERC-20 from the settlement layer becomes native
///
/// # Custom Gas Token (CGT)
///
/// When using a custom base token:
/// - L2: The specified ERC-20 becomes the native token
/// - L3: Settlement layer native token (e.g., ADI) becomes native
/// - Wallets must be funded with both ETH (for L1 gas) and CGT
///
/// # Example
///
/// ```rust
/// // Default ETH base token
/// let base = BaseToken::Eth;
///
/// // Custom ERC-20 base token
/// let custom = BaseToken::Custom {
///     address: "0x...".parse().unwrap(),
///     symbol: "ADI".to_string(),
///     decimals: 18,
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaseToken {
    /// ETH as native token (default).
    /// Canonical address: 0x0000000000000000000000000000000000000001
    #[default]
    Eth,

    /// Custom ERC-20 token as native token.
    Custom {
        /// ERC-20 contract address on the settlement layer.
        address: Address,
        /// Token symbol (e.g., "ADI").
        symbol: String,
        /// Token decimals (typically 18).
        decimals: u8,
    },
}

impl BaseToken {
    /// The canonical ETH address used in ZkSync.
    pub const ETH_ADDRESS: &'static str = "0x0000000000000000000000000000000000000001";

    /// Returns the token address.
    ///
    /// For ETH, returns the canonical ETH address.
    /// For custom tokens, returns the ERC-20 contract address.
    pub fn address(&self) -> Address {
        match self {
            Self::Eth => Self::ETH_ADDRESS.parse().unwrap_or(Address::ZERO),
            Self::Custom { address, .. } => *address,
        }
    }

    /// Checks if this is a custom (non-ETH) base token.
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom { .. })
    }

    /// Returns the token symbol.
    pub fn symbol(&self) -> &str {
        match self {
            Self::Eth => "ETH",
            Self::Custom { symbol, .. } => symbol,
        }
    }
}

impl std::fmt::Display for BaseToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eth => write!(f, "ETH"),
            Self::Custom {
                symbol, address, ..
            } => {
                write!(f, "{} ({})", symbol, address)
            }
        }
    }
}

/// Prover mode for the chain.
///
/// Determines how transaction validity proofs are generated:
/// - `NoProofs`: No ZK proofs (development/testing only)
/// - `Gpu`: GPU-accelerated proof generation (production)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProverMode {
    /// No proofs mode (for development/testing).
    #[default]
    NoProofs,

    /// GPU-accelerated proof generation.
    Gpu,
}

impl ProverMode {
    /// Returns the zkstack CLI argument for this mode.
    pub fn as_zkstack_arg(self) -> &'static str {
        match self {
            Self::NoProofs => "no-proofs",
            Self::Gpu => "gpu",
        }
    }
}

impl std::fmt::Display for ProverMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoProofs => write!(f, "no-proofs"),
            Self::Gpu => write!(f, "gpu"),
        }
    }
}

/// Chain lifecycle state.
///
/// Tracks the current state of a chain through its lifecycle from
/// initialization to running.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChainState {
    /// Chain configuration created but contracts not deployed.
    #[default]
    Initialized,

    /// Chain contracts deployed and registered with Bridgehub.
    Deployed,

    /// Chain server is running and processing transactions.
    Running,

    /// Chain is in the process of upgrading.
    Upgrading,

    /// Chain server is stopped.
    Stopped,
}

impl ChainState {
    /// Checks if the chain is operational (deployed and running/upgrading).
    pub fn is_operational(self) -> bool {
        matches!(self, Self::Running | Self::Upgrading)
    }

    /// Checks if the chain can be started.
    pub fn can_start(self) -> bool {
        matches!(self, Self::Deployed | Self::Stopped)
    }
}

impl std::fmt::Display for ChainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initialized => write!(f, "initialized"),
            Self::Deployed => write!(f, "deployed"),
            Self::Running => write!(f, "running"),
            Self::Upgrading => write!(f, "upgrading"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

/// A ZkSync chain configuration within an ecosystem.
///
/// A chain represents a ZkSync rollup that settles to the parent ecosystem's
/// settlement layer. Each chain has its own contracts, wallets, and state.
///
/// # Directory Structure
///
/// ```text
/// chains/{name}/
/// └── configs/
///     ├── contracts.yaml
///     ├── wallets.yaml
///     ├── genesis.yaml
///     └── general.yaml
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// Unique chain name within the ecosystem (alphanumeric with underscores).
    pub name: String,

    /// Chain ID (must not conflict with settlement layer).
    pub chain_id: u64,

    /// Name of the parent ecosystem.
    pub ecosystem_name: String,

    /// Base token configuration.
    pub base_token: BaseToken,

    /// Prover mode for proof generation.
    pub prover_mode: ProverMode,

    /// Deployed chain contracts (populated after deployment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contracts: Option<ChainContracts>,

    /// Chain wallet keypairs.
    pub wallets: ChainWallets,

    /// Current chain lifecycle state.
    pub state: ChainState,

    /// Timestamp when the chain was created.
    pub created_at: DateTime<Utc>,

    /// Timestamp when the chain was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Chain {
    /// Reserved chain IDs that cannot be used (settlement layer networks).
    pub const RESERVED_CHAIN_IDS: &'static [u64] = &[
        1,        // Ethereum Mainnet
        11155111, // Sepolia
        31337,    // Anvil/Hardhat default
    ];

    /// Validates the chain configuration.
    ///
    /// # Validation Rules
    ///
    /// - Name must be non-empty
    /// - Name must be max 64 characters
    /// - Name must be alphanumeric with underscores only
    /// - Chain ID must be positive
    /// - Chain ID must not conflict with settlement layer networks
    ///
    /// # Errors
    ///
    /// Returns an error if any validation rule fails.
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.name.is_empty(), "Chain name cannot be empty");

        ensure!(
            self.name.len() <= 64,
            "Chain name too long (max 64 characters, got {})",
            self.name.len()
        );

        ensure!(
            self.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "Chain name must be alphanumeric with underscores only: '{}'",
            self.name
        );

        ensure!(self.chain_id > 0, "Chain ID must be positive");

        ensure!(
            !Self::RESERVED_CHAIN_IDS.contains(&self.chain_id),
            "Chain ID {} is reserved (conflicts with settlement layer network)",
            self.chain_id
        );

        Ok(())
    }

    /// Checks if the chain has been deployed (contracts exist).
    pub fn is_deployed(&self) -> bool {
        self.contracts.is_some()
    }

    /// Checks if the chain uses a custom base token (CGT).
    pub fn uses_custom_gas_token(&self) -> bool {
        self.base_token.is_custom()
    }
}
