//! SDK for upgrading ZkSync ecosystem contracts.
//!
//! This crate provides the upgrade orchestration logic for ZkSync
//! ecosystem and chain contracts.

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod error;
mod signing;

pub use error::{Result, UpgradeError};

pub mod versions;

pub use versions::{get_handler, PostUpgradeHook, VersionHandler};

mod config;

pub use config::UpgradeConfig;

mod simulation;

pub use simulation::{run_simulation, SimulationResult, ToolkitRunnerTrait};

mod broadcast;

pub use broadcast::{run_broadcast, BroadcastResult};

pub mod validation;

pub use validation::{validate_upgrade_output, BytecodeManifest, ValidationReport};

mod orchestrator;

pub use orchestrator::UpgradeOrchestrator;

pub mod onchain;

pub mod chain_toml;

pub use chain_toml::{
    generate_chain_toml, write_chain_toml, ChainTomlConfig, PreviousUpgradeValues,
};

pub mod upgrade_yaml;

pub use upgrade_yaml::{load_previous_upgrade_values, save_upgrade_yaml};

pub mod yaml_generator;

pub mod governance;

pub use governance::{
    encode_governance_calls, execute_governance, extract_stage1_calls,
    resolve_governance_contracts, GovernanceAddresses, GovernanceCalldata, GovernanceResult,
};

pub mod chain_upgrade;

pub use chain_upgrade::{
    extract_chain_calldatas, resolve_chain_contracts, run_chain_upgrade, verify_protocol_versions,
    version_to_protocol_uint, ChainCalldatas, ChainContracts, ChainUpgradeParams,
    ChainUpgradeResult,
};
