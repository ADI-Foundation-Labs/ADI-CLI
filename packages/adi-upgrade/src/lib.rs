//! SDK for upgrading ZkSync ecosystem contracts.
//!
//! This crate provides the upgrade orchestration logic for ZkSync
//! ecosystem and chain contracts.

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod error;

pub use error::{Result, UpgradeError};

pub mod versions;

pub use versions::{get_handler, is_supported, PostUpgradeHook, VersionHandler};

mod config;

pub use config::UpgradeConfig;

mod simulation;

pub use simulation::{run_simulation, SimulationResult, ToolkitRunnerTrait};

mod broadcast;

pub use broadcast::{run_broadcast, BroadcastResult};

pub mod validation;

pub use validation::{validate_upgrade_output, BytecodeManifest, ValidationReport};

pub mod governance;

pub use governance::EcosystemGovernance;
