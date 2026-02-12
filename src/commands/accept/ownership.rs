//! Accept ownership command implementation.
//!
//! This command accepts pending ownership transfers for contracts
//! deployed during ecosystem initialization.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, OwnershipStatusSummary,
};
use adi_types::{ChainContracts, EcosystemContracts};
use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, derive_address_from_key, display_ownership_status,
    display_summary, resolve_ecosystem_name, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for `accept` command.
///
/// Accepts pending ownership transfers for contracts deployed during
/// ecosystem initialization.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct AcceptArgs {
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

    /// Chain name for chain-level ownership acceptance.
    #[arg(long, help = "Chain name for chain-level ownership acceptance")]
    pub chain: Option<String>,
}

/// Execute the accept ownership command.
pub async fn run(args: AcceptArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Accept Ownership")?;

    // Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    ui::note(
        "Accept configuration",
        format!(
            "Ecosystem: {}\nRPC URL: {}",
            ui::green(&ecosystem_name),
            ui::green(&rpc_url)
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
    let ecosystem_status =
        check_ecosystem_ownership_status(rpc_url.as_str(), &ecosystem_contracts, governor_address)
            .await
            .wrap_err("Failed to check ecosystem ownership status")?;

    // Display ecosystem contracts with pending status
    ui::info("Ecosystem contracts:")?;
    display_ownership_status(&ecosystem_status)?;

    // Load and check chain contracts if --chain is provided
    let chain_contracts: Option<ChainContracts>;
    let chain_status: Option<OwnershipStatusSummary>;

    if let Some(ref chain_name) = args.chain {
        match state_manager.chain(chain_name).contracts().await {
            Ok(contracts) => {
                ui::info(format!(
                    "Checking chain '{}' ownership status...",
                    chain_name
                ))?;
                let status =
                    check_chain_ownership_status(rpc_url.as_str(), &contracts, governor_address)
                        .await
                        .wrap_err("Failed to check chain ownership status")?;

                ui::info(format!("Chain '{}' contracts:", chain_name))?;
                display_ownership_status(&status)?;

                chain_contracts = Some(contracts);
                chain_status = Some(status);
            }
            Err(e) => {
                ui::warning(format!("Failed to load chain contracts: {}", e))?;
                chain_contracts = None;
                chain_status = None;
            }
        }
    } else {
        chain_contracts = None;
        chain_status = None;
    }

    // Show summary of pending contracts
    let ecosystem_pending = ecosystem_status.pending_count();
    let chain_pending = chain_status.as_ref().map_or(0, |s| s.pending_count());
    let total_pending = ecosystem_pending + chain_pending;

    if total_pending == 0 {
        ui::outro("No contracts have pending ownership transfers.")?;
        return Ok(());
    }

    ui::warning(format!(
        "{} contract(s) have pending ownership transfers.",
        total_pending
    ))?;

    // Dry-run mode
    if args.dry_run {
        ui::outro("Dry-run mode: no transactions will be executed")?;
        return Ok(());
    }

    // Confirmation
    if !args.yes {
        let confirmed = ui::confirm("Proceed with ownership acceptance?")
            .initial_value(true)
            .interact()
            .wrap_err("Failed to get confirmation")?;

        if !confirmed {
            ui::outro_cancel("Aborted by user")?;
            return Ok(());
        }
    }

    // Execute ecosystem ownership acceptance
    ui::info("Processing ecosystem contracts...")?;
    let ecosystem_summary = accept_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &governor.private_key,
        args.gas_price_wei,
        context.logger().as_ref(),
    )
    .await;

    // Execute chain ownership acceptance if --chain was provided
    let chain_summary = if let Some(contracts) = chain_contracts {
        ui::info("Processing chain contracts...")?;
        Some(
            accept_chain_ownership(
                rpc_url.as_str(),
                &contracts,
                &governor.private_key,
                args.gas_price_wei,
                context.logger().as_ref(),
            )
            .await,
        )
    } else {
        None
    };

    // Display summaries
    ui::info("=== Ecosystem Summary ===")?;
    display_summary(&ecosystem_summary)?;

    if let Some(ref summary) = chain_summary {
        ui::info("=== Chain Summary ===")?;
        display_summary(summary)?;
    }

    // Return appropriate status
    let total_successes = ecosystem_summary.successful_count()
        + chain_summary.as_ref().map_or(0, |s| s.successful_count());
    let total_results =
        ecosystem_summary.results.len() + chain_summary.as_ref().map_or(0, |s| s.results.len());

    if total_successes > 0 {
        ui::outro("Ownership acceptance complete!")?;
        Ok(())
    } else if total_results == 0 {
        ui::outro("No contracts were processed")?;
        Ok(())
    } else {
        Err(eyre::eyre!("All ownership acceptances failed"))
    }
}
