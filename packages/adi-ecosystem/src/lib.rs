//! Domain logic for ZkSync ecosystem management.
//!
//! This crate is completely independent - it knows nothing about Docker.
//! It provides:
//! - Ecosystem configuration types
//! - Command argument builders
//! - Result verification
//!
//! # Overview
//!
//! The `adi-ecosystem` crate contains domain logic for managing ZkSync
//! ecosystems. It builds command arguments and verifies results, but
//! does NOT execute commands directly.
//!
//! This separation allows:
//! - Testing without Docker
//! - Running ecosystem logic as a service
//! - Clear separation of concerns
//!
//! # Example
//!
//! ```rust
//! use adi_ecosystem::{EcosystemConfig, L1Network, ProverMode, build_ecosystem_create_args};
//!
//! // Build ecosystem config
//! let config = EcosystemConfig::builder()
//!     .name("my_ecosystem")
//!     .l1_network(L1Network::Sepolia)
//!     .chain_name("my_chain")
//!     .chain_id(123)
//!     .prover_mode(ProverMode::NoProofs)
//!     .build();
//!
//! // Build zkstack command arguments
//! let args = build_ecosystem_create_args(&config);
//!
//! // Args can now be passed to any executor (Docker, local, etc.)
//! assert!(args.contains(&"ecosystem".to_string()));
//! assert!(args.contains(&"create".to_string()));
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod commands;
mod config;
mod error;
mod types;
mod verify;

// Public re-exports
pub use commands::{build_ecosystem_create_args, ERA_CONTRACTS_PATH};
pub use config::{EcosystemConfig, EcosystemConfigBuilder};
pub use error::{EcosystemError, Result};
pub use types::{L1Network, ProverMode};
pub use verify::verify_ecosystem_created;
