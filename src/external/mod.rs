//! External tool wrappers for zkstack, forge, and cast CLIs.
//!
//! This module provides typed Rust wrappers around external CLI tools
//! that run inside Docker toolkit containers. The wrappers handle:
//!
//! - Command construction with proper arguments
//! - Async execution via Docker containers
//! - Output parsing and error handling
//! - Version checking and compatibility validation
//!
//! # Supported Tools
//!
//! - **zkstack**: ZkSync ecosystem and chain management CLI
//! - **forge**: Solidity smart contract compilation and deployment
//! - **cast**: Ethereum RPC interactions and calldata encoding

// Future submodules (to be created in Phase 2):
// pub mod zkstack; // T023: ZkstackCli wrapper for ecosystem/chain operations
// pub mod forge;   // T024: ForgeCli wrapper for script execution
// pub mod cast;    // T025: CastCli wrapper for contract interactions
