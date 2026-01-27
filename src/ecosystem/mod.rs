//! Ecosystem domain logic for ZkSync ecosystem management.
//!
//! This module contains types and functions for managing ZkSync ecosystems,
//! including configuration, contracts, and wallets.
//!
//! An ecosystem is the top-level container for ZkSync infrastructure and can
//! contain multiple chains. Each ecosystem has:
//!
//! - A unique name
//! - Settlement network configuration (Mainnet, Sepolia, Localhost)
//! - Deployed contracts (Bridgehub, Governance, Verifier, etc.)
//! - Wallet keypairs (deployer, governor)
//! - Protocol version tracking

// Future submodules (to be created in Phase 2):
// pub mod config;    // T014: Ecosystem struct with SettlementNetwork enum (T008)
// pub mod contracts; // T012: EcosystemContracts struct
// pub mod wallets;   // T007, T013: Wallet and EcosystemWallets structs
// Protocol version utilities (T018) will also be added here
