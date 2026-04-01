//! Ownership transfer orchestrators.
//!
//! High-level functions that transfer ownership for all ecosystem
//! or chain-level contracts to a new owner.

use adi_types::{ChainContracts, EcosystemContracts, Logger};
use alloy_primitives::Address;
use console::Style;
use secrecy::SecretString;

use super::context::build_signing_context;
use super::transfer::{
    transfer_bridged_token_beacon, transfer_chain_chain_admin, transfer_chain_governance,
    transfer_ecosystem_chain_admin, transfer_governance, transfer_validator_timelock,
    TransferContext,
};
use super::types::OwnershipSummary;

/// Transfer ownership for all ecosystem contracts to a new owner.
///
/// This function transfers ownership for:
/// - Governance (Ownable2Step)
/// - Ecosystem Chain Admin (Ownable2Step)
/// - Bridged Token Beacon (Ownable - immediate transfer)
/// - Validator Timelock (Ownable2Step)
///
/// Note: For Ownable2Step contracts, the new owner must call acceptOwnership()
/// to complete the transfer. The Bridged Token Beacon uses standard Ownable,
/// so ownership transfers immediately.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `new_owner` - Address to transfer ownership to.
/// * `gas_multiplier` - Gas price multiplier percentage (e.g., 120 = 20% buffer). None to use raw estimate.
/// * `logger` - Logger for info/error/warning output.
///
/// # Returns
///
/// Summary of all ownership transfer attempts.
pub async fn transfer_all_ownership(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor_key: &SecretString,
    new_owner: Address,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> OwnershipSummary {
    let mut signing_ctx =
        match build_signing_context(rpc_url, governor_key, gas_multiplier, logger).await {
            Ok(c) => c,
            Err(summary) => return summary,
        };

    let green = Style::new().green();
    logger.info(&format!(
        "Transferring ownership to: {}",
        green.apply_to(new_owner)
    ));

    let mut ctx = TransferContext {
        governor: signing_ctx.governor_address,
        new_owner,
        chain_id: signing_ctx.chain_id,
        nonce: &mut signing_ctx.nonce,
        gas_price: signing_ctx.gas_price,
        logger,
    };

    let mut results = Vec::new();

    // 1. Transfer Governance
    results.push(transfer_governance(&signing_ctx.provider, contracts, &mut ctx).await);

    // 2. Transfer Ecosystem Chain Admin
    results.push(transfer_ecosystem_chain_admin(&signing_ctx.provider, contracts, &mut ctx).await);

    // 3. Transfer Bridged Token Beacon (Ownable - immediate transfer)
    results.push(transfer_bridged_token_beacon(&signing_ctx.provider, contracts, &mut ctx).await);

    // 4. Transfer Validator Timelock
    results.push(transfer_validator_timelock(&signing_ctx.provider, contracts, &mut ctx).await);

    OwnershipSummary::new(results)
}

/// Transfer ownership for chain-level contracts to a new owner.
///
/// This function transfers ownership for:
/// - Chain Governance (Ownable2Step)
/// - Chain Chain Admin (Ownable2Step)
///
/// Note: For Ownable2Step contracts, the new owner must call acceptOwnership()
/// to complete the transfer.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Chain contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `new_owner` - Address to transfer ownership to.
/// * `gas_multiplier` - Gas price multiplier percentage (e.g., 120 = 20% buffer). None to use raw estimate.
/// * `logger` - Logger for info/error/warning output.
///
/// # Returns
///
/// Summary of all ownership transfer attempts.
pub async fn transfer_chain_ownership(
    rpc_url: &str,
    contracts: &ChainContracts,
    governor_key: &SecretString,
    new_owner: Address,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> OwnershipSummary {
    let mut signing_ctx =
        match build_signing_context(rpc_url, governor_key, gas_multiplier, logger).await {
            Ok(c) => c,
            Err(summary) => return summary,
        };

    let green = Style::new().green();
    logger.info(&format!(
        "Transferring chain ownership to: {}",
        green.apply_to(new_owner)
    ));

    let mut ctx = TransferContext {
        governor: signing_ctx.governor_address,
        new_owner,
        chain_id: signing_ctx.chain_id,
        nonce: &mut signing_ctx.nonce,
        gas_price: signing_ctx.gas_price,
        logger,
    };

    let mut results = Vec::new();

    // 1. Transfer Chain Governance
    results.push(transfer_chain_governance(&signing_ctx.provider, contracts, &mut ctx).await);

    // 2. Transfer Chain Chain Admin
    results.push(transfer_chain_chain_admin(&signing_ctx.provider, contracts, &mut ctx).await);

    OwnershipSummary::new(results)
}
