//! Transfer ownership handlers for contracts.
//!
//! This module contains the transfer logic for each contract type.
//! These functions are used after accepting ownership to transfer
//! it to a final owner address.

use super::calldata::build_transfer_ownership_calldata;
use super::status::check_ownership_state;
use super::transaction::send_ownership_tx;
use super::types::{bridgedTokenBeaconCall, OwnershipResult, OwnershipState};
use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::SolCall;
use colored::Colorize;

/// Query bridged token beacon address from NativeTokenVault contract.
pub(crate) async fn get_bridged_token_beacon<P>(
    provider: &P,
    native_token_vault: Address,
) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = bridgedTokenBeaconCall {}.abi_encode();
    let tx = TransactionRequest::default()
        .to(native_token_vault)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => result.get(12..32).map(Address::from_slice),
        Err(e) => {
            log::debug!(
                "Failed to query bridgedTokenBeacon from {}: {}",
                native_token_vault,
                e
            );
            None
        }
    }
}

/// Transfer ownership for Governance contract.
pub(crate) async fn transfer_governance<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    new_owner: Address,
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

    // Verify governor is current owner before transferring
    match check_ownership_state(provider, governance, governor).await {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Governance",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped("Governance", "governor is not the current owner");
        }
    }

    let calldata = build_transfer_ownership_calldata(new_owner);

    match send_ownership_tx(
        provider, governance, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Governance ownership transferred: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Governance", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Governance transfer failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Governance", e.to_string())
        }
    }
}

/// Transfer ownership for ecosystem Chain Admin contract.
pub(crate) async fn transfer_ecosystem_chain_admin<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    new_owner: Address,
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
            return OwnershipResult::skipped(
                "Ecosystem Chain Admin",
                "chain_admin_addr not configured",
            );
        }
    };

    // Verify governor is current owner before transferring
    match check_ownership_state(provider, chain_admin, governor).await {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Ecosystem Chain Admin",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Ecosystem Chain Admin",
                "governor is not the current owner",
            );
        }
    }

    let calldata = build_transfer_ownership_calldata(new_owner);

    match send_ownership_tx(
        provider,
        chain_admin,
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
                "  {} Ecosystem Chain Admin ownership transferred: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Ecosystem Chain Admin", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Ecosystem Chain Admin transfer failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Ecosystem Chain Admin", e.to_string())
        }
    }
}

/// Transfer ownership for Validator Timelock contract.
pub(crate) async fn transfer_validator_timelock<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    new_owner: Address,
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

    // Verify governor is current owner before transferring
    match check_ownership_state(provider, timelock, governor).await {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Validator Timelock",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Validator Timelock",
                "governor is not the current owner",
            );
        }
    }

    let calldata = build_transfer_ownership_calldata(new_owner);

    match send_ownership_tx(
        provider, timelock, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Validator Timelock ownership transferred: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Validator Timelock", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Validator Timelock transfer failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Validator Timelock", e.to_string())
        }
    }
}

/// Transfer ownership for Bridged Token Beacon contract.
///
/// Note: This contract uses Ownable (not Ownable2Step), so ownership
/// transfers immediately without requiring an accept step.
pub(crate) async fn transfer_bridged_token_beacon<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    new_owner: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    // Get native token vault address
    let native_token_vault = match contracts.native_token_vault_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Bridged Token Beacon",
                "native_token_vault_addr not configured",
            );
        }
    };

    // Query bridged token beacon address from native token vault
    let beacon = match get_bridged_token_beacon(provider, native_token_vault).await {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Bridged Token Beacon",
                "failed to query bridgedTokenBeacon from native token vault",
            );
        }
    };

    log::info!("    Bridged Token Beacon address: {}", beacon);

    // Verify governor is current owner before transferring
    // Note: Bridged Token Beacon uses Ownable, not Ownable2Step
    match check_ownership_state(provider, beacon, governor).await {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending | OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Bridged Token Beacon",
                "governor is not the current owner",
            );
        }
    }

    let calldata = build_transfer_ownership_calldata(new_owner);

    match send_ownership_tx(
        provider, beacon, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Bridged Token Beacon ownership transferred: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Bridged Token Beacon", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Bridged Token Beacon transfer failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Bridged Token Beacon", e.to_string())
        }
    }
}

/// Transfer ownership for chain Governance contract.
pub(crate) async fn transfer_chain_governance<P>(
    provider: &P,
    contracts: &ChainContracts,
    governor: Address,
    new_owner: Address,
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
            return OwnershipResult::skipped("Chain Governance", "governance_addr not configured");
        }
    };

    // Verify governor is current owner before transferring
    match check_ownership_state(provider, governance, governor).await {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Chain Governance",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Chain Governance",
                "governor is not the current owner",
            );
        }
    }

    let calldata = build_transfer_ownership_calldata(new_owner);

    match send_ownership_tx(
        provider, governance, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Chain Governance ownership transferred: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Chain Governance", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Chain Governance transfer failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Chain Governance", e.to_string())
        }
    }
}

/// Transfer ownership for chain Chain Admin contract.
pub(crate) async fn transfer_chain_chain_admin<P>(
    provider: &P,
    contracts: &ChainContracts,
    governor: Address,
    new_owner: Address,
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
            return OwnershipResult::skipped(
                "Chain Chain Admin",
                "chain_admin_addr not configured",
            );
        }
    };

    // Verify governor is current owner before transferring
    match check_ownership_state(provider, chain_admin, governor).await {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Chain Chain Admin",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Chain Chain Admin",
                "governor is not the current owner",
            );
        }
    }

    let calldata = build_transfer_ownership_calldata(new_owner);

    match send_ownership_tx(
        provider,
        chain_admin,
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
                "  {} Chain Chain Admin ownership transferred: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Chain Chain Admin", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Chain Chain Admin transfer failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Chain Chain Admin", e.to_string())
        }
    }
}
