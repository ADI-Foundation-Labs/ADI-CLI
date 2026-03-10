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
mod da;
mod defaults;
mod deploy;
mod error;
mod ownership;
mod types;
mod validator;
mod verify;

pub mod verification;

// Public re-exports
pub use commands::{build_chain_create_args, build_ecosystem_create_args, ERA_CONTRACTS_PATH};
pub use config::{
    validate_chain_id, validate_chain_id_unique, validate_chain_name_unique, ChainConfig,
    ChainConfigBuilder, EcosystemConfig, EcosystemConfigBuilder,
};
pub use da::{configure_l3_da, PubdataSource};
pub use defaults::{
    ChainDefaults, ChainFundingDefaults, ChainOwnershipDefaults, EcosystemDefaults,
    EcosystemOwnershipDefaults, OperatorsDefaults,
};
pub use deploy::{add_validator_roles, remove_validator_roles, DeployedContracts};
pub use error::{EcosystemError, Result};
pub use ownership::{
    accept_all_ownership, accept_chain_ownership, build_accept_ownership_calldata,
    build_accept_ownership_multicall_calldata, build_governance_execute_calldata,
    build_governance_schedule_calldata, build_transfer_ownership_calldata,
    check_chain_ownership_status, check_ecosystem_ownership_status,
    check_ecosystem_ownership_status_for_new_owner, collect_all_ownership_calldata,
    collect_chain_ownership_calldata, transfer_all_ownership, transfer_chain_ownership,
    CalldataEntry, CalldataOutput, OwnershipContract, OwnershipMethod, OwnershipResult,
    OwnershipState, OwnershipStatus, OwnershipStatusSummary, OwnershipSummary,
};
pub use types::{L1Network, ProverMode};
pub use validator::{
    build_add_validator_roles_calldata, build_remove_validator_roles_calldata, ValidatorRoles,
};
pub use verify::{verify_chain_created, verify_ecosystem_created};
