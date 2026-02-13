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
use serde::{Deserialize, Serialize};
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, derive_address_from_key, display_ownership_status,
    display_summary, resolve_chain_name, resolve_ecosystem_name, resolve_new_owner,
    resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

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
    ui::intro("ADI Transfer Ownership")?;

    // Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    // Resolve chain name
    let chain_name = resolve_chain_name(args.chain.as_ref(), context.config())?;

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    // Resolve new owner
    let new_owner = resolve_new_owner(args.new_owner, context.config())?;

    ui::note(
        "Transfer configuration",
        format!(
            "Ecosystem: {}\nChain: {}\nRPC URL: {}\nNew owner: {}",
            ui::green(&ecosystem_name),
            ui::green(&chain_name),
            ui::green(&rpc_url),
            ui::green(new_owner)
        ),
    )?;

    // Create state manager
    let state_manager = create_state_manager_with_context(&ecosystem_name, context);

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
    ui::info("Checking ecosystem ownership status...")?;
    let ecosystem_status = check_ecosystem_ownership_status(
        rpc_url.as_str(),
        &ecosystem_contracts,
        governor_address,
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to check ecosystem ownership status")?;

    // Display ecosystem contracts with pending status
    display_ownership_status("Ecosystem contracts", &ecosystem_status)?;

    // Load and check chain contracts
    let chain_contracts: ChainContracts = state_manager
        .chain(&chain_name)
        .contracts()
        .await
        .wrap_err(format!(
            "Failed to load chain contracts for '{}'",
            chain_name
        ))?;

    // Load chain wallets to get chain governor key
    let chain_wallets = state_manager
        .chain(&chain_name)
        .wallets()
        .await
        .wrap_err(format!("Failed to load chain wallets for '{}'", chain_name))?;

    let chain_governor = chain_wallets
        .governor
        .as_ref()
        .ok_or_else(|| eyre::eyre!("Governor wallet not found in chain wallets"))?;

    let chain_governor_address = derive_address_from_key(&chain_governor.private_key)?;

    ui::info(format!(
        "Checking chain '{}' ownership status...",
        chain_name
    ))?;
    let chain_status = check_chain_ownership_status(
        rpc_url.as_str(),
        &chain_contracts,
        chain_governor_address,
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to check chain ownership status")?;

    display_ownership_status(&format!("Chain '{}' contracts", chain_name), &chain_status)?;

    // Show summary of pending contracts
    let ecosystem_pending = ecosystem_status.pending_count();
    let chain_pending = chain_status.pending_count();
    let total_pending = ecosystem_pending + chain_pending;

    if total_pending > 0 {
        ui::warning(format!(
            "{} contract(s) have pending ownership transfers.",
            total_pending
        ))?;
    }

    // Dry-run mode
    if args.dry_run {
        ui::outro("Dry-run mode: no transactions will be executed")?;
        return Ok(());
    }

    // Confirmation
    if !args.yes {
        let confirmed = ui::confirm(format!(
            "Proceed with ownership acceptance and transfer to {}?",
            ui::green(new_owner)
        ))
        .initial_value(true)
        .interact()
        .wrap_err("Failed to get confirmation")?;

        if !confirmed {
            ui::outro_cancel("Aborted by user")?;
            return Ok(());
        }
    }

    // Accept phase
    ui::section("Accept Phase")?;

    // Execute ecosystem ownership acceptance
    ui::info("Processing ecosystem contracts...")?;
    let ecosystem_accept_summary = accept_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &governor.private_key,
        args.gas_price_wei,
        context.logger().as_ref(),
    )
    .await;

    // Execute chain ownership acceptance
    ui::info("Processing chain contracts...")?;
    let chain_accept_summary = accept_chain_ownership(
        rpc_url.as_str(),
        &chain_contracts,
        &chain_governor.private_key,
        args.gas_price_wei,
        context.logger().as_ref(),
    )
    .await;

    // Display accept summaries
    display_summary("Ecosystem Accept Summary", &ecosystem_accept_summary)?;
    display_summary("Chain Accept Summary", &chain_accept_summary)?;

    // Transfer phase
    ui::section("Transfer Phase")?;

    // Execute ecosystem ownership transfer
    ui::info("Transferring ecosystem contracts...")?;
    let ecosystem_transfer_summary = transfer_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &governor.private_key,
        new_owner,
        args.gas_price_wei,
        context.logger().as_ref(),
    )
    .await;

    // Execute chain ownership transfer
    ui::info("Transferring chain contracts...")?;
    let chain_transfer_summary = transfer_chain_ownership(
        rpc_url.as_str(),
        &chain_contracts,
        &chain_governor.private_key,
        new_owner,
        args.gas_price_wei,
        context.logger().as_ref(),
    )
    .await;

    // Display transfer summaries
    display_summary("Ecosystem Transfer Summary", &ecosystem_transfer_summary)?;
    display_summary("Chain Transfer Summary", &chain_transfer_summary)?;

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
        ui::outro(format!(
            "Successfully processed {} operation(s). New owner {} must call acceptOwnership() on Ownable2Step contracts.",
            total_successes,
            new_owner
        ))?;
        Ok(())
    } else if total_results == 0 {
        ui::outro("No contracts were processed")?;
        Ok(())
    } else {
        Err(eyre::eyre!("All ownership operations failed"))
    }
}
