//! Chain domain logic for ZkSync chain management.
//!
//! This module contains types and functions for managing ZkSync chains
//! within an ecosystem, including configuration, contracts, and wallets.
//!
//! A chain represents a ZkSync rollup within an ecosystem. Each chain has:
//!
//! - A unique name and chain ID
//! - Base token configuration (ETH or custom ERC-20)
//! - Prover mode (NoProofs, GPU)
//! - Deployed contracts (Diamond Proxy, Chain Admin, bridges)
//! - Wallet keypairs (deployer, governor, operators)
//! - State lifecycle (Initialized, Deployed, Running, Upgrading, Stopped)
//!
//! # Example
//!
//! ```rust
//! use adi_cli::chain::{Chain, BaseToken, ProverMode, ChainState};
//!
//! // Create a chain with ETH as base token
//! let base_token = BaseToken::Eth;
//! let prover_mode = ProverMode::NoProofs;
//! let state = ChainState::Initialized;
//! ```

pub mod config;
pub mod contracts;
pub mod wallets;

// Re-export commonly used types
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(unused_imports)]
pub use config::{BaseToken, Chain, ChainState, ProverMode};
#[allow(unused_imports)]
pub use contracts::ChainContracts;
#[allow(unused_imports)]
pub use wallets::ChainWallets;
