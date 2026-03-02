//! Accept ownership command implementation.
//!
//! This command accepts pending ownership transfers for contracts
//! deployed during ecosystem initialization.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, check_ecosystem_ownership_status_for_new_owner,
    collect_all_ownership_calldata, collect_chain_ownership_calldata, OwnershipStatusSummary,
};
use adi_types::{ChainContracts, EcosystemContracts};
use clap::Args;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, derive_address_from_key, display_calldata_output,
    display_ownership_status, display_summary, resolve_chain_name, resolve_ecosystem_name,
    resolve_rpc_url,
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

    // Resolve private key with priority:
    // 1. --private-key argument/env var (new owner mode)
    // 2. Config ownership.private_key (new owner mode)
    // 3. --use-governor flag (use stored governor key)
    // 4. Interactive prompt: "Accept as governor?"
    //
    // Track whether we're in governor mode to determine which contracts to check.
    // Governor mode: check all contracts (post-deploy acceptance)
    // New owner mode: check only directly-owned contracts (post-transfer acceptance)
    let (private_key, key_address, is_governor_mode) = if let Some(ref key_hex) = args.private_key {
        // Priority 1: CLI argument or env var - new owner mode
        let secret = SecretString::from(key_hex.clone());
        let address = derive_address_from_key(&secret)?;
        ui::info(format!(
            "Using provided private key (address: {})",
            ui::green(address)
        ))?;
        (secret, address, false)
    } else if let Some(ref config_key) = context.config().ownership.private_key {
        // Priority 2: Config file - new owner mode
        let address = derive_address_from_key(config_key)?;
        ui::info(format!(
            "Using private key from config (address: {})",
            ui::green(address)
        ))?;
        (config_key.clone(), address, false)
    } else if args.use_governor {
        // Priority 2: --use-governor flag - governor mode
        let ecosystem_wallets = state_manager
            .ecosystem()
            .wallets()
            .await
            .wrap_err("Failed to load ecosystem wallets")?;
        let governor = ecosystem_wallets
            .governor
            .as_ref()
            .ok_or_else(|| eyre::eyre!("Governor wallet not found in ecosystem state"))?;
        let address = derive_address_from_key(&governor.private_key)?;
        ui::info(format!(
            "Using governor key (address: {})",
            ui::green(address)
        ))?;
        (governor.private_key.clone(), address, true)
    } else {
        // Priority 3: Interactive prompt
        let use_governor = ui::confirm("Accept ownership as governor?")
            .initial_value(true)
            .interact()
            .wrap_err("Failed to get confirmation")?;

        if use_governor {
            let ecosystem_wallets = state_manager
                .ecosystem()
                .wallets()
                .await
                .wrap_err("Failed to load ecosystem wallets")?;
            let governor = ecosystem_wallets
                .governor
                .as_ref()
                .ok_or_else(|| eyre::eyre!("Governor wallet not found in ecosystem state"))?;
            let address = derive_address_from_key(&governor.private_key)?;
            (governor.private_key.clone(), address, true)
        } else {
            // Prompt for private key using password input - new owner mode
            let key_hex: String = ui::password("Enter private key (hex):")
                .mask('*')
                .interact()
                .wrap_err("Failed to read private key")?;
            let secret = SecretString::from(key_hex);
            let address = derive_address_from_key(&secret)?;
            ui::info(format!(
                "Using provided key (address: {})",
                ui::green(address)
            ))?;
            (secret, address, false)
        }
    };

    // Check ecosystem ownership status
    // In governor mode: check all contracts (post-deploy acceptance)
    // In new owner mode: check only directly-owned contracts (post-transfer acceptance)
    ui::info("Checking ecosystem ownership status...")?;
    let ecosystem_status = if is_governor_mode {
        check_ecosystem_ownership_status(
            rpc_url.as_str(),
            &ecosystem_contracts,
            key_address,
            context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check ecosystem ownership status")?
    } else {
        check_ecosystem_ownership_status_for_new_owner(
            rpc_url.as_str(),
            &ecosystem_contracts,
            key_address,
            context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check ecosystem ownership status")?
    };

    // Display ecosystem contracts with pending status
    display_ownership_status("Ecosystem contracts", &ecosystem_status)?;

    // In new owner mode, automatically include chain contracts from config
    // This ensures Chain Admin is accepted without requiring --chain flag
    let effective_chain_name = if !is_governor_mode && args.chain.is_none() {
        // Try to resolve chain from config, but don't error if not set
        resolve_chain_name(args.chain.as_ref(), context.config()).ok()
    } else {
        args.chain.clone()
    };

    // Load and check chain contracts if chain name is available
    let chain_contracts: Option<ChainContracts>;
    let chain_status: Option<OwnershipStatusSummary>;

    if let Some(ref chain_name) = effective_chain_name {
        match state_manager.chain(chain_name).contracts().await {
            Ok(contracts) => {
                ui::info(format!(
                    "Checking chain '{}' ownership status...",
                    chain_name
                ))?;
                let status = check_chain_ownership_status(
                    rpc_url.as_str(),
                    &contracts,
                    key_address,
                    context.logger().as_ref(),
                )
                .await
                .wrap_err("Failed to check chain ownership status")?;

                display_ownership_status(&format!("Chain '{}' contracts", chain_name), &status)?;

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

    // Calldata mode - collect and display calldata without sending
    if args.calldata {
        ui::info("Collecting calldata for pending contracts...")?;

        let ecosystem_calldata = collect_all_ownership_calldata(
            rpc_url.as_str(),
            &ecosystem_contracts,
            key_address,
            context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to collect ecosystem calldata")?;

        display_calldata_output("Ecosystem Calldata", &ecosystem_calldata)?;

        if let Some(ref contracts) = chain_contracts {
            let chain_calldata = collect_chain_ownership_calldata(
                rpc_url.as_str(),
                contracts,
                key_address,
                context.logger().as_ref(),
            )
            .await
            .wrap_err("Failed to collect chain calldata")?;

            display_calldata_output("Chain Calldata", &chain_calldata)?;
        }

        ui::outro("Calldata collection complete")?;
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

    // Resolve gas multiplier (use config default if not provided)
    let gas_multiplier = args
        .gas_multiplier
        .or(Some(context.config().gas_multiplier));

    // Execute ecosystem ownership acceptance
    ui::info("Processing ecosystem contracts...")?;
    let ecosystem_summary = accept_all_ownership(
        rpc_url.as_str(),
        &ecosystem_contracts,
        &private_key,
        gas_multiplier,
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
                &private_key,
                gas_multiplier,
                context.logger().as_ref(),
            )
            .await,
        )
    } else {
        None
    };

    // Display summaries
    display_summary("Ecosystem Summary", &ecosystem_summary)?;

    if let Some(ref summary) = chain_summary {
        display_summary("Chain Summary", summary)?;
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
