//! Ownership acceptance and transfer for contracts with pending ownership.
//!
//! This module handles accepting and transferring ownership for contracts that use:
//! - Ownable2Step pattern (`acceptOwnership()`)
//! - Multicall pattern (via ChainAdmin)
//! - Governance pattern (via scheduleTransparent + execute)

mod accept;
mod calldata;
mod collect;
mod context;
mod contracts;
mod status;
mod transaction;
mod transfer;
mod transfer_all;
mod types;

// Public re-exports: acceptance orchestrators
pub use accept::{accept_all_ownership, accept_chain_ownership};

// Public re-exports: transfer orchestrators
pub use transfer_all::{transfer_all_ownership, transfer_chain_ownership};

// Public re-exports: calldata collection
pub use collect::{collect_all_ownership_calldata, collect_chain_ownership_calldata};

// Public re-exports: calldata builders
pub use calldata::{
    build_accept_ownership_calldata, build_accept_ownership_multicall_calldata,
    build_governance_execute_calldata, build_governance_schedule_calldata,
    build_transfer_ownership_calldata,
};

// Public re-exports: status checking
pub use status::{
    check_chain_ownership_status, check_ecosystem_ownership_status,
    check_ecosystem_ownership_status_for_new_owner,
};

// Public re-exports: types
pub use types::{
    CalldataEntry, CalldataOutput, OwnershipContract, OwnershipMethod, OwnershipResult,
    OwnershipState, OwnershipStatus, OwnershipStatusSummary, OwnershipSummary,
};
