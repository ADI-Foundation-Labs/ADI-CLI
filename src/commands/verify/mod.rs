//! Verify command implementation.
//!
//! This command verifies deployed smart contracts on block explorers
//! like Etherscan and Blockscout.

mod contracts;

use adi_ecosystem::verification::{
    parse_diamond_cut_data, ContractRegistry, ExplorerClient, ExplorerConfig, ExplorerType,
    VerificationStatus,
};
use adi_types::{ChainContracts, EcosystemContracts};
use alloy_provider::Provider;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_name, resolve_ecosystem_name,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use contracts::verify_contracts;

/// Result of a verification status check for display purposes.
enum CheckResult {
    Verified,
    NotVerified,
    Pending,
    Unknown(String),
    Error(String),
}

/// Arguments for `verify` command.
///
/// Verifies deployed smart contracts on block explorers.
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

    /// Block explorer type.
    #[arg(
        long,
        value_enum,
        default_value = "etherscan",
        help = "Block explorer type (etherscan, blockscout, custom)"
    )]
    pub explorer: ExplorerType,

    /// Block explorer API URL.
    /// Required for custom explorer type.
    #[arg(
        long,
        env = "ADI_EXPLORER_API_URL",
        help = "Block explorer API URL (required for custom explorer)"
    )]
    pub explorer_url: Option<Url>,

    /// Block explorer API key.
    #[arg(long, env = "ADI_EXPLORER_API_KEY", help = "Block explorer API key")]
    pub api_key: Option<String>,

    /// Protocol version for toolkit image (e.g., v30.0.2).
    #[arg(long, short = 'p', help = "Protocol version for toolkit image")]
    pub protocol_version: Option<String>,

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

    /// Preview contracts without submitting verification.
    #[arg(long, help = "Preview contracts without submitting verification")]
    pub dry_run: bool,

    /// Skip confirmation prompt.
    #[arg(long, short = 'y', help = "Skip confirmation prompt")]
    pub yes: bool,

    /// Force verification even if contracts are already verified.
    #[arg(long, help = "Force verification even if already verified")]
    pub force: bool,

    /// Verify only specific contract types (comma-separated).
    #[arg(
        long,
        value_delimiter = ',',
        help = "Verify only specific contracts (comma-separated)"
    )]
    pub contracts: Option<Vec<String>>,
}

/// Check if an RPC URL points to a local network (Anvil, Hardhat, etc.).
fn is_local_network_url(url: &Url) -> bool {
    let host = url.host_str().unwrap_or("");
    host == "localhost"
        || host == "127.0.0.1"
        || host == "host.docker.internal"
        || host == "0.0.0.0"
        || host.starts_with("192.168.")
        || host.starts_with("10.")
}

/// Execute the verify command.
pub async fn run(args: VerifyArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Verify Contracts")?;

    // Early check for local network - verification not supported
    let rpc_url = args
        .rpc_url
        .as_ref()
        .or(context.config().funding.rpc_url.as_ref());

    if let Some(url) = rpc_url {
        if is_local_network_url(url) {
            ui::outro_cancel(
                "Contract verification is not available for local networks (Anvil, Hardhat, etc.)",
            )?;
            return Ok(());
        }
    }

    // Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    // Create state manager
    let state_manager = create_state_manager_with_context(&ecosystem_name, context);

    // Load ecosystem contracts
    let mut ecosystem_contracts: EcosystemContracts =
        state_manager
            .ecosystem()
            .contracts()
            .await
            .wrap_err("Failed to load ecosystem contracts. Have you deployed the ecosystem?")?;

    // Extract facet addresses from diamond_cut_data if present but not yet extracted
    if let Some(ref mut ctm) = ecosystem_contracts.zksync_os_ctm {
        // Only parse if we have diamond_cut_data but no facet addresses yet
        if ctm.admin_facet_addr.is_none() {
            if let Some(ref diamond_cut_data) = ctm.diamond_cut_data {
                match parse_diamond_cut_data(diamond_cut_data) {
                    Ok(facets) => {
                        context
                            .logger()
                            .debug("Extracted facet addresses from diamond_cut_data");
                        ctm.admin_facet_addr = facets.admin_facet;
                        ctm.executor_facet_addr = facets.executor_facet;
                        ctm.mailbox_facet_addr = facets.mailbox_facet;
                        ctm.getters_facet_addr = facets.getters_facet;
                        ctm.diamond_init_addr = facets.diamond_init;
                    }
                    Err(e) => {
                        context
                            .logger()
                            .warning(&format!("Could not parse diamond_cut_data: {}", e));
                    }
                }
            }
        }
    }

    // Try to load chain contracts if chain name is provided or can be resolved
    let effective_chain_name = args
        .chain
        .clone()
        .or_else(|| resolve_chain_name(args.chain.as_ref(), context.config()).ok());

    let chain_contracts: Option<ChainContracts> = if let Some(ref chain_name) = effective_chain_name
    {
        match state_manager.chain(chain_name).contracts().await {
            Ok(contracts) => Some(contracts),
            Err(e) => {
                context.logger().warning(&format!(
                    "Could not load chain '{}' contracts: {}",
                    chain_name, e
                ));
                None
            }
        }
    } else {
        None
    };

    // Get chain ID
    let chain_id = resolve_chain_id(&args, context).await?;

    // Resolve explorer configuration
    let api_key = resolve_api_key(&args, context)?;
    let explorer_url = resolve_explorer_url(&args, chain_id)?;

    ui::note(
        "Verification configuration",
        format!(
            "Ecosystem: {}\nChain: {}\nExplorer: {}\nAPI URL: {}\nChain ID: {}",
            ui::green(&ecosystem_name),
            effective_chain_name
                .as_ref()
                .map_or_else(|| ui::dim("not specified"), |n| ui::green(n)),
            ui::green(&args.explorer.to_string()),
            ui::green(&explorer_url.to_string()),
            ui::green(chain_id)
        ),
    )?;

    // Build verification targets
    let targets =
        ContractRegistry::build_all_targets(&ecosystem_contracts, chain_contracts.as_ref());

    if targets.is_empty() {
        ui::outro("No contracts found to verify.")?;
        return Ok(());
    }

    ui::info(format!("Found {} contracts to verify", targets.len()))?;

    // Create explorer client for status checks
    let explorer_config = ExplorerConfig::new(
        args.explorer,
        explorer_url.clone(),
        Some(api_key.clone()),
        chain_id,
    );
    let explorer_client = ExplorerClient::new(explorer_config, Arc::clone(context.logger()));

    // Check current verification status
    ui::section("Checking Verification Status")?;

    let mut verified_count = 0;
    let mut unverified_targets = Vec::new();

    let progress = cliclack::progress_bar(targets.len() as u64);
    progress.start("Checking verification status...");

    let mut results: Vec<(String, CheckResult)> = Vec::new();

    for target in &targets {
        let name = target.contract_type.display_name().to_string();

        let result = match explorer_client
            .check_verification_status(target.address)
            .await
        {
            Ok(VerificationStatus::Verified) => {
                verified_count += 1;
                CheckResult::Verified
            }
            Ok(VerificationStatus::NotVerified) => {
                unverified_targets.push(target.clone());
                CheckResult::NotVerified
            }
            Ok(VerificationStatus::Pending) => CheckResult::Pending,
            Ok(VerificationStatus::Unknown(msg)) => {
                unverified_targets.push(target.clone());
                CheckResult::Unknown(msg)
            }
            Err(e) => {
                unverified_targets.push(target.clone());
                CheckResult::Error(e.to_string())
            }
        };

        results.push((name, result));
        progress.inc(1);
        explorer_client.rate_limit_delay().await;
    }

    progress.stop("Verification status check complete");

    // Format results for display
    let results_text = results
        .iter()
        .map(|(name, result)| match result {
            CheckResult::Verified => {
                format!("{}  {} → {}", ui::green("✓"), name, ui::green("Verified"))
            }
            CheckResult::NotVerified => {
                format!(
                    "{}  {} → {}",
                    ui::yellow("✗"),
                    name,
                    ui::yellow("Not Verified")
                )
            }
            CheckResult::Pending => {
                format!("{}  {} → {}", ui::cyan("○"), name, ui::cyan("Pending"))
            }
            CheckResult::Unknown(msg) => {
                format!(
                    "{}  {} → {}",
                    ui::yellow("?"),
                    name,
                    ui::yellow(format!("Unknown: {}", msg))
                )
            }
            CheckResult::Error(msg) => {
                format!(
                    "{}  {} → {}",
                    ui::red("✗"),
                    name,
                    ui::red(format!("Error: {}", msg))
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    ui::note("Verification Status", results_text)?;

    // Summary
    ui::note(
        "Status Summary",
        format!(
            "Verified: {}  Unverified: {}",
            ui::green(verified_count),
            ui::yellow(unverified_targets.len())
        ),
    )?;

    // Early exit if all contracts are verified
    if unverified_targets.is_empty() && !args.force {
        ui::outro("All contracts are already verified!")?;
        return Ok(());
    }

    // If force flag is set, verify all contracts
    let targets_to_verify = if args.force {
        targets
    } else {
        unverified_targets
    };

    if targets_to_verify.is_empty() {
        ui::outro("No contracts need verification.")?;
        return Ok(());
    }

    // Dry-run mode
    if args.dry_run {
        ui::note(
            "Dry Run",
            format!(
                "Would verify {} contracts:\n{}",
                targets_to_verify.len(),
                targets_to_verify
                    .iter()
                    .map(|t| format!("  - {} ({})", t.contract_type.display_name(), t.address))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )?;
        ui::outro("Dry-run mode: no verifications submitted")?;
        return Ok(());
    }

    // Confirmation
    if !args.yes {
        let confirmed = ui::confirm(format!(
            "Proceed with verification of {} contracts?",
            targets_to_verify.len()
        ))
        .initial_value(true)
        .interact()
        .wrap_err("Failed to get confirmation")?;

        if !confirmed {
            ui::outro_cancel("Aborted by user")?;
            return Ok(());
        }
    }

    // Execute verification
    ui::section("Submitting Verifications")?;

    let summary = verify_contracts(
        &targets_to_verify,
        &explorer_url,
        &api_key,
        chain_id,
        &args
            .protocol_version
            .clone()
            .unwrap_or_else(|| "v30.0.2".to_string()),
        context,
    )
    .await;

    // Display final summary
    ui::note(
        "Verification Summary",
        format!(
            "Already Verified: {}  Submitted: {}  Failed: {}",
            ui::green(summary.already_verified_count()),
            ui::cyan(summary.submitted_count()),
            ui::red(summary.failed_count())
        ),
    )?;

    if summary.failed_count() > 0 {
        for result in &summary.results {
            if let adi_ecosystem::verification::VerificationOutcome::Failed { reason } =
                &result.outcome
            {
                context
                    .logger()
                    .error(&format!("{}: {}", result.name, reason));
            }
        }
    }

    if summary.submitted_count() > 0 || summary.already_verified_count() > 0 {
        ui::outro("Verification complete!")?;
        Ok(())
    } else if summary.failed_count() > 0 {
        Err(eyre::eyre!("All verification attempts failed"))
    } else {
        ui::outro("No contracts were verified")?;
        Ok(())
    }
}

/// Resolve chain ID from args or RPC.
async fn resolve_chain_id(args: &VerifyArgs, context: &Context) -> Result<u64> {
    if let Some(chain_id) = args.chain_id {
        return Ok(chain_id);
    }

    // Try to get from RPC
    if let Some(ref rpc_url) = args.rpc_url {
        context.logger().debug("Fetching chain ID from RPC...");
        let provider = alloy_provider::ProviderBuilder::new().connect_http(rpc_url.clone());
        let chain_id = provider
            .get_chain_id()
            .await
            .wrap_err("Failed to get chain ID from RPC")?;
        context.logger().debug(&format!("Chain ID: {}", chain_id));
        return Ok(chain_id);
    }

    // Try from funding config
    if let Some(ref rpc_url) = context.config().funding.rpc_url {
        context
            .logger()
            .debug("Fetching chain ID from config RPC...");
        let provider = alloy_provider::ProviderBuilder::new().connect_http(rpc_url.clone());
        let chain_id = provider
            .get_chain_id()
            .await
            .wrap_err("Failed to get chain ID from config RPC")?;
        return Ok(chain_id);
    }

    Err(eyre::eyre!(
        "Chain ID required. Provide --chain-id or --rpc-url"
    ))
}

/// Resolve API key from args, env, or config.
fn resolve_api_key(args: &VerifyArgs, context: &Context) -> Result<String> {
    if let Some(ref key) = args.api_key {
        return Ok(key.clone());
    }

    if let Some(ref key) = context.config().verification.api_key {
        use secrecy::ExposeSecret;
        return Ok(key.expose_secret().to_string());
    }

    Err(eyre::eyre!(
        "API key required. Provide --api-key or set ADI_EXPLORER_API_KEY"
    ))
}

/// Resolve explorer URL from args or defaults.
fn resolve_explorer_url(args: &VerifyArgs, chain_id: u64) -> Result<Url> {
    if let Some(ref url) = args.explorer_url {
        return Ok(url.clone());
    }

    // Get default URL for known explorers
    if let Some(url) = ExplorerConfig::default_api_url(args.explorer, chain_id) {
        return Ok(url);
    }

    // Custom explorer requires explicit URL
    if args.explorer == ExplorerType::Custom {
        return Err(eyre::eyre!(
            "Explorer URL required for custom explorer. Provide --explorer-url"
        ));
    }

    Err(eyre::eyre!(
        "No default explorer URL for chain ID {}. Provide --explorer-url",
        chain_id
    ))
}
