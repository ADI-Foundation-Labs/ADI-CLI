//! v31 upgrade path.
//!
//! Mirrors the crate-root upgrade engine (orchestrator + per-phase helpers +
//! version handler) but drives `protocol_ops` sub-commands instead of a single
//! forge script. v31 is dispatched at the command layer; it does not go through
//! `get_handler`, whose `VersionHandler` trait is forge/chain.toml shaped.

pub mod bundle;
pub mod env_toml;
pub mod orchestrator;
pub mod outputs;
pub mod ownership_prefix;
pub mod readiness;
pub mod script;

pub use bundle::{BundleInfo, BundleTx};
pub use ownership_prefix::{ensure_governance_owned, OwnershipTargets};

pub use env_toml::{
    render_permanent_values, render_v31_input, write_env_tomls, CtmInput, V31EnvInputs,
};
pub use orchestrator::{V31Orchestrator, V31OrchestratorParams};
pub use readiness::{finalized_block_number, wait_for_finalized_block};
