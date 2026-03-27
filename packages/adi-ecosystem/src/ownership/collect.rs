//! Calldata collection for ownership acceptance.
//!
//! Functions that build calldata for ownership acceptance without
//! sending transactions. Used for external submission (e.g., multisig, Safe).

use crate::error::{EcosystemError, Result};
use adi_types::{normalize_rpc_url, ChainContracts, EcosystemContracts, Logger};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::ProviderBuilder;

use super::calldata::{
    build_accept_ownership_calldata, build_accept_ownership_multicall_calldata,
    build_governance_execute_calldata, build_governance_schedule_calldata,
};
use super::status::check_ownership_state;
use super::types::{CalldataEntry, CalldataOutput, OwnershipState};

/// Collect calldata for all ecosystem ownership acceptance without sending.
///
/// This function checks ownership status and builds calldata for contracts
/// with pending ownership transfers. Use this to prepare transactions for
/// external submission (e.g., multisig, Safe).
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `governor` - Governor address that will accept ownership.
/// * `logger` - Logger for debug output.
///
/// # Returns
///
/// Collection of calldata entries for contracts with pending ownership.
pub async fn collect_all_ownership_calldata(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor: Address,
    logger: &dyn Logger,
) -> Result<CalldataOutput> {
    let normalized_rpc = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_rpc
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut output = CalldataOutput::new();

    // Get chain admin for multicall operations
    let chain_admin = contracts.chain_admin_addr();

    // 1. Server Notifier (via multicall through ChainAdmin)
    if let (Some(server_notifier), Some(chain_admin_addr)) =
        (contracts.server_notifier_addr(), chain_admin)
    {
        let state = check_ownership_state(
            &provider,
            server_notifier,
            chain_admin_addr,
            "Server Notifier",
            logger,
        )
        .await;
        if state == OwnershipState::Pending {
            let calldata = build_accept_ownership_multicall_calldata(server_notifier);
            output.push(CalldataEntry::new(
                "Server Notifier",
                chain_admin_addr,
                calldata,
                "multicall([(server_notifier, 0, acceptOwnership())])".to_string(),
            ));
        }
    }

    // 2. Validator Timelock (direct)
    if let Some(timelock) = contracts.validator_timelock_addr() {
        let state =
            check_ownership_state(&provider, timelock, governor, "Validator Timelock", logger)
                .await;
        if state == OwnershipState::Pending {
            let calldata = build_accept_ownership_calldata();
            output.push(CalldataEntry::new(
                "Validator Timelock",
                timelock,
                calldata,
                "acceptOwnership()".to_string(),
            ));
        }
    }

    // 3. Verifier (direct)
    if let Some(verifier) = contracts.verifier_addr() {
        let state = check_ownership_state(&provider, verifier, governor, "Verifier", logger).await;
        if state == OwnershipState::Pending {
            let calldata = build_accept_ownership_calldata();
            output.push(CalldataEntry::new(
                "Verifier",
                verifier,
                calldata,
                "acceptOwnership()".to_string(),
            ));
        }
    }

    // 4. Governance (direct)
    if let Some(governance) = contracts.governance_addr() {
        let state =
            check_ownership_state(&provider, governance, governor, "Governance", logger).await;
        if state == OwnershipState::Pending {
            let calldata = build_accept_ownership_calldata();
            output.push(CalldataEntry::new(
                "Governance",
                governance,
                calldata,
                "acceptOwnership()".to_string(),
            ));
        }
    }

    // 5. Ecosystem Chain Admin (direct)
    if let Some(chain_admin_addr) = contracts.chain_admin_addr() {
        let state = check_ownership_state(
            &provider,
            chain_admin_addr,
            governor,
            "Ecosystem Chain Admin",
            logger,
        )
        .await;
        if state == OwnershipState::Pending {
            let calldata = build_accept_ownership_calldata();
            output.push(CalldataEntry::new(
                "Ecosystem Chain Admin",
                chain_admin_addr,
                calldata,
                "acceptOwnership()".to_string(),
            ));
        }
    }

    // 6. Rollup DA Manager (via governance timelock - 2 transactions)
    if let (Some(da_manager), Some(governance)) = (
        contracts.l1_rollup_da_manager_addr(),
        contracts.governance_addr(),
    ) {
        let state = check_ownership_state(
            &provider,
            da_manager,
            governance,
            "Rollup DA Manager",
            logger,
        )
        .await;
        if state == OwnershipState::Pending {
            let salt = B256::from(U256::from(1u64));

            let schedule_calldata = build_governance_schedule_calldata(da_manager, salt);
            output.push(CalldataEntry::new(
                "Rollup DA Manager (schedule)",
                governance,
                schedule_calldata,
                "scheduleTransparent(Operation{acceptOwnership()}, 0)".to_string(),
            ));

            let execute_calldata = build_governance_execute_calldata(da_manager, salt);
            output.push(CalldataEntry::new(
                "Rollup DA Manager (execute)",
                governance,
                execute_calldata,
                "execute(Operation{acceptOwnership()})".to_string(),
            ));
        }
    }

    Ok(output)
}

/// Collect calldata for chain ownership acceptance without sending.
///
/// This function checks ownership status and builds calldata for chain contracts
/// with pending ownership transfers.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Chain contracts containing addresses.
/// * `governor` - Governor address that will accept ownership.
/// * `logger` - Logger for debug output.
///
/// # Returns
///
/// Collection of calldata entries for contracts with pending ownership.
pub async fn collect_chain_ownership_calldata(
    rpc_url: &str,
    contracts: &ChainContracts,
    governor: Address,
    logger: &dyn Logger,
) -> Result<CalldataOutput> {
    let normalized_rpc = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_rpc
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut output = CalldataOutput::new();

    // 1. Chain Governance (direct)
    if let Some(governance) = contracts.governance_addr() {
        let state =
            check_ownership_state(&provider, governance, governor, "Chain Governance", logger)
                .await;
        if state == OwnershipState::Pending {
            let calldata = build_accept_ownership_calldata();
            output.push(CalldataEntry::new(
                "Chain Governance",
                governance,
                calldata,
                "acceptOwnership()".to_string(),
            ));
        }
    }

    // 2. Chain Admin (direct)
    if let Some(chain_admin) = contracts.chain_admin_addr() {
        let state =
            check_ownership_state(&provider, chain_admin, governor, "Chain Admin", logger).await;
        if state == OwnershipState::Pending {
            let calldata = build_accept_ownership_calldata();
            output.push(CalldataEntry::new(
                "Chain Admin",
                chain_admin,
                calldata,
                "acceptOwnership()".to_string(),
            ));
        }
    }

    Ok(output)
}
