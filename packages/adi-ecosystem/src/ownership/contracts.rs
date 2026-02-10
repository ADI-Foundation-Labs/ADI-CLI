//! Per-contract ownership acceptance handlers.
//!
//! This module contains the acceptance logic for each contract type:
//! - Chain Admin (direct)
//! - Server Notifier (via multicall)
//! - Validator Timelock (direct)
//! - Verifier (direct)
//! - Governance (direct)
//! - Rollup DA Manager (via governance timelock)

use super::calldata::{
    build_accept_ownership_calldata, build_accept_ownership_multicall_calldata,
    build_governance_execute_calldata, build_governance_schedule_calldata,
};
use super::status::check_ownership_state;
use super::transaction::send_ownership_tx;
use super::types::{OwnershipResult, OwnershipState};
use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::Provider;
use colored::Colorize;

/// Accept ownership for Chain Admin contract.
pub(crate) async fn accept_chain_admin<P>(
    provider: &P,
    contracts: &ChainContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let chain_admin = match contracts.chain_admin_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Chain Admin", "chain_admin_addr not configured");
        }
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, chain_admin, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Chain Admin: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Chain Admin", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Chain Admin: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Chain Admin", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, chain_admin, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Chain Admin ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Chain Admin", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Chain Admin ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Chain Admin", e.to_string())
        }
    }
}

/// Accept ownership for Server Notifier via multicall.
pub(crate) async fn accept_server_notifier<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    chain_admin: Option<Address>,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let server_notifier = match contracts.server_notifier_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Server Notifier",
                "server_notifier_proxy_addr not configured",
            );
        }
    };

    let chain_admin_addr = match chain_admin {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Server Notifier", "chain_admin_addr not configured");
        }
    };

    // Check if ownership acceptance is needed
    // Note: Server Notifier is owned by ChainAdmin, not governor
    match check_ownership_state(provider, server_notifier, chain_admin_addr).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Server Notifier: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Server Notifier", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Server Notifier: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Server Notifier", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let calldata = build_accept_ownership_multicall_calldata(server_notifier);

    match send_ownership_tx(
        provider,
        chain_admin_addr,
        calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Server Notifier ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Server Notifier", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Server Notifier ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Server Notifier", e.to_string())
        }
    }
}

/// Accept ownership for Validator Timelock.
pub(crate) async fn accept_validator_timelock<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let timelock = match contracts.validator_timelock_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Validator Timelock",
                "validator_timelock_addr not configured",
            );
        }
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, timelock, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Validator Timelock: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Validator Timelock", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Validator Timelock: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Validator Timelock", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, timelock, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Validator Timelock ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Validator Timelock", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Validator Timelock ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Validator Timelock", e.to_string())
        }
    }
}

/// Accept ownership for Verifier.
pub(crate) async fn accept_verifier<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let verifier = match contracts.verifier_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Verifier", "verifier_addr not configured");
        }
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, verifier, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Verifier: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Verifier", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Verifier: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Verifier", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, verifier, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Verifier ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Verifier", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Verifier ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Verifier", e.to_string())
        }
    }
}

/// Accept ownership for Governance contract.
pub(crate) async fn accept_governance<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let governance = match contracts.governance_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Governance", "governance_addr not configured");
        }
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, governance, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Governance: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Governance", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Governance: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Governance", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, governance, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Governance ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Governance", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Governance ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Governance", e.to_string())
        }
    }
}

/// Accept ownership for Rollup DA Manager via Governance.
///
/// This uses the Governance timelock pattern:
/// 1. Call scheduleTransparent(operation, 0) to schedule the acceptOwnership call
/// 2. Call execute(operation) to execute the scheduled operation
///
/// The operation contains a Call to the DA Manager's acceptOwnership() function.
pub(crate) async fn accept_rollup_da_manager<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let da_manager = match contracts.l1_rollup_da_manager_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Rollup DA Manager",
                "l1_rollup_da_manager not configured",
            );
        }
    };

    let governance = match contracts.governance_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Rollup DA Manager",
                "governance_addr not configured (required for Governance timelock)",
            );
        }
    };

    // Check if ownership acceptance is needed by checking pendingOwner on DA Manager
    // Note: for DA Manager, the expected pending owner is the governance contract
    match check_ownership_state(provider, da_manager, governance).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Rollup DA Manager: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Rollup DA Manager", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Rollup DA Manager: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Rollup DA Manager", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    // Generate a unique salt for this operation (using current nonce as entropy)
    let salt = B256::from(U256::from(*nonce));

    // Step 1: Schedule the operation via Governance
    log::info!("    Scheduling operation via Governance timelock...");
    let schedule_calldata = build_governance_schedule_calldata(da_manager, salt);

    match send_ownership_tx(
        provider,
        governance,
        schedule_calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "    {} Scheduled: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
        }
        Err(e) => {
            log::warn!(
                "  {} Rollup DA Manager schedule failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            return OwnershipResult::failure(
                "Rollup DA Manager",
                format!("Schedule failed: {}", e),
            );
        }
    }

    // Step 2: Execute the scheduled operation
    log::info!("    Executing scheduled operation...");
    let execute_calldata = build_governance_execute_calldata(da_manager, salt);

    match send_ownership_tx(
        provider,
        governance,
        execute_calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Rollup DA Manager ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Rollup DA Manager", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Rollup DA Manager execute failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Rollup DA Manager", format!("Execute failed: {}", e))
        }
    }
}
