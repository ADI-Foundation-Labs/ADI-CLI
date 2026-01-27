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

// Future submodules (to be created in Phase 2):
// pub mod config;    // T009 BaseToken, T010 ProverMode, T011 ChainState, T017 Chain struct
// pub mod contracts; // T015: ChainContracts struct
// pub mod wallets;   // T016: ChainWallets struct
