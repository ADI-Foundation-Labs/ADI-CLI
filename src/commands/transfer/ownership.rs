//! Transfer ownership command implementation.
//!
//! This command first accepts pending ownership transfers (like the accept command),
//! then transfers ownership to a new owner address.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, transfer_all_ownership, transfer_chain_ownership,
    OwnershipSummary,
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

/// Resolved transfer configuration bundling all data needed by execution phases.
struct TransferConfig<'a> {
    rpc_url: Url,
    gas_multiplier: Option<u64>,
    ecosystem_data: Option<(EcosystemContracts, Wallet)>,
    chain_data: Option<(ChainContracts, Wallet)>,
    chain_name: Option<String>,
    ecosystem_new_owner: Option<Address>,
    chain_new_owner: Option<Address>,
    context: &'a Context,
}

/// Aggregated results from accept and transfer phases.
struct TransferSummaries {
    ecosystem_accept: Option<OwnershipSummary>,
    ecosystem_transfer: Option<OwnershipSummary>,
    chain_accept: Option<OwnershipSummary>,
    chain_transfer: Option<OwnershipSummary>,
}

/// Execute the transfer ownership command.
pub async fn run(args: TransferArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Transfer Ownership")?;

    let config = resolve_config(&args, context).await?;
    display_config(&config, &args)?;
    check_ownership_statuses(&config).await?;

    if args.dry_run {
        ui::outro("Dry-run mode: no transactions will be executed")?;
        return Ok(());
    }

    if !confirm_transfer(&config, args.yes)? {
        ui::outro_cancel("Aborted by user")?;
        return Ok(());
    }

    let summaries = execute_phases(&config).await?;
    display_results(&config, &summaries)
}

/// Resolve all configuration: names, URLs, owners, contracts, wallets, gas.
async fn resolve_config<'a>(
    args: &TransferArgs,
    context: &'a Context,
) -> Result<TransferConfig<'a>> {
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    let include_ecosystem = matches!(args.scope, OwnershipScope::Ecosystem | OwnershipScope::All);
    let include_chain = matches!(args.scope, OwnershipScope::Chain | OwnershipScope::All);

    let state_manager = create_state_manager_with_context(&ecosystem_name, context)?;

    let chain_name = if include_chain {
        Some(select_chain_from_state(args.chain.as_ref(), &state_manager, &ecosystem_name).await?)
    } else {
        None
    };

    let ecosystem_new_owner = include_ecosystem
        .then(|| resolve_ecosystem_new_owner(args.new_owner, context.config()))
        .transpose()?;

    let chain_new_owner = chain_name
        .as_ref()
        .map(|name| resolve_chain_new_owner(args.new_owner, name, context.config()))
        .transpose()?;

    let ecosystem_data = if include_ecosystem {
        Some(load_ecosystem_data(&state_manager, &ecosystem_name).await?)
    } else {
        None
    };

    let chain_data = if let Some(ref name) = chain_name {
        Some(load_chain_data(&state_manager, name).await?)
    } else {
        None
    };

    let gas_multiplier = args
        .gas_multiplier
        .or(Some(context.config().gas_multiplier));

    Ok(TransferConfig {
        rpc_url,
        gas_multiplier,
        ecosystem_data,
        chain_data,
        chain_name,
        ecosystem_new_owner,
        chain_new_owner,
        context,
    })
}

/// Load ecosystem contracts and governor wallet from state.
async fn load_ecosystem_data(
    state_manager: &adi_state::StateManager,
    ecosystem_name: &str,
) -> Result<(EcosystemContracts, Wallet)> {
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

    let governor = wallets.governor.ok_or_else(|| {
        eyre::eyre!(
            "Governor wallet not found in ecosystem wallets for '{}'",
            ecosystem_name
        )
    })?;

    Ok((contracts, governor))
}

/// Load chain contracts and governor wallet from state.
async fn load_chain_data(
    state_manager: &adi_state::StateManager,
    chain_name: &str,
) -> Result<(ChainContracts, Wallet)> {
    let contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err(format!(
            "Failed to load chain contracts for '{}'",
            chain_name
        ))?;

    let wallets = state_manager
        .chain(chain_name)
        .wallets()
        .await
        .wrap_err(format!("Failed to load chain wallets for '{}'", chain_name))?;

    let governor = wallets.governor.ok_or_else(|| {
        eyre::eyre!(
            "Governor wallet not found in chain wallets for '{}'",
            chain_name
        )
    })?;

    Ok((contracts, governor))
}

/// Build and display the transfer configuration note.
fn display_config(config: &TransferConfig<'_>, args: &TransferArgs) -> Result<()> {
    let mut lines = vec![
        format!("Scope: {}", ui::green(&args.scope)),
        format!("RPC URL: {}", ui::green(&config.rpc_url)),
    ];
    if let Some(owner) = config.ecosystem_new_owner {
        lines.push(format!("Ecosystem new owner: {}", ui::green(owner)));
    }
    if let Some(ref name) = config.chain_name {
        lines.push(format!("Chain: {}", ui::green(name)));
    }
    if let Some(owner) = config.chain_new_owner {
        lines.push(format!("Chain new owner: {}", ui::green(owner)));
    }
    ui::note("Transfer configuration", lines.join("\n"))?;
    Ok(())
}

/// Check and display ecosystem/chain ownership statuses, warn on pending transfers.
async fn check_ownership_statuses(config: &TransferConfig<'_>) -> Result<()> {
    let mut total_pending: usize = 0;

    if let Some((ref contracts, ref governor)) = config.ecosystem_data {
        let governor_address = derive_address_from_key(&governor.private_key)?;
        ui::info("Checking ecosystem ownership status...")?;
        let status = check_ecosystem_ownership_status(
            config.rpc_url.as_str(),
            contracts,
            governor_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check ecosystem ownership status")?;
        display_ownership_status("Ecosystem contracts", &status)?;
        total_pending += status.pending_count();
    }

    if let (Some(ref name), Some((ref contracts, ref governor))) =
        (&config.chain_name, &config.chain_data)
    {
        let governor_address = derive_address_from_key(&governor.private_key)?;
        ui::info(format!("Checking chain '{}' ownership status...", name))?;
        let status = check_chain_ownership_status(
            config.rpc_url.as_str(),
            contracts,
            governor_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check chain ownership status")?;
        display_ownership_status(&format!("Chain '{}' contracts", name), &status)?;
        total_pending += status.pending_count();
    }

    if total_pending > 0 {
        ui::warning(format!(
            "{} contract(s) have pending ownership transfers.",
            total_pending
        ))?;
    }

    Ok(())
}

/// Build confirmation message and prompt the user. Returns `true` if confirmed.
fn confirm_transfer(config: &TransferConfig<'_>, skip_confirm: bool) -> Result<bool> {
    let msg = match (config.ecosystem_new_owner, config.chain_new_owner) {
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
        (None, None) => return Err(eyre::eyre!("No new owner specified for transfer")),
    };

    if skip_confirm {
        return Ok(true);
    }

    ui::confirm(msg)
        .initial_value(true)
        .interact()
        .wrap_err("Failed to get confirmation")
}

/// Run the accept phase then the transfer phase.
async fn execute_phases(config: &TransferConfig<'_>) -> Result<TransferSummaries> {
    let mut summaries = TransferSummaries {
        ecosystem_accept: None,
        ecosystem_transfer: None,
        chain_accept: None,
        chain_transfer: None,
    };

    // Accept phase
    ui::section("Accept Phase")?;

    if let Some((ref contracts, ref governor)) = config.ecosystem_data {
        ui::info("Processing ecosystem contracts...")?;
        let summary = accept_all_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Ecosystem Accept Summary", &summary)?;
        summaries.ecosystem_accept = Some(summary);
    }

    if let Some((ref contracts, ref governor)) = config.chain_data {
        ui::info("Processing chain contracts...")?;
        let summary = accept_chain_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Chain Accept Summary", &summary)?;
        summaries.chain_accept = Some(summary);
    }

    // Transfer phase
    ui::section("Transfer Phase")?;

    if let (Some((ref contracts, ref governor)), Some(new_owner)) =
        (&config.ecosystem_data, config.ecosystem_new_owner)
    {
        ui::info("Transferring ecosystem contracts...")?;
        let summary = transfer_all_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            new_owner,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Ecosystem Transfer Summary", &summary)?;
        summaries.ecosystem_transfer = Some(summary);
    }

    if let (Some((ref contracts, ref governor)), Some(new_owner)) =
        (&config.chain_data, config.chain_new_owner)
    {
        ui::info("Transferring chain contracts...")?;
        let summary = transfer_chain_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            new_owner,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Chain Transfer Summary", &summary)?;
        summaries.chain_transfer = Some(summary);
    }

    Ok(summaries)
}

/// Aggregate results and display final status with next-step instructions.
fn display_results(config: &TransferConfig<'_>, summaries: &TransferSummaries) -> Result<()> {
    let total_successes = [
        &summaries.ecosystem_accept,
        &summaries.chain_accept,
        &summaries.ecosystem_transfer,
        &summaries.chain_transfer,
    ]
    .iter()
    .filter_map(|s| s.as_ref())
    .map(|s| s.successful_count())
    .sum::<usize>();

    let total_results: usize = [
        &summaries.ecosystem_accept,
        &summaries.chain_accept,
        &summaries.ecosystem_transfer,
        &summaries.chain_transfer,
    ]
    .iter()
    .filter_map(|s| s.as_ref())
    .map(|s| s.results.len())
    .sum();

    if total_results == 0 {
        ui::outro("No contracts were processed")?;
        return Ok(());
    }

    if total_successes == 0 {
        return Err(eyre::eyre!("All ownership operations failed"));
    }

    display_next_steps(config)?;

    ui::outro(format!(
        "Transfer complete! {} operation(s) processed.",
        total_successes
    ))?;
    Ok(())
}

/// Display next-step instructions based on the transfer scope.
fn display_next_steps(config: &TransferConfig<'_>) -> Result<()> {
    let msg = match (config.ecosystem_new_owner, config.chain_new_owner) {
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
        (None, None) => return Ok(()),
    };

    ui::note("Next step", msg)?;
    Ok(())
}
