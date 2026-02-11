//! Accept ownership command implementation.
//!
//! This command accepts pending ownership transfers for contracts
//! deployed during ecosystem initialization.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, OwnershipState, OwnershipStatusSummary, OwnershipSummary,
};
use adi_state::StateManager;
use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use clap::Args;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::context::Context;
use crate::error::{Result, WrapErr};

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

    // Load and check chain contracts if --chain is provided
    let chain_contracts: Option<ChainContracts>;
    let chain_status: Option<OwnershipStatusSummary>;

    if let Some(ref chain_name) = args.chain {
        match state_manager.chain(chain_name).contracts().await {
            Ok(contracts) => {
                log::info!("");
                log::info!(
                    "{}",
                    format!("Checking chain '{}' ownership status...", chain_name).cyan()
                );
                let status =
                    check_chain_ownership_status(rpc_url.as_str(), &contracts, governor_address)
                        .await
                        .wrap_err("Failed to check chain ownership status")?;

                log::info!("");
                log::info!("{}", format!("Chain '{}' contracts:", chain_name).cyan());
                display_ownership_status(&status);

                chain_contracts = Some(contracts);
                chain_status = Some(status);
            }
            Err(e) => {
                log::warn!("Failed to load chain contracts: {}", e);
                chain_contracts = None;
                chain_status = None;
            }
        }
    } else {
        chain_contracts = None;
        chain_status = None;
    }

    // Show summary of pending contracts
    log::info!("");
    let ecosystem_pending = ecosystem_status.pending_count();
    let chain_pending = chain_status.as_ref().map_or(0, |s| s.pending_count());
    let total_pending = ecosystem_pending + chain_pending;

    if total_pending == 0 {
        log::info!(
            "{}",
            "No contracts have pending ownership transfers.".green()
        );
        return Ok(());
    }

    log::info!(
        "{}",
        format!(
            "{} contract(s) have pending ownership transfers.",
            total_pending
        )
        .yellow()
    );

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

    // Execute ecosystem ownership acceptance
    log::info!("{}", "Processing ecosystem contracts...".cyan());
    let ecosystem_summary = accept_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &governor.private_key,
        args.gas_price_wei,
    )
    .await;

    // Execute chain ownership acceptance if --chain was provided
    let chain_summary = if let Some(contracts) = chain_contracts {
        log::info!("");
        log::info!("{}", "Processing chain contracts...".cyan());
        Some(
            accept_chain_ownership(
                rpc_url.as_str(),
                &contracts,
                &governor.private_key,
                args.gas_price_wei,
            )
            .await,
        )
    } else {
        None
    };

    // Display summaries
    log::info!("");
    log::info!("{}", "=== Ecosystem Summary ===".cyan());
    display_summary(&ecosystem_summary);

    if let Some(ref summary) = chain_summary {
        log::info!("");
        log::info!("{}", "=== Chain Summary ===".cyan());
        display_summary(summary);
    }

    // Return appropriate status
    let total_successes = ecosystem_summary.successful_count()
        + chain_summary.as_ref().map_or(0, |s| s.successful_count());
    let total_results =
        ecosystem_summary.results.len() + chain_summary.as_ref().map_or(0, |s| s.results.len());

    if total_successes > 0 {
        Ok(())
    } else if total_results == 0 {
        log::warn!("No contracts were processed");
        Ok(())
    } else {
        Err(eyre::eyre!("All ownership acceptances failed"))
    }
}

/// Category of ownership result for display purposes.
enum ResultCategory<'a> {
    SuccessWithTx(String),
    SuccessNoTx,
    Skipped(&'a str),
    Failed(&'a str),
}

/// Categorize an ownership result for display.
fn categorize_result(result: &adi_ecosystem::OwnershipResult) -> ResultCategory<'_> {
    if result.success {
        match &result.tx_hash {
            Some(tx) => ResultCategory::SuccessWithTx(tx.to_string()),
            None => ResultCategory::SuccessNoTx,
        }
    } else {
        match &result.error {
            Some(e) if e.starts_with("Skipped: ") => {
                ResultCategory::Skipped(e.strip_prefix("Skipped: ").unwrap_or(e))
            }
            Some(e) => ResultCategory::Failed(e),
            None => ResultCategory::Failed("unknown error"),
        }
    }
}

/// Display the ownership acceptance summary.
fn display_summary(summary: &OwnershipSummary) {
    log::info!(
        "Successful: {}",
        summary.successful_count().to_string().green()
    );
    log::info!("Skipped: {}", summary.skipped_count().to_string().cyan());
    log::info!("Failed: {}", summary.failed_count().to_string().yellow());
    log::info!("");

    for result in &summary.results {
        match categorize_result(result) {
            ResultCategory::SuccessWithTx(tx) => {
                log::info!("  {} {}: {}", "✓".green(), result.name, tx.green());
            }
            ResultCategory::SuccessNoTx => {
                log::info!("  {} {}", "✓".green(), result.name);
            }
            ResultCategory::Skipped(reason) => {
                log::info!("  {} {}: {}", "⊘".cyan(), result.name, reason.cyan());
            }
            ResultCategory::Failed(error) => {
                log::info!("  {} {}: {}", "✗".yellow(), result.name, error.yellow());
            }
        }
    }
}

/// Resolve ecosystem name from args or config.
fn resolve_ecosystem_name(args: &AcceptArgs, context: &Context) -> Result<String> {
    args.ecosystem_name
        .clone()
        .or_else(|| Some(context.config().ecosystem.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem name required: use --ecosystem-name or set in config")
        })
}

/// Resolve RPC URL from args or config.
fn resolve_rpc_url(args: &AcceptArgs, context: &Context) -> Result<Url> {
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

/// Derive address from private key.
fn derive_address_from_key(key: &secrecy::SecretString) -> Result<Address> {
    use alloy_signer_local::PrivateKeySigner;
    use secrecy::ExposeSecret;

    let key_str = key.expose_secret();
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);

    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .wrap_err("Invalid private key hex")?
        .try_into()
        .map_err(|_| eyre::eyre!("Private key must be 32 bytes"))?;

    let signer = PrivateKeySigner::from_bytes(&key_bytes.into()).wrap_err("Invalid private key")?;

    Ok(signer.address())
}

/// Display ownership status for contracts.
fn display_ownership_status(summary: &OwnershipStatusSummary) {
    for status in &summary.statuses {
        match (status.address, status.state) {
            (Some(addr), OwnershipState::Pending) => {
                log::info!(
                    "  {} {}: {} {}",
                    "⏳".yellow(),
                    status.name,
                    addr.to_string().green(),
                    "(pending)".yellow()
                );
            }
            (Some(addr), OwnershipState::Accepted) => {
                log::info!(
                    "  {} {}: {} {}",
                    "✓".green(),
                    status.name,
                    addr.to_string().green(),
                    "(accepted)".green()
                );
            }
            (Some(addr), OwnershipState::NotTransferred) => {
                log::info!(
                    "  {} {}: {} {}",
                    "⚠".red(),
                    status.name,
                    addr.to_string().green(),
                    "(ownership not transferred!)".red()
                );
            }
            (None, _) => {
                log::info!(
                    "  {} {}: {}",
                    "⊘".cyan(),
                    status.name,
                    "not configured".cyan()
                );
            }
        }
    }
}
