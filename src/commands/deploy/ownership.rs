//! Post-deployment ownership operations.
//!
//! This module handles automatic ownership acceptance and transfer after deployment.
//! Operations are triggered based on config values:
//!
//! 1. **Governor Accept**: Always runs (governor key available from state wallets)
//! 2. **Transfer**: Runs if `ownership.new_owner` is configured
//! 3. **New Owner Accept**: Runs if `ownership.private_key` is also configured

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, transfer_all_ownership, transfer_chain_ownership,
    OwnershipSummary,
};
use adi_state::StateManager;
use alloy_primitives::Address;

use crate::commands::helpers::display_summary;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Result of post-deployment ownership operations.
pub struct OwnershipOperationResult {
    /// Whether ownership was transferred to a new owner.
    pub transferred: bool,
    /// Whether the new owner accepted ownership.
    pub new_owner_accepted: bool,
}

/// Run post-deployment ownership operations based on config.
///
/// This function automatically handles ownership after deployment:
/// 1. Accepts ownership as governor (key available from state wallets)
/// 2. If `new_owner` configured: transfers ownership
/// 3. If `new_owner` + `private_key` configured: accepts as new owner
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL
/// * `state_manager` - State manager for loading contracts and wallets
/// * `chain_name` - Chain name for chain-level operations
/// * `gas_multiplier` - Gas price multiplier percentage
/// * `context` - CLI context with config and logger
///
/// # Returns
///
/// Result indicating what operations were performed.
pub async fn run_post_deploy_ownership(
    rpc_url: &str,
    state_manager: &StateManager,
    chain_name: &str,
    gas_multiplier: Option<u64>,
    context: &Context,
) -> Result<OwnershipOperationResult> {
    let config = context.config();
    let logger = context.logger();

    // Load contracts from state
    let ecosystem_contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")?;

    let chain_contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts")?;

    // Load wallets to get governor key
    let wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;

    let governor = wallets
        .governor
        .as_ref()
        .ok_or_else(|| eyre::eyre!("Governor wallet not found in state"))?;

    let governor_key = governor.private_key.clone();

    // Resolve ownership config
    let ecosystem_new_owner = config.ecosystem.ownership.new_owner;
    let chain_new_owner = config
        .ecosystem
        .get_chain(chain_name)
        .and_then(|c| c.ownership.new_owner);

    let ecosystem_private_key = config.ecosystem.ownership.private_key.clone();
    let chain_private_key = config
        .ecosystem
        .get_chain(chain_name)
        .and_then(|c| c.ownership.private_key.clone());

    let has_new_owner = ecosystem_new_owner.is_some() || chain_new_owner.is_some();
    let has_private_key = ecosystem_private_key.is_some() || chain_private_key.is_some();

    // Step 1: Accept ownership as governor
    ui::section("Accepting ownership as governor")?;

    let eco_accept = accept_all_ownership(
        rpc_url,
        &ecosystem_contracts,
        &governor_key,
        gas_multiplier,
        logger.as_ref(),
    )
    .await;

    let chain_accept = accept_chain_ownership(
        rpc_url,
        &chain_contracts,
        &governor_key,
        gas_multiplier,
        logger.as_ref(),
    )
    .await;

    display_accept_results("Ecosystem Ownership", &eco_accept)?;
    display_accept_results("Chain Ownership", &chain_accept)?;

    // Step 2: Transfer ownership (if new_owner configured)
    let mut transferred = false;
    if has_new_owner {
        ui::section("Transferring ownership to new owner")?;

        if let Some(new_owner) = ecosystem_new_owner {
            let result = transfer_all_ownership(
                rpc_url,
                &ecosystem_contracts,
                &governor_key,
                new_owner,
                gas_multiplier,
                logger.as_ref(),
            )
            .await;
            display_transfer_results("Ecosystem Transfer", &result, new_owner)?;
            transferred = true;
        }

        if let Some(new_owner) = chain_new_owner {
            let result = transfer_chain_ownership(
                rpc_url,
                &chain_contracts,
                &governor_key,
                new_owner,
                gas_multiplier,
                logger.as_ref(),
            )
            .await;
            display_transfer_results("Chain Transfer", &result, new_owner)?;
            transferred = true;
        }
    }

    // Step 3: Accept as new owner (if private_key configured)
    let mut new_owner_accepted = false;
    if has_new_owner && has_private_key {
        ui::section("Accepting ownership as new owner")?;

        if let Some(ref new_owner_key) = ecosystem_private_key {
            let result = accept_all_ownership(
                rpc_url,
                &ecosystem_contracts,
                new_owner_key,
                gas_multiplier,
                logger.as_ref(),
            )
            .await;
            display_accept_results("Ecosystem (New Owner)", &result)?;
            new_owner_accepted = true;
        }

        if let Some(ref new_owner_key) = chain_private_key {
            let result = accept_chain_ownership(
                rpc_url,
                &chain_contracts,
                new_owner_key,
                gas_multiplier,
                logger.as_ref(),
            )
            .await;
            display_accept_results("Chain (New Owner)", &result)?;
            new_owner_accepted = true;
        }
    }

    Ok(OwnershipOperationResult {
        transferred,
        new_owner_accepted,
    })
}

/// Display acceptance results summary.
fn display_accept_results(title: &str, summary: &OwnershipSummary) -> Result<()> {
    if summary.results.is_empty() {
        return Ok(());
    }
    display_summary(title, summary)
}

/// Display transfer results summary with new owner address.
fn display_transfer_results(
    title: &str,
    summary: &OwnershipSummary,
    new_owner: Address,
) -> Result<()> {
    if summary.results.is_empty() {
        return Ok(());
    }
    let title_with_owner = format!("{} → {}", title, new_owner);
    display_summary(&title_with_owner, summary)
}
