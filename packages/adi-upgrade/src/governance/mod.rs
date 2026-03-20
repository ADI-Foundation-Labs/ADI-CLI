//! Governance transaction execution for upgrades.
//!
//! Handles encoding, resolving, and executing governance calls
//! (scheduleTransparent + execute) on the Governance contract.

mod encode;
mod execute;
mod extract;
mod resolve;

pub use encode::{encode_governance_calls, GovernanceCalldata};
pub use execute::{execute_governance, GovernanceResult};
pub use extract::extract_stage1_calls;
pub use resolve::{resolve_governance_contracts, GovernanceAddresses};
