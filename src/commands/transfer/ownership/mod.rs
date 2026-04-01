//! Transfer ownership command implementation.
//!
//! This command first accepts pending ownership transfers (like the accept command),
//! then transfers ownership to a new owner address.

mod data;
mod execute;
mod results;

use adi_types::{ChainContracts, EcosystemContracts, Wallet};
use alloy_primitives::Address;
use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_new_owner, resolve_ecosystem_name,
    resolve_ecosystem_new_owner, resolve_rpc_url, select_chain_from_state, OwnershipScope,
};
use crate::context::Context;
use crate::error::Result;
use crate::ui;

use data::{load_chain_data, load_ecosystem_data};
use execute::{check_ownership_statuses, confirm_transfer, execute_phases};
use results::display_results;

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
pub(super) struct TransferConfig<'a> {
    pub(super) rpc_url: Url,
    pub(super) gas_multiplier: Option<u64>,
    pub(super) ecosystem_data: Option<(EcosystemContracts, Wallet)>,
    pub(super) chain_data: Option<(ChainContracts, Wallet)>,
    pub(super) chain_name: Option<String>,
    pub(super) ecosystem_new_owner: Option<Address>,
    pub(super) chain_new_owner: Option<Address>,
    pub(super) context: &'a Context,
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
