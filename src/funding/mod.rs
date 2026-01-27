//! Wallet funding logic for deployment operations.
//!
//! This module handles automatic funding of ecosystem and chain wallets
//! before deployment operations. It supports both ETH transfers and
//! ERC-20 token transfers for custom gas tokens (CGT).
//!
//! # Funding Requirements
//!
//! Wallet funding varies based on the chain's base token configuration:
//!
//! - **ETH base token**: Fund wallets with ETH only
//! - **Custom base token (CGT)**: Fund wallets with ETH + CGT
//!
//! | Wallet           | ETH Required | CGT Required* |
//! |------------------|--------------|---------------|
//! | Ecosystem Deployer | 1 ETH       | -             |
//! | Ecosystem Governor | 1 ETH       | 5 CGT         |
//! | Chain Governor     | 1 ETH       | 5 CGT         |
//! | Chain Operator     | 5 ETH       | -             |
//! | Prove Operator     | 5 ETH       | -             |
//! | Execute Operator   | 5 ETH       | -             |
//!
//! *CGT only required when base token != ETH

// Future submodules (to be created in Phase 2):
// pub mod transfer; // T048-T052: Balance checking and ETH/ERC-20 transfers
