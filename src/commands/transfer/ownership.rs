//! Transfer ownership command implementation.
//!
//! This command first accepts pending ownership transfers (like the accept command),
//! then transfers ownership to a new owner address.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, transfer_all_ownership, transfer_chain_ownership,
};
use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use clap::Args;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::commands::helpers::{
    create_state_manager, derive_address_from_key, display_ownership_status, display_summary,
    resolve_chain_name, resolve_ecosystem_name, resolve_new_owner, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};

/// Arguments for `transfer` command.
///
/// Accepts pending ownership transfers and then transfers ownership
/// to a new owner address.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct TransferArgs {
    /// Ecosystem name (falls back to config file if not provided).
    #[arg(
        long,
        help = "Ecosystem name (falls back to config file if not provided)"
    )]
    pub ecosystem_name: Option<String>,

    /// Settlement layer JSON-RPC URL (falls back to config file if not provided).
    #[arg(
        long,
        env = "ADI_RPC_URL",
        help = "Settlement layer JSON-RPC URL (falls back to config file if not provided)"
    )]
    pub rpc_url: Option<Url>,

    /// Custom gas price in wei.
    #[arg(long, help = "Custom gas price in wei")]
    pub gas_price_wei: Option<u128>,

    /// Preview contracts without executing transactions.
    #[arg(long, help = "Preview contracts without executing transactions")]
    pub dry_run: bool,

    /// Skip confirmation prompt.
    #[arg(long, short = 'y', help = "Skip confirmation prompt")]
    pub yes: bool,

    /// Chain name (falls back to config file if not provided).
    #[arg(long, help = "Chain name (falls back to config file if not provided)")]
    pub chain: Option<String>,

    /// Address to transfer ownership to (falls back to config file if not provided).
    #[arg(
        long,
        help = "Address to transfer ownership to (falls back to config file if not provided)"
    )]
    pub new_owner: Option<Address>,
}

/// Execute the transfer ownership command.
pub async fn run(args: TransferArgs, context: &Context) -> Result<()> {
    log::info!("{}", "Accepting and transferring ownership...".cyan());

    // Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    log::info!("Ecosystem: {}", ecosystem_name.green());

    // Resolve chain name
    let chain_name = resolve_chain_name(args.chain.as_ref(), context.config())?;
    log::info!("Chain: {}", chain_name.green());

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    log::info!("RPC URL: {}", rpc_url.to_string().green());

    // Resolve new owner
    let new_owner = resolve_new_owner(args.new_owner, context.config())?;
    log::info!("New owner: {}", new_owner.to_string().green());

    // Create state manager
    let state_manager = create_state_manager(&ecosystem_name, context.config());

    // Load ecosystem contracts
    let ecosystem_contracts: EcosystemContracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")?;

    // Load ecosystem wallets to get governor key
    let ecosystem_wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;

    let governor = ecosystem_wallets
        .governor
        .as_ref()
        .ok_or_else(|| eyre::eyre!("Governor wallet not found in ecosystem wallets"))?;

    // Get governor address for ownership checks
    let governor_address = derive_address_from_key(&governor.private_key)?;

    // Check ecosystem ownership status
    log::info!("");
    log::info!("{}", "Checking ecosystem ownership status...".cyan());
    let ecosystem_status =
        check_ecosystem_ownership_status(rpc_url.as_str(), &ecosystem_contracts, governor_address)
            .await
            .wrap_err("Failed to check ecosystem ownership status")?;

    // Display ecosystem contracts with pending status
    log::info!("");
    log::info!("{}", "Ecosystem contracts:".cyan());
    display_ownership_status(&ecosystem_status);

    // Load and check chain contracts
    let chain_contracts: ChainContracts = state_manager
        .chain(&chain_name)
        .contracts()
        .await
        .wrap_err(format!(
            "Failed to load chain contracts for '{}'",
            chain_name
        ))?;

    log::info!("");
    log::info!(
        "{}",
        format!("Checking chain '{}' ownership status...", chain_name).cyan()
    );
    let chain_status =
        check_chain_ownership_status(rpc_url.as_str(), &chain_contracts, governor_address)
            .await
            .wrap_err("Failed to check chain ownership status")?;

    log::info!("");
    log::info!("{}", format!("Chain '{}' contracts:", chain_name).cyan());
    display_ownership_status(&chain_status);

    // Show summary of pending contracts
    log::info!("");
    let ecosystem_pending = ecosystem_status.pending_count();
    let chain_pending = chain_status.pending_count();
    let total_pending = ecosystem_pending + chain_pending;

    if total_pending > 0 {
        log::info!(
            "{}",
            format!(
                "{} contract(s) have pending ownership transfers.",
                total_pending
            )
            .yellow()
        );
    }

    // Dry-run mode
    if args.dry_run {
        log::info!(
            "{}",
            "Dry-run mode: no transactions will be executed".yellow()
        );
        return Ok(());
    }

    // Confirmation
    if !args.yes {
        use dialoguer::Confirm;
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Proceed with ownership acceptance and transfer to {}?",
                new_owner
            ))
            .default(true)
            .interact()
            .wrap_err("Failed to get confirmation")?;

        if !confirmed {
            log::info!("Aborted by user");
            return Ok(());
        }
    }

    // ========================================================================
    // ACCEPT PHASE
    // ========================================================================
    log::info!("");
    log::info!("{}", "=== ACCEPT PHASE ===".cyan().bold());

    // Execute ecosystem ownership acceptance
    log::info!("{}", "Processing ecosystem contracts...".cyan());
    let ecosystem_accept_summary = accept_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &governor.private_key,
        args.gas_price_wei,
    )
    .await;

    // Execute chain ownership acceptance
    log::info!("");
    log::info!("{}", "Processing chain contracts...".cyan());
    let chain_accept_summary = accept_chain_ownership(
        rpc_url.as_str(),
        &chain_contracts,
        &governor.private_key,
        args.gas_price_wei,
    )
    .await;

    // Display accept summaries
    log::info!("");
    log::info!("{}", "=== Accept Summary ===".cyan());
    log::info!("Ecosystem:");
    display_summary(&ecosystem_accept_summary);
    log::info!("Chain:");
    display_summary(&chain_accept_summary);

    // ========================================================================
    // TRANSFER PHASE
    // ========================================================================
    log::info!("");
    log::info!("{}", "=== TRANSFER PHASE ===".cyan().bold());

    // Execute ecosystem ownership transfer
    log::info!("{}", "Transferring ecosystem contracts...".cyan());
    let ecosystem_transfer_summary = transfer_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &governor.private_key,
        new_owner,
        args.gas_price_wei,
    )
    .await;

    // Execute chain ownership transfer
    log::info!("");
    log::info!("{}", "Transferring chain contracts...".cyan());
    let chain_transfer_summary = transfer_chain_ownership(
        rpc_url.as_str(),
        &chain_contracts,
        &governor.private_key,
        new_owner,
        args.gas_price_wei,
    )
    .await;

    // Display transfer summaries
    log::info!("");
    log::info!("{}", "=== Transfer Summary ===".cyan());
    log::info!("Ecosystem:");
    display_summary(&ecosystem_transfer_summary);
    log::info!("Chain:");
    display_summary(&chain_transfer_summary);

    // Return appropriate status
    let total_accept_successes =
        ecosystem_accept_summary.successful_count() + chain_accept_summary.successful_count();
    let total_transfer_successes =
        ecosystem_transfer_summary.successful_count() + chain_transfer_summary.successful_count();
    let total_successes = total_accept_successes + total_transfer_successes;

    let total_results = ecosystem_accept_summary.results.len()
        + chain_accept_summary.results.len()
        + ecosystem_transfer_summary.results.len()
        + chain_transfer_summary.results.len();

    if total_successes > 0 {
        log::info!("");
        log::info!(
            "{}",
            format!(
                "Successfully processed {} operation(s). New owner {} must call acceptOwnership() on Ownable2Step contracts.",
                total_successes,
                new_owner
            )
            .green()
        );
        Ok(())
    } else if total_results == 0 {
        log::warn!("No contracts were processed");
        Ok(())
    } else {
        Err(eyre::eyre!("All ownership operations failed"))
    }
}
