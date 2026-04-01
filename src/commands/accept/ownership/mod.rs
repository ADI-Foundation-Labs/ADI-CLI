//! Accept ownership command implementation.
//!
//! This command accepts pending ownership transfers for contracts
//! deployed during ecosystem initialization.

mod config;
mod execute;

use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::commands::helpers::OwnershipScope;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for `accept` command.
///
/// Accepts pending ownership transfers for contracts deployed during
/// ecosystem initialization.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct AcceptArgs {
    /// Ownership scope: ecosystem, chain, or all (default: all).
    ///
    /// - `ecosystem`: Accept only ecosystem-level contracts (Governance, ValidatorTimelock, etc.)
    /// - `chain`: Accept only chain-level contracts (requires --chain)
    /// - `all`: Accept both ecosystem and chain contracts (default)
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

    /// Chain name for chain-level ownership acceptance.
    #[arg(long, help = "Chain name for chain-level ownership acceptance")]
    pub chain: Option<String>,

    /// Private key for accepting ownership (hex format).
    /// Use this when accepting ownership as a new owner after transfer.
    /// Prefer environment variable for security.
    #[arg(
        long,
        env = "ADI_PRIVATE_KEY",
        help = "Private key for accepting ownership (hex). Use when accepting as new owner after transfer"
    )]
    pub private_key: Option<String>,

    /// Use stored governor key without prompting.
    #[arg(long, help = "Use stored governor key without prompting")]
    pub use_governor: bool,

    /// Print calldata without sending transactions (for multisig/external submission).
    #[arg(long, help = "Print calldata without sending transactions")]
    pub calldata: bool,
}

/// Execute the accept ownership command.
pub async fn run(args: AcceptArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Accept Ownership")?;

    let cfg = config::resolve_config(&args, context).await?;
    config::display_config(&cfg, &args)?;

    let (eco_status, chain_status) = execute::check_statuses(&cfg).await?;

    let eco_pending = eco_status.as_ref().map_or(0, |s| s.pending_count());
    let chain_pending = chain_status.as_ref().map_or(0, |s| s.pending_count());
    let total_pending = eco_pending + chain_pending;

    if total_pending == 0 {
        ui::outro("No contracts have pending ownership transfers.")?;
        return Ok(());
    }

    ui::warning(format!(
        "{} contract(s) have pending ownership transfers.",
        total_pending
    ))?;

    if args.dry_run {
        ui::outro("Dry-run mode: no transactions will be executed")?;
        return Ok(());
    }

    if args.calldata {
        return execute::collect_calldata(&cfg).await;
    }

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

    let (eco_summary, chain_summary) = execute::execute_acceptance(&cfg).await?;
    execute::evaluate_results(&eco_summary, &chain_summary)
}
