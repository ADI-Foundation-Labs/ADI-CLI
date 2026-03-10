//! Transfer ownership command implementation.
//!
//! This command first accepts pending ownership transfers (like the accept command),
//! then transfers ownership to a new owner address.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, transfer_all_ownership, transfer_chain_ownership,
    OwnershipStatusSummary, OwnershipSummary,
};
use adi_types::{ChainContracts, EcosystemContracts, Wallet};
use alloy_primitives::Address;
use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, derive_address_from_key, display_ownership_status,
    display_summary, resolve_chain_new_owner, resolve_ecosystem_name, resolve_ecosystem_new_owner,
    resolve_rpc_url, select_chain_from_state, OwnershipScope,
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
    /// Ownership scope: ecosystem, chain, or all (default: all).
    ///
    /// - `ecosystem`: Transfer only ecosystem-level contracts
    /// - `chain`: Transfer only chain-level contracts (requires --chain)
    /// - `all`: Transfer both ecosystem and chain contracts (default)
    #[arg(
        long,
        value_enum,
        default_value = "all",
        help = "Ownership scope: ecosystem, chain, or all"
    )]
    pub scope: OwnershipScope,

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

    /// Gas price multiplier percentage (default: 120 = 20% buffer over estimated gas).
    #[arg(
        long,
        help = "Gas price multiplier percentage (default: 120 = 20% buffer over estimated gas)"
    )]
    pub gas_multiplier: Option<u64>,

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

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    // Determine scope flags
    let include_ecosystem = matches!(args.scope, OwnershipScope::Ecosystem | OwnershipScope::All);
    let include_chain = matches!(args.scope, OwnershipScope::Chain | OwnershipScope::All);

    // Create state manager
    let state_manager = create_state_manager_with_context(&ecosystem_name, context);

    // Resolve chain name if needed for chain scope
    let chain_name: Option<String> = if include_chain {
        Some(select_chain_from_state(args.chain.as_ref(), &state_manager, &ecosystem_name).await?)
    } else {
        None
    };

    // Resolve new owner(s) based on scope
    let ecosystem_new_owner: Option<Address> = if include_ecosystem {
        Some(resolve_ecosystem_new_owner(
            args.new_owner,
            context.config(),
        )?)
    } else {
        None
    };

    let chain_new_owner: Option<Address> = if let Some(ref name) = chain_name {
        Some(resolve_chain_new_owner(
            args.new_owner,
            name,
            context.config(),
        )?)
    } else {
        None
    };

    // Build configuration note
    let mut config_lines = vec![
        format!("Ecosystem: {}", ui::green(&ecosystem_name)),
        format!("Scope: {}", ui::green(&args.scope)),
        format!("RPC URL: {}", ui::green(&rpc_url)),
    ];
    if let Some(owner) = ecosystem_new_owner {
        config_lines.push(format!("Ecosystem new owner: {}", ui::green(owner)));
    }
    if let Some(ref name) = chain_name {
        config_lines.push(format!("Chain: {}", ui::green(name)));
    }
    if let Some(owner) = chain_new_owner {
        config_lines.push(format!("Chain new owner: {}", ui::green(owner)));
    }
    ui::note("Transfer configuration", config_lines.join("\n"))?;

    // Load ecosystem contracts and wallets (if scope includes ecosystem)
    let ecosystem_data: Option<(EcosystemContracts, Wallet)> = if include_ecosystem {
        let contracts = state_manager
            .ecosystem()
            .contracts()
            .await
            .wrap_err("Failed to load ecosystem contracts")?;

        let wallets = state_manager
            .ecosystem()
            .wallets()
            .await
            .wrap_err("Failed to load ecosystem wallets")?;

        let governor = wallets
            .governor
            .ok_or_else(|| eyre::eyre!("Governor wallet not found in ecosystem wallets"))?;

        Some((contracts, governor))
    } else {
        None
    };

    // Load chain contracts and wallets (if scope includes chain)
    let chain_data: Option<(ChainContracts, Wallet)> = if let Some(ref name) = chain_name {
        let contracts = state_manager
            .chain(name)
            .contracts()
            .await
            .wrap_err(format!("Failed to load chain contracts for '{}'", name))?;

        let wallets = state_manager
            .chain(name)
            .wallets()
            .await
            .wrap_err(format!("Failed to load chain wallets for '{}'", name))?;

        let governor = wallets
            .governor
            .ok_or_else(|| eyre::eyre!("Governor wallet not found in chain wallets"))?;

        Some((contracts, governor))
    } else {
        None
    };

    // Check ecosystem ownership status
    let ecosystem_status: Option<OwnershipStatusSummary> =
        if let Some((ref contracts, ref governor)) = ecosystem_data {
            let governor_address = derive_address_from_key(&governor.private_key)?;
            ui::info("Checking ecosystem ownership status...")?;
            let status = check_ecosystem_ownership_status(
                rpc_url.as_str(),
                contracts,
                governor_address,
                context.logger().as_ref(),
            )
            .await
            .wrap_err("Failed to check ecosystem ownership status")?;
            display_ownership_status("Ecosystem contracts", &status)?;
            Some(status)
        } else {
            None
        };

    // Check chain ownership status
    let chain_status: Option<OwnershipStatusSummary> =
        if let (Some(ref name), Some((ref contracts, ref governor))) = (&chain_name, &chain_data) {
            let governor_address = derive_address_from_key(&governor.private_key)?;
            ui::info(format!("Checking chain '{}' ownership status...", name))?;
            let status = check_chain_ownership_status(
                rpc_url.as_str(),
                contracts,
                governor_address,
                context.logger().as_ref(),
            )
            .await
            .wrap_err("Failed to check chain ownership status")?;
            display_ownership_status(&format!("Chain '{}' contracts", name), &status)?;
            Some(status)
        } else {
            None
        };

    // Show summary of pending contracts
    let ecosystem_pending = ecosystem_status.as_ref().map_or(0, |s| s.pending_count());
    let chain_pending = chain_status.as_ref().map_or(0, |s| s.pending_count());
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

    // Confirmation message based on scope
    let confirm_msg = match (&ecosystem_new_owner, &chain_new_owner) {
        (Some(eco), Some(chain)) if eco == chain => {
            format!("Proceed with ownership transfer to {}?", ui::green(eco))
        }
        (Some(eco), Some(chain)) => {
            format!(
                "Proceed with ownership transfer?\n  Ecosystem → {}\n  Chain → {}",
                ui::green(eco),
                ui::green(chain)
            )
        }
        (Some(eco), None) => {
            format!(
                "Proceed with ecosystem ownership transfer to {}?",
                ui::green(eco)
            )
        }
        (None, Some(chain)) => {
            format!(
                "Proceed with chain ownership transfer to {}?",
                ui::green(chain)
            )
        }
        (None, None) => {
            return Err(eyre::eyre!("No new owner specified for transfer"));
        }
    };

    if !args.yes {
        let confirmed = ui::confirm(confirm_msg)
            .initial_value(true)
            .interact()
            .wrap_err("Failed to get confirmation")?;

        if !confirmed {
            ui::outro_cancel("Aborted by user")?;
            return Ok(());
        }
    }

    // Resolve gas multiplier (use config default if not provided)
    let gas_multiplier = args
        .gas_multiplier
        .or(Some(context.config().gas_multiplier));

    // Track summaries for final status
    let mut ecosystem_accept_summary: Option<OwnershipSummary> = None;
    let mut ecosystem_transfer_summary: Option<OwnershipSummary> = None;
    let mut chain_accept_summary: Option<OwnershipSummary> = None;
    let mut chain_transfer_summary: Option<OwnershipSummary> = None;

    // Accept phase
    ui::section("Accept Phase")?;

    // Execute ecosystem ownership acceptance
    if let Some((ref contracts, ref governor)) = ecosystem_data {
        ui::info("Processing ecosystem contracts...")?;
        let summary = accept_all_ownership(
            rpc_url.as_str(),
            contracts,
            &governor.private_key,
            gas_multiplier,
            context.logger().as_ref(),
        )
        .await;
        display_summary("Ecosystem Accept Summary", &summary)?;
        ecosystem_accept_summary = Some(summary);
    }

    // Execute chain ownership acceptance
    if let Some((ref contracts, ref governor)) = chain_data {
        ui::info("Processing chain contracts...")?;
        let summary = accept_chain_ownership(
            rpc_url.as_str(),
            contracts,
            &governor.private_key,
            gas_multiplier,
            context.logger().as_ref(),
        )
        .await;
        display_summary("Chain Accept Summary", &summary)?;
        chain_accept_summary = Some(summary);
    }

    // Transfer phase
    ui::section("Transfer Phase")?;

    // Execute ecosystem ownership transfer
    if let (Some((ref contracts, ref governor)), Some(new_owner)) =
        (&ecosystem_data, ecosystem_new_owner)
    {
        ui::info("Transferring ecosystem contracts...")?;
        let summary = transfer_all_ownership(
            rpc_url.as_str(),
            contracts,
            &governor.private_key,
            new_owner,
            gas_multiplier,
            context.logger().as_ref(),
        )
        .await;
        display_summary("Ecosystem Transfer Summary", &summary)?;
        ecosystem_transfer_summary = Some(summary);
    }

    // Execute chain ownership transfer
    if let (Some((ref contracts, ref governor)), Some(new_owner)) = (&chain_data, chain_new_owner) {
        ui::info("Transferring chain contracts...")?;
        let summary = transfer_chain_ownership(
            rpc_url.as_str(),
            contracts,
            &governor.private_key,
            new_owner,
            gas_multiplier,
            context.logger().as_ref(),
        )
        .await;
        display_summary("Chain Transfer Summary", &summary)?;
        chain_transfer_summary = Some(summary);
    }

    // Return appropriate status
    let total_accept_successes = ecosystem_accept_summary
        .as_ref()
        .map_or(0, |s| s.successful_count())
        + chain_accept_summary
            .as_ref()
            .map_or(0, |s| s.successful_count());
    let total_transfer_successes = ecosystem_transfer_summary
        .as_ref()
        .map_or(0, |s| s.successful_count())
        + chain_transfer_summary
            .as_ref()
            .map_or(0, |s| s.successful_count());
    let total_successes = total_accept_successes + total_transfer_successes;

    let total_results = ecosystem_accept_summary
        .as_ref()
        .map_or(0, |s| s.results.len())
        + chain_accept_summary.as_ref().map_or(0, |s| s.results.len())
        + ecosystem_transfer_summary
            .as_ref()
            .map_or(0, |s| s.results.len())
        + chain_transfer_summary
            .as_ref()
            .map_or(0, |s| s.results.len());

    if total_successes > 0 {
        // Build next step message based on scope
        let next_step_msg = match (&ecosystem_new_owner, &chain_new_owner) {
            (Some(eco), Some(chain)) if eco == chain => {
                format!(
                    "New owner {} must accept ownership:\n\n  {}",
                    ui::green(eco),
                    ui::cyan("adi accept --private-key <NEW_OWNER_PRIVATE_KEY>")
                )
            }
            (Some(eco), Some(chain)) => {
                format!(
                    "New owners must accept ownership:\n\n  Ecosystem ({}): {}\n  Chain ({}): {}",
                    ui::green(eco),
                    ui::cyan("adi accept --scope ecosystem --private-key <KEY>"),
                    ui::green(chain),
                    ui::cyan("adi accept --scope chain --private-key <KEY>")
                )
            }
            (Some(eco), None) => {
                format!(
                    "New owner {} must accept ecosystem ownership:\n\n  {}",
                    ui::green(eco),
                    ui::cyan("adi accept --scope ecosystem --private-key <NEW_OWNER_PRIVATE_KEY>")
                )
            }
            (None, Some(chain)) => {
                format!(
                    "New owner {} must accept chain ownership:\n\n  {}",
                    ui::green(chain),
                    ui::cyan("adi accept --scope chain --private-key <NEW_OWNER_PRIVATE_KEY>")
                )
            }
            (None, None) => String::new(),
        };

        if !next_step_msg.is_empty() {
            ui::note("Next step", next_step_msg)?;
        }

        ui::outro(format!(
            "Transfer complete! {} operation(s) processed.",
            total_successes
        ))?;
        Ok(())
    } else if total_results == 0 {
        ui::outro("No contracts were processed")?;
        Ok(())
    } else {
        Err(eyre::eyre!("All ownership operations failed"))
    }
}
