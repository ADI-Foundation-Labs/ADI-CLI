//! Ownership acceptance orchestrators.
//!
//! High-level functions that accept ownership for all ecosystem
//! or chain-level contracts in sequence.

use adi_types::{ChainContracts, EcosystemContracts, Logger};
use secrecy::SecretString;

use super::context::build_signing_context;
use super::contracts::{accept_ownable2step, accept_rollup_da_manager, accept_server_notifier};
use super::types::OwnershipSummary;

/// Accept ownership for all pending ecosystem contracts.
///
/// This function attempts to accept ownership for:
/// - Server Notifier (via multicall)
/// - Validator Timelock (direct)
/// - Verifier (direct)
/// - Native Token Vault (direct)
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

    // 1. Server Notifier (via multicall through ChainAdmin)
    results.push(accept_server_notifier(&mut ctx, contracts, logger).await);

    // 2. Validator Timelock
    results.push(
        accept_ownable2step(
            &mut ctx,
            contracts.validator_timelock_addr(),
            "Validator Timelock",
            logger,
        )
        .await,
    );

    // 3. Verifier
    results
        .push(accept_ownable2step(&mut ctx, contracts.verifier_addr(), "Verifier", logger).await);

    // 4. Native Token Vault
    results.push(
        accept_ownable2step(
            &mut ctx,
            contracts.native_token_vault_addr(),
            "Native Token Vault",
            logger,
        )
        .await,
    );

    // 5. Governance
    results.push(
        accept_ownable2step(&mut ctx, contracts.governance_addr(), "Governance", logger).await,
    );

    // 6. Ecosystem Chain Admin
    results.push(
        accept_ownable2step(
            &mut ctx,
            contracts.chain_admin_addr(),
            "Ecosystem Chain Admin",
            logger,
        )
        .await,
    );

    // 7. L1 Nullifier
    results.push(
        accept_ownable2step(
            &mut ctx,
            contracts.l1_nullifier_addr(),
            "L1 Nullifier",
            logger,
        )
        .await,
    );

    // 8. Rollup DA Manager (via Governance timelock)
    results.push(accept_rollup_da_manager(&mut ctx, contracts, logger).await);

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

    // 1. Chain Governance
    results.push(
        accept_ownable2step(
            &mut ctx,
            contracts.governance_addr(),
            "Chain Governance",
            logger,
        )
        .await,
    );

    // 2. Chain Admin
    results.push(
        accept_ownable2step(
            &mut ctx,
            contracts.chain_admin_addr(),
            "Chain Admin",
            logger,
        )
        .await,
    );

    OwnershipSummary::new(results)
}
