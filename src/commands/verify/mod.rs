//! Verify command implementation.
//!
//! This command checks the verification status of deployed smart contracts
//! on block explorers like Etherscan and Blockscout, and can submit
//! unverified contracts for verification.

mod check;
mod config;
mod contracts;
mod submit;

use adi_ecosystem::verification::ExplorerType;
use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for `verify` command.
///
/// Checks verification status of deployed smart contracts on block explorers,
/// and optionally submits unverified contracts for verification.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct VerifyArgs {
    /// Ecosystem name (falls back to config file if not provided).
    #[arg(
        long,
        help = "Ecosystem name (falls back to config file if not provided)"
    )]
    pub ecosystem_name: Option<String>,

    /// Chain name for chain-level contract verification.
    #[arg(long, help = "Chain name for chain-level contract verification")]
    pub chain: Option<String>,

    /// Include ecosystem-level contracts.
    #[arg(long, help = "Include ecosystem contracts")]
    pub ecosystem: bool,

    /// Submit unverified contracts for verification.
    #[arg(long, help = "Submit unverified contracts to block explorer")]
    pub submit: bool,

    /// Continue verification even if some contracts fail.
    #[arg(long, help = "Continue on verification errors")]
    pub continue_on_error: bool,

    /// Protocol version for toolkit image (required for --submit).
    #[arg(long, short = 'p', help = "Protocol version (e.g., v30.0.2)")]
    pub protocol_version: Option<String>,

    /// Dry run: show what would be verified without submitting.
    #[arg(long, help = "Show verification plan without submitting")]
    pub dry_run: bool,

    /// Block explorer type.
    #[arg(
        long,
        value_enum,
        default_value = "etherscan",
        help = "Block explorer type (etherscan, blockscout, custom)"
    )]
    pub explorer: ExplorerType,

    /// Block explorer API URL.
    /// For Etherscan: auto-detected (uses V2 API).
    /// For Blockscout: use the /api endpoint (e.g., https://eth-sepolia.blockscout.com/api).
    /// Required for custom explorer type.
    #[arg(
        long,
        env = "ADI_EXPLORER_API_URL",
        help = "Block explorer API URL.\n\
                Blockscout: https://<instance>/api (NOT /api/eth-rpc or /api/v2)"
    )]
    pub explorer_url: Option<Url>,

    /// Block explorer API key.
    #[arg(long, env = "ADI_EXPLORER_API_KEY", help = "Block explorer API key")]
    pub api_key: Option<String>,

    /// Settlement layer chain ID.
    /// If not provided, will be fetched from RPC.
    #[arg(long, help = "Settlement layer chain ID")]
    pub chain_id: Option<u64>,

    /// Settlement layer JSON-RPC URL (for fetching chain ID).
    #[arg(
        long,
        env = "ADI_RPC_URL",
        help = "Settlement layer JSON-RPC URL (for fetching chain ID)"
    )]
    pub rpc_url: Option<Url>,

    /// Check only specific contract types (comma-separated).
    #[arg(
        long,
        value_delimiter = ',',
        help = "Check only specific contracts (comma-separated)"
    )]
    pub contracts: Option<Vec<String>>,
}

/// Execute the verify command.
pub async fn run(args: VerifyArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Contract Verification")?;

    // Resolve all configuration (returns None for local networks)
    let cfg = match config::resolve_config(&args, context).await? {
        Some(c) => c,
        None => return Ok(()),
    };
    config::display_config(&cfg, &args)?;

    if cfg.targets.is_empty() {
        ui::outro("No contracts found to check.")?;
        return Ok(());
    }

    ui::info(format!("Found {} contracts to check", cfg.targets.len()))?;
    ui::info(ui::dim(
        "Note: Excludes Create2 Factory, Multicall3 (external), L2 contracts, \
         Forge libraries (internal in v30), and contracts unavailable in toolkit \
         (TransparentUpgradeableProxy, some DA validators).",
    ))?;

    // Check verification status
    ui::section("Checking Verification Status")?;

    let (results, counts) =
        match check::check_verification_status(&cfg.targets, &cfg.explorer_client).await? {
            Some(r) => r,
            None => {
                ui::outro_cancel("Verification interrupted by user")?;
                return Ok(());
            }
        };

    check::display_status(&results, &counts)?;

    if counts.errors > 0 {
        ui::outro_cancel(
            "Status checks failed. Please verify the explorer URL and API key are correct.",
        )?;
        return Ok(());
    }

    // Submit or prompt
    if args.submit && counts.unverified > 0 {
        return submit::submit_verifications(&cfg, &args, &results).await;
    }

    if counts.unverified == 0 {
        ui::outro("All contracts are verified!")?;
        return Ok(());
    }

    let do_submit = ui::confirm(format!(
        "Submit {} contracts for verification?",
        counts.unverified
    ))
    .initial_value(false)
    .interact()
    .wrap_err("Failed to read confirmation")?;

    if do_submit {
        return submit::submit_verifications(&cfg, &args, &results).await;
    }

    ui::outro(format!(
        "{} contracts need verification. Use 'adi verify --submit' to skip this prompt.",
        counts.unverified
    ))?;

    Ok(())
}
