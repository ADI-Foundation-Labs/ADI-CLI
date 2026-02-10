//! Accept ownership command implementation.
//!
//! This command accepts pending ownership transfers for contracts
//! deployed during ecosystem initialization.

use adi_ecosystem::{accept_all_ownership, OwnershipSummary};
use adi_state::StateManager;
use adi_types::EcosystemContracts;
use clap::Args;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::context::Context;
use crate::error::{Result, WrapErr};

/// Arguments for `accept ownership` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct OwnershipAcceptArgs {
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
}

/// Execute the accept ownership command.
pub async fn run(args: OwnershipAcceptArgs, context: &Context) -> Result<()> {
    log::info!("{}", "Accepting pending ownership transfers...".cyan());

    // Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(&args, context)?;
    log::info!("Ecosystem: {}", ecosystem_name.green());

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(&args, context)?;
    log::info!("RPC URL: {}", rpc_url.to_string().green());

    // Create state manager
    let state_manager = create_state_manager(&ecosystem_name, context)?;

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

    // Display contracts to process
    log::info!("");
    log::info!("{}", "Contracts to process:".cyan());

    if let Some(addr) = ecosystem_contracts.server_notifier_addr() {
        log::info!("  - Server Notifier: {}", addr.to_string().green());
    } else {
        log::info!("  - Server Notifier: {}", "not configured".yellow());
    }

    if let Some(addr) = ecosystem_contracts.validator_timelock_addr() {
        log::info!("  - Validator Timelock: {}", addr.to_string().green());
    } else {
        log::info!("  - Validator Timelock: {}", "not configured".yellow());
    }

    if let Some(addr) = ecosystem_contracts.verifier_addr() {
        log::info!("  - Verifier: {}", addr.to_string().green());
    } else {
        log::info!("  - Verifier: {}", "not configured".yellow());
    }

    log::info!("");

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
            .with_prompt("Proceed with ownership acceptance?")
            .default(true)
            .interact()
            .wrap_err("Failed to get confirmation")?;

        if !confirmed {
            log::info!("Aborted by user");
            return Ok(());
        }
    }

    log::info!("");

    // Execute ownership acceptance
    let summary = accept_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &governor.private_key,
        args.gas_price_wei,
    )
    .await;

    // Display summary
    display_summary(&summary);

    // Return appropriate status
    if summary.has_successes() {
        Ok(())
    } else if summary.results.is_empty() {
        log::warn!("No contracts were processed");
        Ok(())
    } else {
        Err(eyre::eyre!("All ownership acceptances failed"))
    }
}

/// Display the ownership acceptance summary.
fn display_summary(summary: &OwnershipSummary) {
    log::info!("");
    log::info!("{}", "=== Summary ===".cyan());
    log::info!(
        "Successful: {}",
        summary.successful_count().to_string().green()
    );
    log::info!("Failed: {}", summary.failed_count().to_string().yellow());
    log::info!("");

    for result in &summary.results {
        if result.success {
            if let Some(tx) = &result.tx_hash {
                log::info!(
                    "  {} {}: {}",
                    "✓".green(),
                    result.name,
                    tx.to_string().green()
                );
            } else {
                log::info!("  {} {}", "✓".green(), result.name);
            }
        } else if let Some(error) = &result.error {
            log::info!("  {} {}: {}", "✗".yellow(), result.name, error.yellow());
        } else {
            log::info!("  {} {}: unknown error", "✗".yellow(), result.name);
        }
    }
}

/// Resolve ecosystem name from args or config.
fn resolve_ecosystem_name(args: &OwnershipAcceptArgs, context: &Context) -> Result<String> {
    args.ecosystem_name
        .clone()
        .or_else(|| Some(context.config().ecosystem.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem name required: use --ecosystem-name or set in config")
        })
}

/// Resolve RPC URL from args or config.
fn resolve_rpc_url(args: &OwnershipAcceptArgs, context: &Context) -> Result<Url> {
    args.rpc_url
        .clone()
        .or_else(|| context.config().funding.rpc_url.clone())
        .ok_or_else(|| eyre::eyre!("RPC URL required: use --rpc-url or set in config"))
}

/// Create state manager for the ecosystem.
fn create_state_manager(ecosystem_name: &str, context: &Context) -> Result<StateManager> {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    Ok(StateManager::with_backend_type(
        context.config().state_backend.clone(),
        &ecosystem_path,
    ))
}
