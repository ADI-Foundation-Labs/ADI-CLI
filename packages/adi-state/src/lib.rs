//! State management SDK for ADI CLI.
//!
//! This crate provides abstract state storage with support for multiple backends.
//! The default backend is filesystem-based, storing state as YAML files.
//!
//! # Overview
//!
//! The `adi-state` crate separates state storage concerns from business logic:
//! - [`StateBackend`] trait defines low-level key-value operations
//! - [`FilesystemBackend`] implements storage using tokio::fs
//! - [`StateManager`] provides high-level typed API for ecosystem/chain state
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_state::StateManager;
//! use std::path::Path;
//!
//! # async fn example() -> adi_state::Result<()> {
//! let manager = StateManager::new(Path::new("/home/user/.adi_cli/state/my_ecosystem"));
//!
//! // Read ecosystem metadata
//! let metadata = manager.ecosystem().metadata().await?;
//! println!("Ecosystem: {}", metadata.name);
//!
//! // Read chain wallets
//! let wallets = manager.chain("my_chain").wallets().await?;
//!
//! // Update metadata with partial values
//! use adi_types::PartialEcosystemMetadata;
//! let partial = PartialEcosystemMetadata {
//!     default_chain: Some("new_default".to_string()),
//!     ..Default::default()
//! };
//! manager.ecosystem().update_metadata(&partial).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Write requires existing file**: Update operations return `NotFound` if the
//!   target file doesn't exist. This prevents accidental file creation.
//! - **Merge in state layer**: Partial types from `adi-types` are merged here,
//!   keeping type definitions pure.
//! - **Async-first**: All I/O operations are async using tokio.

#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod backend;
mod error;
mod manager;
mod paths;

// Public re-exports
pub use backend::{BackendType, FilesystemBackend, StateBackend};
pub use error::{Result, StateError};
pub use manager::{ChainStateOps, EcosystemStateOps, StateManager};

// Path constants
pub use paths::{
    apps_path, chain_contracts_path, chain_dir, chain_metadata_path, chain_wallets_path,
    ecosystem_contracts_path, ecosystem_wallets_path, erc20_deployments_path,
    initial_deployments_path, APPS_FILE, CHAIN_METADATA, CHAINS_DIR, CONFIGS_DIR, CONTRACTS_FILE,
    ECOSYSTEM_METADATA, ERC20_DEPLOYMENTS_FILE, INITIAL_DEPLOYMENTS_FILE, WALLETS_FILE,
};
