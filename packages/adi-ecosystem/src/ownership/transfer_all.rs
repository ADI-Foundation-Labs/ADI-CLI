//! Ownership transfer orchestrators.
//!
//! High-level functions that transfer ownership for all ecosystem
//! or chain-level contracts to a new owner.

use adi_types::{ChainContracts, EcosystemContracts, Logger};
use alloy_primitives::Address;
use console::Style;
use secrecy::SecretString;

use super::context::build_signing_context;
use super::contracts::accept_ownable2step;
use super::transfer::{transfer_bridged_token_beacon, transfer_ownable2step, TransferContext};
use super::types::OwnershipSummary;

/// Transfer ownership for all ecosystem contracts to a new owner.
///
/// This function transfers ownership for:
/// - Governance (Ownable2Step)
/// - Ecosystem Chain Admin (Ownable2Step)
/// - Bridged Token Beacon (Ownable - immediate transfer)
/// - Validator Timelock (Ownable2Step)
/// - Verifier (Ownable2Step)
/// - Native Token Vault (Ownable2Step)
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
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.governance_addr(),
            "Governance",
            &mut ctx,
        )
        .await,
    );

    // 2. Transfer Ecosystem Chain Admin
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.chain_admin_addr(),
            "Ecosystem Chain Admin",
            &mut ctx,
        )
        .await,
    );

    // 3. Transfer Bridged Token Beacon (Ownable - immediate transfer)
    results.push(transfer_bridged_token_beacon(&signing_ctx.provider, contracts, &mut ctx).await);

    // 4. Transfer Validator Timelock
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.validator_timelock_addr(),
            "Validator Timelock",
            &mut ctx,
        )
        .await,
    );

    // 5. Transfer Verifier
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.verifier_addr(),
            "Verifier",
            &mut ctx,
        )
        .await,
    );

    // 6. Transfer Native Token Vault
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.native_token_vault_addr(),
            "Native Token Vault",
            &mut ctx,
        )
        .await,
    );

    // 7. Transfer L1 Nullifier
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.l1_nullifier_addr(),
            "L1 Nullifier",
            &mut ctx,
        )
        .await,
    );

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
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.governance_addr(),
            "Chain Governance",
            &mut ctx,
        )
        .await,
    );

    // 2. Transfer Chain Admin
    results.push(
        transfer_ownable2step(
            &signing_ctx.provider,
            contracts.chain_admin_addr(),
            "Chain Admin",
            &mut ctx,
        )
        .await,
    );

    OwnershipSummary::new(results)
}

/// Reclaim L1 Nullifier ownership from the deployer to the governor.
///
/// zkstack leaves the L1 Nullifier owned by the deployer EOA instead of handing
/// it to the governor like the other ecosystem contracts. This performs the full
/// two-step handover: the deployer calls `transferOwnership(governor)`, then the
/// governor calls `acceptOwnership()`. Idempotent — re-runs skip cleanly once the
/// governor already owns it.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing the L1 Nullifier address.
/// * `deployer_key` - Deployer private key (current owner) for the transfer.
/// * `governor_key` - Governor private key for the acceptance.
/// * `gas_multiplier` - Gas price multiplier percentage. None to use raw estimate.
/// * `logger` - Logger for info/error/warning output.
pub async fn reclaim_l1_nullifier_to_governor(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    deployer_key: &SecretString,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> OwnershipSummary {
    // Build the governor context first to learn the governor address (the
    // transfer target) and to accept the transfer afterwards.
    let mut governor_ctx =
        match build_signing_context(rpc_url, governor_key, gas_multiplier, logger).await {
            Ok(c) => c,
            Err(summary) => return summary,
        };
    let governor_address = governor_ctx.governor_address;

    let mut deployer_ctx =
        match build_signing_context(rpc_url, deployer_key, gas_multiplier, logger).await {
            Ok(c) => c,
            Err(summary) => return summary,
        };

    let mut results = Vec::new();

    // 1. Deployer transfers L1 Nullifier ownership to the governor.
    {
        let mut transfer_ctx = TransferContext {
            governor: deployer_ctx.governor_address, // current owner = deployer
            new_owner: governor_address,
            chain_id: deployer_ctx.chain_id,
            nonce: &mut deployer_ctx.nonce,
            gas_price: deployer_ctx.gas_price,
            logger,
        };
        results.push(
            transfer_ownable2step(
                &deployer_ctx.provider,
                contracts.l1_nullifier_addr(),
                "L1 Nullifier",
                &mut transfer_ctx,
            )
            .await,
        );
    }

    // 2. Governor accepts the pending transfer.
    results.push(
        accept_ownable2step(
            &mut governor_ctx,
            contracts.l1_nullifier_addr(),
            "L1 Nullifier",
            logger,
        )
        .await,
    );

    OwnershipSummary::new(results)
}
