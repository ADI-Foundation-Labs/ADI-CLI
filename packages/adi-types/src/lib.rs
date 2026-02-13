//! Shared domain types for ADI CLI ecosystem.
//!
//! This crate provides common types used across ADI packages:
//! - Network configuration ([`L1Network`])
//! - Prover and wallet modes ([`ProverMode`], [`WalletCreation`], [`BatchCommitDataMode`])
//! - Wallet structures ([`Wallet`], [`Wallets`])
//! - YAML configuration types ([`EcosystemMetadata`], [`ChainMetadata`], etc.)
//! - Contract address types ([`EcosystemContracts`], [`ChainContracts`])
//! - Partial types for merge operations ([`PartialEcosystemMetadata`], [`PartialChainMetadata`])
//!
//! # Example
//!
//! ```rust
//! use adi_types::{L1Network, ProverMode, EcosystemMetadata};
//!
//! // Parse network from string (case-insensitive)
//! let network: L1Network = serde_yaml::from_str("Sepolia").unwrap();
//! assert_eq!(network, L1Network::Sepolia);
//!
//! // Prover mode with kebab-case support
//! let prover: ProverMode = serde_yaml::from_str("NoProofs").unwrap();
//! assert_eq!(prover, ProverMode::NoProofs);
//! ```
//!
//! # Serde Behavior
//!
//! All enum types support case-insensitive deserialization to maintain
//! compatibility with both zkstack YAML files (PascalCase) and CLI usage (lowercase).
//!
//! - **Serialization**: lowercase or kebab-case (for backward compatibility)
//! - **Deserialization**: case-insensitive (accepts both "Sepolia" and "sepolia")

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod apps;
mod base_token;
mod contracts;
mod deployments;
mod logger;
mod metadata;
mod network;
mod partial;
mod prover;
mod url;
mod wallet;

// Re-export apps types
pub use apps::{Apps, ExplorerConfig, PortalConfig};

// Re-export base token types
pub use base_token::{BaseToken, ETH_TOKEN_ADDRESS, ETH_TOKEN_ADDRESS_STR};

// Re-export contract types
pub use contracts::{
    BridgeContracts, BridgesConfig, ChainContracts, ChainEcosystemContracts, ChainL1Contracts,
    ChainL2Contracts, CoreEcosystemContracts, EcosystemContracts, L1Contracts, ZkSyncOsCtm,
};

// Re-export deployment types
pub use deployments::{Erc20Deployments, Erc20Token, InitialDeployments};

// Re-export metadata types
pub use metadata::{ChainMetadata, EcosystemMetadata, VmOption};

// Re-export network types
pub use network::L1Network;

// Re-export partial types
pub use partial::{PartialChainMetadata, PartialEcosystemMetadata};

// Re-export prover types
pub use prover::{BatchCommitDataMode, ProverMode, WalletCreation};

// Re-export wallet types
pub use wallet::{Wallet, Wallets};

// Re-export logger types
pub use logger::{LogCrateLogger, Logger, NoopLogger};

// Re-export URL utilities
pub use url::{is_localhost_rpc, normalize_rpc_url};
