//! Verify command implementation.
//!
//! This command checks the verification status of deployed smart contracts
//! on block explorers like Etherscan and Blockscout, and can submit
//! unverified contracts for verification.

use adi_ecosystem::verification::{
    apply_implementations, encode_chain_admin_constructor_args,
    encode_era_verifier_constructor_args, encode_proxy_constructor_args,
    encode_verifier_constructor_args, parse_diamond_cut_data, read_all_implementations, read_owner,
    ContractRegistry, ExplorerClient, ExplorerConfig, ExplorerType, VerificationOutcome,
    VerificationResult, VerificationStatus, VerificationSummary, VerificationTarget,
};
use adi_toolkit::{ProtocolVersion, ToolkitRunner};
use adi_types::{ChainContracts, EcosystemContracts};
use alloy_provider::Provider;
use clap::Args;
use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_name, resolve_ecosystem_name,
    resolve_protocol_version,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Maximum concurrent verification status checks.
const MAX_CONCURRENT_CHECKS: usize = 5;

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

/// Get RPC URL from args or config.
fn get_rpc_url(args: &VerifyArgs, context: &Context) -> Option<Url> {
    args.rpc_url
        .clone()
        .or_else(|| context.config().ecosystem.rpc_url.clone())
        .or_else(|| context.config().funding.rpc_url.clone())
}

/// Execute the verify command.
pub async fn run(args: VerifyArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Contract Verification")?;

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

    let mut chain_contracts: Option<ChainContracts> =
        if let Some(ref chain_name) = effective_chain_name {
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

    // Read implementation addresses from RPC if available (for constructor args)
    let rpc_url = get_rpc_url(&args, context);
    if let Some(ref url) = rpc_url {
        if !is_local_network_url(url) {
            let spinner = cliclack::spinner();
            spinner.start("Reading contract implementations from RPC...");

            let provider = alloy_provider::ProviderBuilder::new().connect_http(url.clone());
            let impls =
                read_all_implementations(&provider, &ecosystem_contracts, context.logger().clone())
                    .await;
            apply_implementations(&mut ecosystem_contracts, &impls);

            // Also read chain-level ChainAdmin owner if chain contracts available
            if let Some(ref mut chain) = chain_contracts.as_mut() {
                if let Some(ref mut l1) = chain.l1.as_mut() {
                    if let Some(chain_admin_addr) = l1.chain_admin_addr {
                        if let Some(owner) = read_owner(&provider, chain_admin_addr).await {
                            l1.chain_admin_owner = Some(owner);
                        }
                    }
                }
            }

            spinner.stop("Contract implementations loaded");
        }
    }

    // Get chain ID
    let chain_id = resolve_chain_id(&args, context).await?;

    // Resolve explorer configuration
    let explorer_type = resolve_explorer_type(&args, context);
    let api_key = resolve_api_key(&args, context);
    let explorer_url = resolve_explorer_url(&args, explorer_type, chain_id, context)?;

    ui::note(
        "Verification configuration",
        format!(
            "Ecosystem: {}\nChain: {}\nExplorer: {}\nAPI URL: {}\nChain ID: {}\nMode: {}",
            ui::green(&ecosystem_name),
            effective_chain_name
                .as_ref()
                .map_or_else(|| ui::dim("not specified"), |n| ui::green(n)),
            ui::green(&explorer_type.to_string()),
            ui::green(&explorer_url.to_string()),
            ui::green(chain_id),
            if args.submit {
                ui::cyan("submit")
            } else {
                ui::dim("status check")
            }
        ),
    )?;

    // Build verification targets based on flags
    let targets = build_targets(&args, &ecosystem_contracts, chain_contracts.as_ref())?;

    if targets.is_empty() {
        ui::outro("No contracts found to check.")?;
        return Ok(());
    }

    ui::info(format!("Found {} contracts to check", targets.len()))?;
    ui::info(ui::dim(
        "Note: Excludes Create2 Factory, Multicall3 (external), L2 contracts, \
         Forge libraries (internal in v30), and contracts unavailable in toolkit \
         (TransparentUpgradeableProxy, some DA validators).",
    ))?;

    // Create explorer client for status checks
    let explorer_config = ExplorerConfig::new(
        explorer_type,
        explorer_url.clone(),
        api_key.clone(),
        chain_id,
    );
    let explorer_client = ExplorerClient::new(explorer_config, Arc::clone(context.logger()))
        .map_err(|e| eyre::eyre!("Failed to create explorer client: {}", e))?;

    // Check current verification status
    ui::section("Checking Verification Status")?;

    let mut verified_count = 0;
    let mut unverified_count = 0;
    let mut error_count = 0;

    let progress = cliclack::progress_bar(targets.len() as u64);
    progress.start("Checking verification status...");

    let mut results: Vec<(String, CheckResult)> = Vec::new();
    let mut interrupted = false;

    // Wrap explorer client in Arc for shared access across concurrent tasks
    let explorer_client = Arc::new(explorer_client);

    // Create indexed futures for all targets
    let check_futures = targets.iter().enumerate().map(|(idx, target)| {
        let client = Arc::clone(&explorer_client);
        let name = target.contract_type.display_name().to_string();
        let address = target.address;

        async move {
            let api_result = client.check_verification_status(address).await;
            let result = match api_result {
                Ok(VerificationStatus::Verified) => CheckResult::Verified,
                Ok(VerificationStatus::NotVerified) => CheckResult::NotVerified,
                Ok(VerificationStatus::Pending) => CheckResult::Pending,
                Ok(VerificationStatus::Unknown(msg)) => CheckResult::Unknown(msg),
                Err(e) => CheckResult::Error(e.to_string()),
            };
            (idx, name, result)
        }
    });

    // Process with bounded concurrency
    let mut check_stream = stream::iter(check_futures).buffer_unordered(MAX_CONCURRENT_CHECKS);

    let mut indexed_results: Vec<(usize, String, CheckResult)> = Vec::with_capacity(targets.len());

    loop {
        tokio::select! {
            biased;

            _ = tokio::signal::ctrl_c() => {
                interrupted = true;
                progress.stop("Interrupted by user");
                break;
            }

            result = check_stream.next() => {
                match result {
                    Some((idx, name, check_result)) => {
                        indexed_results.push((idx, name, check_result));
                        progress.inc(1);
                    }
                    None => break, // Stream exhausted
                }
            }
        }
    }

    if !interrupted {
        progress.stop("Verification status check complete");
    }

    // Sort by original index for consistent display ordering
    indexed_results.sort_by_key(|(idx, _, _)| *idx);

    // Count results and convert to final format
    for (_, name, result) in indexed_results {
        match &result {
            CheckResult::Verified => verified_count += 1,
            CheckResult::NotVerified | CheckResult::Unknown(_) => unverified_count += 1,
            CheckResult::Error(_) => error_count += 1,
            CheckResult::Pending => {}
        }
        results.push((name, result));
    }

    // Exit early if interrupted
    if interrupted {
        ui::outro_cancel("Verification interrupted by user")?;
        return Ok(());
    }

    // Format results for display
    let results_text = format_check_results(&results);
    ui::note("Verification Status", results_text)?;

    // Summary
    ui::note(
        "Status Summary",
        format!(
            "Verified: {}  Unverified: {}  Errors: {}",
            ui::green(verified_count),
            ui::yellow(unverified_count),
            if error_count > 0 {
                ui::red(error_count).to_string()
            } else {
                ui::dim("0").to_string()
            }
        ),
    )?;

    // Exit early if any checks failed with errors - indicates API misconfiguration
    if error_count > 0 {
        ui::outro_cancel(
            "Status checks failed. Please verify the explorer URL and API key are correct.",
        )?;
        return Ok(());
    }

    // If --submit flag is set and there are unverified contracts
    if args.submit && unverified_count > 0 {
        return submit_verifications(
            &args,
            &targets,
            &results,
            Arc::clone(&explorer_client),
            context,
        )
        .await;
    }

    // Final message based on status
    if unverified_count == 0 {
        ui::outro("All contracts are verified!")?;
    } else {
        // Interactive prompt to submit verifications
        let submit = ui::confirm(format!(
            "Submit {} contracts for verification?",
            unverified_count
        ))
        .initial_value(false)
        .interact()
        .wrap_err("Failed to read confirmation")?;

        if submit {
            return submit_verifications(
                &args,
                &targets,
                &results,
                Arc::clone(&explorer_client),
                context,
            )
            .await;
        }

        ui::outro(format!(
            "{} contracts need verification. Use 'adi verify --submit' to skip this prompt.",
            unverified_count
        ))?;
    }

    Ok(())
}

/// Resolve chain ID from args or RPC.
async fn resolve_chain_id(args: &VerifyArgs, context: &Context) -> Result<u64> {
    if let Some(chain_id) = args.chain_id {
        return Ok(chain_id);
    }

    // Try to get from RPC (CLI arg)
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

    // Try from ecosystem config
    if let Some(ref rpc_url) = context.config().ecosystem.rpc_url {
        context
            .logger()
            .debug("Fetching chain ID from ecosystem config RPC...");
        let provider = alloy_provider::ProviderBuilder::new().connect_http(rpc_url.clone());
        let chain_id = provider
            .get_chain_id()
            .await
            .wrap_err("Failed to get chain ID from ecosystem config RPC")?;
        return Ok(chain_id);
    }

    // Try from funding config (backward compatibility)
    if let Some(ref rpc_url) = context.config().funding.rpc_url {
        context
            .logger()
            .debug("Fetching chain ID from funding config RPC...");
        let provider = alloy_provider::ProviderBuilder::new().connect_http(rpc_url.clone());
        let chain_id = provider
            .get_chain_id()
            .await
            .wrap_err("Failed to get chain ID from funding config RPC")?;
        return Ok(chain_id);
    }

    Err(eyre::eyre!(
        "Chain ID required. Provide --chain-id or --rpc-url"
    ))
}

/// Resolve API key from args, env, or config.
/// Returns None if no API key is provided (optional for public explorers).
fn resolve_api_key(args: &VerifyArgs, context: &Context) -> Option<String> {
    if let Some(ref key) = args.api_key {
        return Some(key.clone());
    }

    if let Some(ref key) = context.config().verification.api_key {
        use secrecy::ExposeSecret;
        return Some(key.expose_secret().to_string());
    }

    None
}

/// Resolve explorer type from args or config.
fn resolve_explorer_type(args: &VerifyArgs, context: &Context) -> ExplorerType {
    // Config takes priority over clap default (etherscan)
    // But explicit CLI arg would have been parsed and is in args.explorer
    if let Some(ref explorer_str) = context.config().verification.explorer {
        if let Ok(explorer_type) = explorer_str.parse::<ExplorerType>() {
            return explorer_type;
        }
    }
    args.explorer
}

/// Resolve explorer URL from args, config, or defaults.
fn resolve_explorer_url(
    args: &VerifyArgs,
    explorer_type: ExplorerType,
    chain_id: u64,
    context: &Context,
) -> Result<Url> {
    // 1. CLI arg takes priority
    if let Some(ref url) = args.explorer_url {
        let url_str = url.as_str();

        // Validate Blockscout URLs to catch common mistakes
        if explorer_type == ExplorerType::Blockscout {
            if url_str.contains("/api/eth-rpc") {
                return Err(eyre::eyre!(
                    "Invalid Blockscout URL: '/api/eth-rpc' is the JSON-RPC endpoint.\n\
                     For contract verification, use the REST API endpoint instead.\n\
                     Example: https://eth-sepolia.blockscout.com/api"
                ));
            }
            if url_str.contains("/api/v2") {
                return Err(eyre::eyre!(
                    "Invalid Blockscout URL: '/api/v2' is the native REST API.\n\
                     For contract verification, use the Etherscan-compatible endpoint.\n\
                     Example: https://eth-sepolia.blockscout.com/api"
                ));
            }
        }

        return Ok(url.clone());
    }

    // 2. Fall back to config
    if let Some(ref url) = context.config().verification.explorer_url {
        return Ok(url.clone());
    }

    // 3. Get default URL for known explorers
    if let Some(url) = ExplorerConfig::default_api_url(explorer_type, chain_id) {
        return Ok(url);
    }

    // Custom explorer requires explicit URL
    if explorer_type == ExplorerType::Custom {
        return Err(eyre::eyre!(
            "Explorer URL required for custom explorer. Provide --explorer-url"
        ));
    }

    Err(eyre::eyre!(
        "No default explorer URL for chain ID {}. Provide --explorer-url",
        chain_id
    ))
}

/// Build verification targets based on command flags.
fn build_targets(
    args: &VerifyArgs,
    ecosystem_contracts: &EcosystemContracts,
    chain_contracts: Option<&ChainContracts>,
) -> Result<Vec<VerificationTarget>> {
    let mut targets = Vec::new();

    // Add ecosystem targets if --ecosystem flag is set or if no chain is specified
    if args.ecosystem || args.chain.is_none() {
        targets.extend(ContractRegistry::build_ecosystem_targets(
            ecosystem_contracts,
        ));
    }

    // Add chain targets if chain contracts are available
    if let Some(chain) = chain_contracts {
        targets.extend(ContractRegistry::build_chain_targets(chain));
    }

    Ok(targets)
}

/// Format check results for display.
fn format_check_results(results: &[(String, CheckResult)]) -> String {
    results
        .iter()
        .map(|(name, result)| match result {
            CheckResult::Verified => {
                format!("{}  {} -> {}", ui::green("✓"), name, ui::green("Verified"))
            }
            CheckResult::NotVerified => {
                format!(
                    "{}  {} -> {}",
                    ui::yellow("✗"),
                    name,
                    ui::yellow("Not Verified")
                )
            }
            CheckResult::Pending => {
                format!("{}  {} -> {}", ui::cyan("○"), name, ui::cyan("Pending"))
            }
            CheckResult::Unknown(msg) => {
                format!(
                    "{}  {} -> {}",
                    ui::yellow("?"),
                    name,
                    ui::yellow(format!("Unknown: {}", msg))
                )
            }
            CheckResult::Error(msg) => {
                format!(
                    "{}  {} -> {}",
                    ui::red("✗"),
                    name,
                    ui::red(format!("Error: {}", msg))
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Submit verifications for unverified contracts.
async fn submit_verifications(
    args: &VerifyArgs,
    targets: &[VerificationTarget],
    status_results: &[(String, CheckResult)],
    explorer_client: Arc<ExplorerClient>,
    context: &Context,
) -> Result<()> {
    // Filter to only unverified targets
    let unverified_targets: Vec<_> = targets
        .iter()
        .filter(|t| {
            let name = t.contract_type.display_name();
            status_results
                .iter()
                .any(|(n, r)| n == name && !matches!(r, CheckResult::Verified))
        })
        .cloned()
        .collect();

    if unverified_targets.is_empty() {
        ui::outro("No contracts need verification.")?;
        return Ok(());
    }

    // Dry run mode - just show the plan
    if args.dry_run {
        display_verification_plan(&unverified_targets)?;
        ui::outro("Dry-run mode: verification plan displayed")?;
        return Ok(());
    }

    // Require protocol version for verification
    let protocol_version_str =
        resolve_protocol_version(args.protocol_version.as_ref(), context.config())?;
    let protocol_version = ProtocolVersion::parse(&protocol_version_str)
        .map_err(|e| eyre::eyre!("Invalid protocol version: {}", e))?;

    ui::section("Submitting Verifications")?;
    ui::info(format!(
        "Submitting {} contracts for verification...",
        unverified_targets.len()
    ))?;

    // Create toolkit runner
    let runner = ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;

    // state_dir is passed as log_dir; stream.rs adds "logs" subdirectory
    let log_dir = context.config().state_dir.clone();

    // Ensure logs subdirectory exists (stream.rs will create files under logs/)
    tokio::fs::create_dir_all(log_dir.join("logs"))
        .await
        .wrap_err("Failed to create log directory")?;

    // Verify each contract
    let mut results = Vec::new();
    let progress = cliclack::progress_bar(unverified_targets.len() as u64);
    progress.start("Submitting verifications...");

    for target in &unverified_targets {
        let name = target.contract_type.display_name();
        let address = target.address;

        // Update progress bar label with current contract
        progress.start(format!("Verifying {}...", name));

        // Compute constructor args based on contract type
        let constructor_args = target
            .proxy_info
            .as_ref()
            .map(|info| {
                encode_proxy_constructor_args(
                    info.impl_addr,
                    info.proxy_admin_addr,
                    &info.init_data,
                )
            })
            .or_else(|| {
                target.verifier_info.as_ref().map(|info| {
                    if let Some(owner) = info.owner_addr {
                        // ZKsyncOSDualVerifier: (fflonk, plonk, owner)
                        encode_verifier_constructor_args(info.fflonk_addr, info.plonk_addr, owner)
                    } else {
                        // EraDualVerifier: (fflonk, plonk)
                        encode_era_verifier_constructor_args(info.fflonk_addr, info.plonk_addr)
                    }
                })
            })
            .or_else(|| {
                target.chain_admin_info.as_ref().map(|info| {
                    encode_chain_admin_constructor_args(
                        info.owner_addr,
                        info.token_multiplier_setter,
                    )
                })
            });

        let exit_code = runner
            .run_forge_verify(
                &format!("{:?}", address),
                &target.forge_contract_path(),
                explorer_client.config().chain_id,
                explorer_client.config().api_url.as_str(),
                explorer_client.config().explorer_type.forge_verifier_name(),
                explorer_client.config().api_key.as_deref(),
                constructor_args.as_deref(),
                &protocol_version.to_semver(),
                &log_dir,
                target.root_path,
            )
            .await;

        let result = match exit_code {
            Ok(0) => VerificationResult::submitted(name, address, "submitted".to_string()),
            Ok(code) => VerificationResult::failed(name, address, format!("Exit code: {}", code)),
            Err(e) => VerificationResult::failed(name, address, e.to_string()),
        };

        let is_failure = matches!(result.outcome, VerificationOutcome::Failed { .. });
        results.push(result);
        progress.inc(1);

        if is_failure && !args.continue_on_error {
            progress.stop("Stopped due to failure");
            context.logger().warning(
                "Stopping verification due to failure (use --continue-on-error to continue)",
            );
            break;
        }

        // Rate limiting between requests
        explorer_client.rate_limit_delay().await;
    }

    progress.stop("Verification submission complete");

    let summary = VerificationSummary::new(results);

    // Display summary
    ui::note(
        "Verification Summary",
        format!(
            "Submitted: {}  Already verified: {}  Skipped: {}  Failed: {}",
            ui::green(summary.submitted_count()),
            ui::cyan(summary.already_verified_count()),
            ui::yellow(summary.skipped_count()),
            ui::red(summary.failed_count())
        ),
    )?;

    if summary.failed_count() > 0 {
        ui::outro_cancel(format!(
            "{} contracts failed verification. Check logs in {}",
            summary.failed_count(),
            log_dir.display()
        ))?;
    } else {
        ui::outro("Verification submission complete!")?;
    }

    Ok(())
}

/// Display verification plan (dry-run mode).
fn display_verification_plan(targets: &[VerificationTarget]) -> Result<()> {
    let lines: Vec<String> = targets
        .iter()
        .map(|t| format!("  {} -> {:?}", t.contract_type.display_name(), t.address))
        .collect();
    ui::note("Contracts to verify", lines.join("\n"))?;
    Ok(())
}
