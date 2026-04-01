//! Ownership acceptance orchestrators.
//!
//! High-level functions that accept ownership for all ecosystem
//! or chain-level contracts in sequence.

use adi_types::{ChainContracts, EcosystemContracts, Logger};
use secrecy::SecretString;

use super::context::build_signing_context;
use super::contracts::{
    accept_chain_admin, accept_chain_governance, accept_ecosystem_chain_admin, accept_governance,
    accept_rollup_da_manager, accept_server_notifier, accept_validator_timelock, accept_verifier,
};
use super::types::OwnershipSummary;

/// Accept ownership for all pending ecosystem contracts.
///
/// This function attempts to accept ownership for:
/// - Server Notifier (via multicall)
/// - Validator Timelock (direct)
/// - Verifier (direct)
/// - Governance (direct)
/// - Ecosystem Chain Admin (direct)
/// - Rollup DA Manager (via governance timelock)
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `gas_multiplier` - Gas price multiplier percentage (e.g., 120 = 20% buffer). None to use raw estimate.
/// * `logger` - Logger for info/error/warning output.
///
/// # Returns
///
/// Summary of all ownership acceptance attempts.
pub async fn accept_all_ownership(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> OwnershipSummary {
    let mut ctx = match build_signing_context(rpc_url, governor_key, gas_multiplier, logger).await {
        Ok(c) => c,
        Err(summary) => return summary,
    };

    let mut results = Vec::new();

    // 1. Server Notifier (via multicall)
    results.push(
        accept_server_notifier(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    // 2. Validator Timelock (direct)
    results.push(
        accept_validator_timelock(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    // 3. Verifier (direct)
    results.push(
        accept_verifier(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    // 4. Governance (direct)
    results.push(
        accept_governance(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    // 5. Ecosystem Chain Admin (direct)
    results.push(
        accept_ecosystem_chain_admin(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    // 6. Rollup DA Manager (via governance acceptOwner)
    results.push(
        accept_rollup_da_manager(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    OwnershipSummary::new(results)
}

/// Accept ownership for chain-level contracts.
///
/// This function attempts to accept ownership for chain-specific contracts:
/// - Chain Governance (direct acceptOwnership)
/// - Chain Admin (direct acceptOwnership)
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Chain contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `gas_multiplier` - Gas price multiplier percentage (e.g., 120 = 20% buffer). None to use raw estimate.
/// * `logger` - Logger for info/error/warning output.
///
/// # Returns
///
/// Summary of all ownership acceptance attempts.
pub async fn accept_chain_ownership(
    rpc_url: &str,
    contracts: &ChainContracts,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> OwnershipSummary {
    let mut ctx = match build_signing_context(rpc_url, governor_key, gas_multiplier, logger).await {
        Ok(c) => c,
        Err(summary) => return summary,
    };

    let mut results = Vec::new();

    // 1. Chain Governance (direct)
    results.push(
        accept_chain_governance(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    // 2. Chain Admin (direct)
    results.push(
        accept_chain_admin(
            &ctx.provider,
            contracts,
            ctx.governor_address,
            ctx.chain_id,
            &mut ctx.nonce,
            ctx.gas_price,
            logger,
        )
        .await,
    );

    OwnershipSummary::new(results)
}
