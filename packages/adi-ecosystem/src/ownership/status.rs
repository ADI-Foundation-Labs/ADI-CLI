//! Ownership status checking functionality.
//!
//! This module provides functions to check the ownership status
//! of ecosystem and chain contracts.

use super::types::OwnershipStatusSummary;
use super::types::{ownerCall, pendingOwnerCall, OwnershipState, OwnershipStatus};
use crate::error::{EcosystemError, Result};
use adi_types::{normalize_rpc_url, ChainContracts, EcosystemContracts, Logger};
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
/// * `logger` - Logger for debug output.
///
/// # Returns
///
/// Summary of ownership status for all contracts.
pub async fn check_ecosystem_ownership_status(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor_address: Address,
    logger: &dyn Logger,
) -> Result<OwnershipStatusSummary> {
    // Normalize URL for host-side connections (host.docker.internal -> localhost)
    let normalized_url = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_url
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut statuses = Vec::new();

    logger.debug(&format!(
        "Checking ecosystem ownership with governor: {}",
        governor_address
    ));

    // Get chain_admin for contracts that are owned by it
    let chain_admin = contracts.chain_admin_addr();

    // Check Server Notifier (owned by ChainAdmin, not governor)
    let server_notifier_addr = contracts.server_notifier_addr();
    let state = match (server_notifier_addr, chain_admin) {
        (Some(addr), Some(ca)) => {
            check_ownership_state(&provider, addr, ca, "Server Notifier", logger).await
        }
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
        check_ownership_state(
            &provider,
            addr,
            governor_address,
            "Validator Timelock",
            logger,
        )
        .await
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
        check_ownership_state(&provider, addr, governor_address, "Verifier", logger).await
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
        check_ownership_state(&provider, addr, governor_address, "Governance", logger).await
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
        check_ownership_state(&provider, da_addr, gov_addr, "Rollup DA Manager", logger).await
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
/// * `logger` - Logger for debug output.
///
/// # Returns
///
/// Summary of ownership status for chain contracts.
pub async fn check_chain_ownership_status(
    rpc_url: &str,
    contracts: &ChainContracts,
    governor_address: Address,
    logger: &dyn Logger,
) -> Result<OwnershipStatusSummary> {
    // Normalize URL for host-side connections (host.docker.internal -> localhost)
    let normalized_url = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_url
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut statuses = Vec::new();

    logger.debug(&format!(
        "Checking chain ownership with governor: {}",
        governor_address
    ));

    // Check Chain Governance
    let chain_governance_addr = contracts.governance_addr();
    let state = if let Some(addr) = chain_governance_addr {
        check_ownership_state(
            &provider,
            addr,
            governor_address,
            "Chain Governance",
            logger,
        )
        .await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Chain Governance",
        address: chain_governance_addr,
        state,
    });

    // Check Chain Admin
    let chain_admin_addr = contracts.chain_admin_addr();
    let state = if let Some(addr) = chain_admin_addr {
        check_ownership_state(&provider, addr, governor_address, "Chain Admin", logger).await
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
    contract_name: &str,
    logger: &dyn Logger,
) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = pendingOwnerCall {}.abi_encode();
    let tx = alloy_rpc_types::TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => {
            let addr = result.get(12..32).map(Address::from_slice);
            logger.debug(&format!(
                "  {} pendingOwner() = {:?}",
                contract_name,
                addr.map(|a| a.to_string())
                    .unwrap_or_else(|| "None".to_string())
            ));
            addr
        }
        Err(e) => {
            logger.error(&format!(
                "  {} pendingOwner() call failed: {}",
                contract_name, e
            ));
            None
        }
    }
}

/// Call owner() on a contract and return the result.
pub(crate) async fn call_owner<P>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
    logger: &dyn Logger,
) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = ownerCall {}.abi_encode();
    let tx = alloy_rpc_types::TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => {
            let addr = result.get(12..32).map(Address::from_slice);
            logger.debug(&format!(
                "  {} owner() = {:?}",
                contract_name,
                addr.map(|a| a.to_string())
                    .unwrap_or_else(|| "None".to_string())
            ));
            addr
        }
        Err(e) => {
            logger.error(&format!("  {} owner() call failed: {}", contract_name, e));
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
    contract_name: &str,
    logger: &dyn Logger,
) -> OwnershipState
where
    P: Provider + Clone,
{
    logger.debug(&format!(
        "Checking {} at {} (expected owner: {})",
        contract_name, contract_address, governor_address
    ));

    // Check if governor is pending owner
    let pending_owner = call_pending_owner(provider, contract_address, contract_name, logger).await;
    if pending_owner == Some(governor_address) {
        logger.debug(&format!(
            "  {} -> Pending (pendingOwner matches)",
            contract_name
        ));
        return OwnershipState::Pending;
    }

    // Check if governor is already owner
    let current_owner = call_owner(provider, contract_address, contract_name, logger).await;
    if current_owner == Some(governor_address) {
        logger.debug(&format!("  {} -> Accepted (owner matches)", contract_name));
        return OwnershipState::Accepted;
    }

    // Neither pending nor owner - ownership transfer not initiated
    logger.debug(&format!(
        "  {} -> NotTransferred (neither pendingOwner nor owner matches)",
        contract_name
    ));
    OwnershipState::NotTransferred
}

/// Check ownership status for ecosystem contracts after transfer to new owner.
///
/// This function only checks contracts that are directly owned by the new owner
/// (not proxy-owned contracts like Server Notifier or Rollup DA Manager).
///
/// Contracts checked:
/// - Governance (Ownable2Step - new owner must accept)
/// - Validator Timelock (Ownable2Step - new owner must accept)
/// - Ecosystem Chain Admin (Ownable2Step - new owner must accept)
///
/// Contracts NOT checked (proxy-owned):
/// - Server Notifier (owned by ChainAdmin contract)
/// - Rollup DA Manager (owned by Governance contract)
/// - Verifier (not transferred to new owner)
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `new_owner_address` - New owner address to check as pending owner.
/// * `logger` - Logger for debug output.
///
/// # Returns
///
/// Summary of ownership status for directly-owned contracts only.
pub async fn check_ecosystem_ownership_status_for_new_owner(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    new_owner_address: Address,
    logger: &dyn Logger,
) -> Result<OwnershipStatusSummary> {
    // Normalize URL for host-side connections (host.docker.internal -> localhost)
    let normalized_url = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_url
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut statuses = Vec::new();

    logger.debug(&format!(
        "Checking ecosystem ownership for new owner: {}",
        new_owner_address
    ));

    // Check Governance (directly transferred to new owner)
    let governance_addr = contracts.governance_addr();
    let state = if let Some(addr) = governance_addr {
        check_ownership_state(&provider, addr, new_owner_address, "Governance", logger).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Governance",
        address: governance_addr,
        state,
    });

    // Check Ecosystem Chain Admin (directly transferred to new owner)
    let chain_admin_addr = contracts.chain_admin_addr();
    let state = if let Some(addr) = chain_admin_addr {
        check_ownership_state(
            &provider,
            addr,
            new_owner_address,
            "Ecosystem Chain Admin",
            logger,
        )
        .await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Ecosystem Chain Admin",
        address: chain_admin_addr,
        state,
    });

    // Check Validator Timelock (directly transferred to new owner)
    let timelock_addr = contracts.validator_timelock_addr();
    let state = if let Some(addr) = timelock_addr {
        check_ownership_state(
            &provider,
            addr,
            new_owner_address,
            "Validator Timelock",
            logger,
        )
        .await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Validator Timelock",
        address: timelock_addr,
        state,
    });

    // NOTE: The following contracts are NOT checked because they are proxy-owned:
    // - Server Notifier: owned by ChainAdmin contract, not directly by new owner
    // - Rollup DA Manager: owned by Governance contract, not directly by new owner
    // - Verifier: not transferred to new owner per documentation
    // - Bridged Token Beacon: uses Ownable (not Ownable2Step), no accept needed

    Ok(OwnershipStatusSummary { statuses })
}

/// Check ownership state for Ownable contracts (not Ownable2Step).
///
/// This function only calls `owner()` - it does not call `pendingOwner()`
/// since Ownable contracts don't have this function.
///
/// Returns:
/// - `Accepted` if governor is the current owner
/// - `NotTransferred` if governor is not the owner
pub(crate) async fn check_ownership_state_for_ownable<P>(
    provider: &P,
    contract_address: Address,
    governor_address: Address,
    contract_name: &str,
    logger: &dyn Logger,
) -> OwnershipState
where
    P: Provider + Clone,
{
    logger.debug(&format!(
        "Checking {} at {} (expected owner: {}) [Ownable]",
        contract_name, contract_address, governor_address
    ));

    // Only check owner() - Ownable contracts don't have pendingOwner()
    let current_owner = call_owner(provider, contract_address, contract_name, logger).await;
    if current_owner == Some(governor_address) {
        logger.debug(&format!("  {} -> Accepted (owner matches)", contract_name));
        return OwnershipState::Accepted;
    }

    // Governor is not owner
    logger.debug(&format!(
        "  {} -> NotTransferred (owner does not match)",
        contract_name
    ));
    OwnershipState::NotTransferred
}
