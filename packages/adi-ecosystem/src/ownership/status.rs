//! Ownership status checking functionality.
//!
//! This module provides functions to check the ownership status
//! of ecosystem and chain contracts.

use super::types::OwnershipStatusSummary;
use super::types::{ownerCall, pendingOwnerCall, OwnershipState, OwnershipStatus};
use crate::error::{EcosystemError, Result};
use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_sol_types::SolCall;

/// Check ownership status for all ecosystem contracts.
///
/// This function queries `pendingOwner()` on each contract to determine
/// which ones have pending ownership transfers that need to be accepted.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `governor_address` - Governor address to check as pending owner.
///
/// # Returns
///
/// Summary of ownership status for all contracts.
pub async fn check_ecosystem_ownership_status(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor_address: Address,
) -> Result<OwnershipStatusSummary> {
    let url: url::Url = rpc_url
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut statuses = Vec::new();

    // Get chain_admin for contracts that are owned by it
    let chain_admin = contracts.chain_admin_addr();

    // Check Server Notifier (owned by ChainAdmin, not governor)
    let server_notifier_addr = contracts.server_notifier_addr();
    let state = match (server_notifier_addr, chain_admin) {
        (Some(addr), Some(ca)) => check_ownership_state(&provider, addr, ca).await,
        _ => OwnershipState::NotTransferred,
    };
    statuses.push(OwnershipStatus {
        name: "Server Notifier",
        address: server_notifier_addr,
        state,
    });

    // Check Validator Timelock
    let timelock_addr = contracts.validator_timelock_addr();
    let state = if let Some(addr) = timelock_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Validator Timelock",
        address: timelock_addr,
        state,
    });

    // Check Verifier
    let verifier_addr = contracts.verifier_addr();
    let state = if let Some(addr) = verifier_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Verifier",
        address: verifier_addr,
        state,
    });

    // Check Governance
    let governance_addr = contracts.governance_addr();
    let state = if let Some(addr) = governance_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Governance",
        address: governance_addr,
        state,
    });

    // Check Rollup DA Manager (pending owner should be governance, not governor)
    let da_manager_addr = contracts.l1_rollup_da_manager_addr();
    let state = if let (Some(da_addr), Some(gov_addr)) = (da_manager_addr, governance_addr) {
        check_ownership_state(&provider, da_addr, gov_addr).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Rollup DA Manager",
        address: da_manager_addr,
        state,
    });

    Ok(OwnershipStatusSummary { statuses })
}

/// Check ownership status for chain contracts.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Chain contracts containing addresses.
/// * `governor_address` - Governor address to check as pending owner.
///
/// # Returns
///
/// Summary of ownership status for chain contracts.
pub async fn check_chain_ownership_status(
    rpc_url: &str,
    contracts: &ChainContracts,
    governor_address: Address,
) -> Result<OwnershipStatusSummary> {
    let url: url::Url = rpc_url
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut statuses = Vec::new();

    // Check Chain Admin
    let chain_admin_addr = contracts.chain_admin_addr();
    let state = if let Some(addr) = chain_admin_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Chain Admin",
        address: chain_admin_addr,
        state,
    });

    Ok(OwnershipStatusSummary { statuses })
}

/// Call pendingOwner() on a contract and return the result.
pub(crate) async fn call_pending_owner<P>(
    provider: &P,
    contract_address: Address,
) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = pendingOwnerCall {}.abi_encode();
    let tx = alloy_rpc_types::TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => result.get(12..32).map(Address::from_slice),
        Err(e) => {
            log::debug!(
                "Failed to call pendingOwner for {}: {}",
                contract_address,
                e
            );
            None
        }
    }
}

/// Call owner() on a contract and return the result.
pub(crate) async fn call_owner<P>(provider: &P, contract_address: Address) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = ownerCall {}.abi_encode();
    let tx = alloy_rpc_types::TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => result.get(12..32).map(Address::from_slice),
        Err(e) => {
            log::debug!("Failed to call owner for {}: {}", contract_address, e);
            None
        }
    }
}

/// Check the ownership state of a contract.
///
/// Returns:
/// - `Pending` if governor is the pending owner (needs acceptOwnership)
/// - `Accepted` if governor is already the owner
/// - `NotTransferred` if ownership was never transferred to governor
pub(crate) async fn check_ownership_state<P>(
    provider: &P,
    contract_address: Address,
    governor_address: Address,
) -> OwnershipState
where
    P: Provider + Clone,
{
    // Check if governor is pending owner
    let pending_owner = call_pending_owner(provider, contract_address).await;
    if pending_owner == Some(governor_address) {
        return OwnershipState::Pending;
    }

    // Check if governor is already owner
    let current_owner = call_owner(provider, contract_address).await;
    if current_owner == Some(governor_address) {
        return OwnershipState::Accepted;
    }

    // Neither pending nor owner - ownership transfer not initiated
    OwnershipState::NotTransferred
}
