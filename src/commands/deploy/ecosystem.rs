//! Ecosystem deployment command implementation.
//!
//! This command:
//! 1. Funds ecosystem and chain wallets
//! 2. Deploys ecosystem contracts via zkstack
//! 3. Configures validator roles for operators

use adi_ecosystem::verification::{
    apply_implementations, parse_diamond_cut_data, read_all_implementations, ExplorerType,
};
use adi_ecosystem::{add_validator_roles, configure_l3_da, validate_chain_id, DeployedContracts};
use adi_funding::{
    build_funding_target_statuses, is_localhost_rpc, normalize_rpc_url, AnvilFunder,
    AnvilFundingTarget, DefaultAmounts, FundingConfig, FundingError, FundingExecutor,
    FundingPlanBuilder, FundingTargetStatus, LoggingEventHandler, SpinnerEventHandler,
};
use adi_state::StateManager;
use adi_toolkit::{ProtocolVersion, ToolkitRunner, VerificationOpts, GENESIS_FILENAME};
use adi_types::Wallets;
use alloy_primitives::{Address, U256};
use clap::Args;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_s3, resolve_protocol_version, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for `deploy` command.
///
/// Funds ecosystem wallets and deploys core infrastructure contracts.
/// Requires initialized ecosystem (run `adi init` first).
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct DeployArgs {
    /// Ecosystem name (falls back to config file if not provided)
    #[arg(
        long,
        help = "Ecosystem name (falls back to config file if not provided)"
    )]
    pub ecosystem_name: Option<String>,

    /// Chain name for wallet funding (falls back to config file if not provided)
    #[arg(
        long,
        help = "Chain name for wallet funding (falls back to config file if not provided)"
    )]
    pub chain_name: Option<String>,

    /// Settlement layer JSON-RPC URL (e.g., http://localhost:8545 or https://sepolia.infura.io/v3/KEY)
    #[arg(
        long,
        env = "ADI_RPC_URL",
        help = "Settlement layer JSON-RPC URL (e.g., http://localhost:8545)"
    )]
    pub rpc_url: Option<Url>,

    /// Funder wallet private key (hex). Prefer config file or env var for security
    #[arg(
        long,
        env = "ADI_FUNDER_KEY",
        help = "Funder wallet private key (hex). Prefer config file or env var for security"
    )]
    pub funder_key: Option<String>,

    /// Gas price multiplier percentage (default: 120 = 20% buffer over estimated gas)
    #[arg(
        long,
        help = "Gas price multiplier percentage (default: 120 = 20% buffer over estimated gas)"
    )]
    pub gas_multiplier: Option<u64>,

    /// Deployer wallet ETH amount in ether (default: 1.0)
    #[arg(long, help = "Deployer wallet ETH amount in ether (default: 1.0)")]
    pub deployer_eth: Option<f64>,

    /// Governor wallet ETH amount in ether (default: 1.0)
    #[arg(long, help = "Governor wallet ETH amount in ether (default: 1.0)")]
    pub governor_eth: Option<f64>,

    /// Governor custom gas token (CGT) amount. Only for chains with custom base token (default: 5.0)
    #[arg(
        long,
        help = "Governor custom gas token (CGT) amount. Only for chains with custom base token (default: 5.0)"
    )]
    pub governor_cgt_units: Option<f64>,

    /// Operator wallet ETH amount in ether (default: 5.0)
    #[arg(long, help = "Operator wallet ETH amount in ether (default: 5.0)")]
    pub operator_eth: Option<f64>,

    /// Prove operator wallet ETH (submits validity proofs to L1, default: 5.0)
    #[arg(
        long,
        help = "Prove operator wallet ETH (submits validity proofs to L1, default: 5.0)"
    )]
    pub prove_operator_eth: Option<f64>,

    /// Execute operator wallet ETH (executes batches on L1, default: 5.0)
    #[arg(
        long,
        help = "Execute operator wallet ETH (executes batches on L1, default: 5.0)"
    )]
    pub execute_operator_eth: Option<f64>,

    /// Skip wallet funding step (use if wallets are already funded)
    #[arg(
        long,
        help = "Skip wallet funding step (use if wallets are already funded)"
    )]
    pub skip_funding: bool,

    /// Preview funding plan without executing transactions
    #[arg(long, help = "Preview funding plan without executing transactions")]
    pub dry_run: bool,

    /// Skip confirmation prompt (for automation/scripting)
    #[arg(
        long,
        short = 'y',
        help = "Skip confirmation prompt (for automation/scripting)"
    )]
    pub yes: bool,

    /// Skip contract deployment step (only fund wallets)
    #[arg(long, help = "Skip contract deployment step (only fund wallets)")]
    pub skip_deployment: bool,

    /// Protocol version for toolkit image (e.g., v30.0.2). Required for deployment
    #[arg(
        long,
        short = 'p',
        help = "Protocol version for toolkit image (e.g., v30.0.2)"
    )]
    pub protocol_version: Option<String>,

    /// Deploy as L3 chain (disables blobs, uses calldata DA)
    #[arg(
        long,
        help = "Deploy as L3 chain (disables blobs, uses calldata DA on L2 settlement layer)"
    )]
    pub l3: bool,

    /// Enable contract verification during deployment.
    #[arg(long, help = "Enable contract verification during deployment")]
    pub verify: bool,

    /// Disable contract verification (overrides --verify and config).
    #[arg(
        long,
        conflicts_with = "verify",
        help = "Disable contract verification"
    )]
    pub no_verify: bool,

    /// Block explorer type for verification (etherscan, blockscout, custom).
    #[arg(long, value_enum, help = "Block explorer type")]
    pub explorer: Option<ExplorerType>,

    /// Block explorer API URL for verification.
    #[arg(long, env = "ADI_EXPLORER_URL", help = "Block explorer API URL")]
    pub explorer_url: Option<Url>,

    /// Block explorer API key for verification.
    #[arg(long, env = "ADI_EXPLORER_API_KEY", help = "Block explorer API key")]
    pub explorer_api_key: Option<String>,
}

/// Execute the ecosystem deploy command.
///
/// This command:
/// 1. Validates ecosystem and chain exist in state
/// 2. Resolves RPC URL and funder key from args/config
/// 3. Loads ecosystem and chain wallets from state
/// 4. Displays current wallet balances
/// 5. Builds funding plan (checks balances, calculates transfers)
/// 6. Prompts for confirmation (unless --yes)
/// 7. Executes funding transfers
/// 8. (Future) Deploys ecosystem contracts
pub async fn run(args: DeployArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Deploy")?;
    context.logger().debug("Starting ecosystem deployment");

    // 1. Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(&args, context)?;

    // 2. Resolve chain name
    let chain_name = resolve_chain_name(&args, context)?;

    // 3. Create state manager and validate ecosystem exists
    let state_manager = create_state_manager(&ecosystem_name, context).await?;
    validate_ecosystem_exists(&state_manager, &ecosystem_name).await?;

    // 4. Validate chain exists
    validate_chain_exists(&state_manager, &chain_name, &ecosystem_name).await?;

    // 5. Skip funding if requested
    if args.skip_funding {
        ui::info("Skipping wallet funding (--skip-funding)")?;
        ui::outro("Ecosystem deployment complete (funding skipped)")?;
        return Ok(());
    }

    // 6. Resolve RPC URL (args > ecosystem.rpc_url > funding.rpc_url)
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    // 7. Validate chain ID doesn't conflict with settlement layer
    ui::info("Validating chain ID against settlement layer...")?;
    let chain_metadata = state_manager
        .chain(&chain_name)
        .metadata()
        .await
        .wrap_err("Failed to load chain metadata")?;

    let normalized_rpc = normalize_rpc_url(rpc_url.as_str());
    let validation_provider = adi_funding::FundingProvider::new(&normalized_rpc)
        .wrap_err("Failed to connect to settlement layer for validation")?;
    let settlement_chain_id = validation_provider
        .get_chain_id()
        .await
        .wrap_err("Failed to get settlement layer chain ID")?;

    if let Err(msg) = validate_chain_id(chain_metadata.chain_id, settlement_chain_id) {
        return Err(eyre::eyre!("{}", msg));
    }
    ui::success(format!(
        "Chain ID {} validated (settlement layer: {})",
        chain_metadata.chain_id, settlement_chain_id
    ))?;

    // Display deployment info
    let is_l3 = resolve_l3_mode(&args, context);
    let chain_type = if is_l3 { "L3" } else { "L2" };
    ui::note(
        "Deployment target",
        format!(
            "Ecosystem: {}\nChain: {} ({})\nSettlement layer RPC: {}",
            ui::green(&ecosystem_name),
            ui::green(&chain_name),
            ui::green(chain_type),
            ui::green(&rpc_url)
        ),
    )?;

    // 7. Check for Anvil mode (localhost RPC + no custom funder key)
    let is_anvil = is_localhost_rpc(rpc_url.as_str()) && args.funder_key.is_none();
    if is_anvil {
        return run_anvil_funding(
            &args,
            context,
            &state_manager,
            &ecosystem_name,
            &chain_name,
            &rpc_url,
        )
        .await;
    }

    // 8. Get funder key (args > config) - required for production mode
    let funder_key = resolve_funder_key(&args, context)?;
    context.logger().debug("Funder key resolved");

    // 8. Load wallets from state
    let ecosystem_wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;

    let chain_wallets = state_manager
        .chain(&chain_name)
        .wallets()
        .await
        .wrap_err_with(|| format!("Failed to load chain '{}' wallets", chain_name))?;

    ui::info(format!(
        "Loaded wallets: ecosystem={}, chain={}",
        ui::green(count_wallets(&ecosystem_wallets)),
        ui::green(count_wallets(&chain_wallets))
    ))?;

    // 9. Create executor with spinner handler for visual progress
    let executor = FundingExecutor::new(rpc_url.as_str(), &funder_key)
        .wrap_err("Failed to create funding executor")?
        .with_event_handler(Arc::new(SpinnerEventHandler::new()));

    let funder_address = executor.funder_address();
    ui::info(format!("Funder address: {}", ui::green(funder_address)))?;

    // 10. Build funding config (reads base_token from chain metadata)
    let funding_config = build_funding_config(
        &args,
        context,
        &rpc_url,
        &state_manager,
        &chain_name,
        executor.provider(),
    )
    .await?;

    // 11. Display funding plan with current balances (unified Anvil-style display)
    let spinner = cliclack::spinner();
    spinner.start("Checking wallet balances...");
    let target_statuses = build_funding_target_statuses(
        executor.provider(),
        &funding_config,
        &ecosystem_wallets,
        &chain_wallets,
    )
    .await
    .wrap_err("Failed to get funding target statuses")?;
    spinner.stop("Wallet balances checked");

    display_funding_plan(
        funder_address,
        &target_statuses,
        funding_config.token_symbol.as_deref(),
    )?;

    // 12. Build funding plan
    let spinner = cliclack::spinner();
    spinner.start("Building funding plan...");
    let plan_result = FundingPlanBuilder::new(executor.provider(), &funding_config, funder_address)
        .with_ecosystem_wallets(&ecosystem_wallets)
        .with_chain_wallets(&chain_wallets)
        .build()
        .await;
    spinner.stop("Funding plan ready");

    let plan = match plan_result {
        Ok(p) => Some(p),
        Err(FundingError::NoFundingRequired) => {
            ui::success("All wallets already funded - no funding required!")?;
            None
        }
        Err(e) => return Err(e).wrap_err("Failed to build funding plan"),
    };

    // 13-16. Funding plan display, confirmation, and execution (if needed)
    if let Some(plan) = plan {
        // Display funding summary in boxed note (matches Anvil style)
        display_funding_summary(&plan, &funding_config)?;

        if !plan.is_valid() {
            let needed = plan.total_eth_required;
            let have = plan.funder_eth_balance;
            return Err(eyre::eyre!(
                "Funder has insufficient balance.\n  Have: {} ETH\n  Need: {} ETH\n  \
                 Please fund the funder wallet with at least {} ETH",
                format_eth(have),
                format_eth(needed),
                format_eth(needed - have)
            ));
        }

        // Dry-run mode - show plan without executing
        if args.dry_run {
            display_plan_details(&plan)?;
            ui::outro("Dry-run mode: funding plan created but not executed")?;
            return Ok(());
        }

        // Confirmation prompt (unless --yes)
        if !args.yes {
            let confirmed = ui::confirm("Proceed with funding?")
                .initial_value(false)
                .interact()
                .wrap_err("Failed to read confirmation")?;

            if !confirmed {
                ui::outro_cancel("Funding cancelled by user")?;
                return Ok(());
            }
        }

        // Execute funding with spinner progress
        ui::section("Executing Transfers")?;
        let result = executor
            .execute(&plan)
            .await
            .wrap_err("Funding execution failed")?;

        ui::note(
            "Funding Complete",
            format!(
                "Successful transfers: {}\nTotal gas used: {}",
                ui::green(result.successful),
                ui::green(result.total_gas_used)
            ),
        )?;
        ui::success("Ecosystem wallets funded successfully!")?;
    }

    // 17. Skip deployment if requested
    if args.skip_deployment {
        ui::outro("Skipping contract deployment (--skip-deployment)")?;
        return Ok(());
    }

    // 18. Continue to deployment
    run_ecosystem_deployment(
        &args,
        context,
        &state_manager,
        &ecosystem_name,
        &chain_name,
        &rpc_url,
        &chain_wallets,
    )
    .await
}

/// Run Anvil-specific funding flow.
///
/// Uses the well-known Anvil default account (first account) to fund
/// wallets. Checks current balances and only funds wallets that need more ETH.
async fn run_anvil_funding(
    args: &DeployArgs,
    context: &Context,
    state_manager: &StateManager,
    ecosystem_name: &str,
    chain_name: &str,
    rpc_url: &Url,
) -> Result<()> {
    ui::info("Using Anvil funding mode (localhost detected)")?;

    // Load wallets from state
    let ecosystem_wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;

    let chain_wallets = state_manager
        .chain(chain_name)
        .wallets()
        .await
        .wrap_err_with(|| format!("Failed to load chain '{}' wallets", chain_name))?;

    ui::info(format!(
        "Loaded wallets: ecosystem={}, chain={}",
        ui::green(count_wallets(&ecosystem_wallets)),
        ui::green(count_wallets(&chain_wallets))
    ))?;

    // Build funding amounts from config
    let funding_defaults = &context.config().funding;
    let amounts = build_default_amounts(args, funding_defaults);

    // Create funder to check balances (normalize URL for host-side connection)
    let funder = AnvilFunder::with_rpc(&normalize_rpc_url(rpc_url.as_str()))?
        .with_amounts(amounts)
        .with_event_handler(Arc::new(LoggingEventHandler::new(Arc::clone(
            context.logger(),
        ))));

    // Get funding targets with current balances
    let spinner = cliclack::spinner();
    spinner.start("Checking wallet balances...");
    let targets = funder
        .get_funding_targets(&ecosystem_wallets, &chain_wallets)
        .await
        .wrap_err("Failed to check wallet balances")?;
    spinner.stop("Wallet balances checked");

    // Display current balances and funding plan
    display_anvil_funding_plan(&targets)?;

    let needs_funding = targets.iter().filter(|t| t.needs_funding).count();
    let already_funded = targets.len() - needs_funding;

    // Dry-run mode
    if args.dry_run {
        ui::outro("Dry-run mode: funding plan created but not executed")?;
        return Ok(());
    }

    // If all wallets are already funded, skip confirmation and funding
    if needs_funding == 0 {
        ui::success("All wallets already funded, skipping funding step.")?;
    } else {
        // Confirmation (default to yes for local dev)
        if !args.yes {
            let confirmed = ui::confirm(format!(
                "Proceed with Anvil funding ({} wallets)?",
                needs_funding
            ))
            .initial_value(true)
            .interact()
            .wrap_err("Failed to read confirmation")?;

            if !confirmed {
                ui::outro_cancel("Funding cancelled by user")?;
                return Ok(());
            }
        }

        // Execute Anvil funding
        ui::info("Funding wallets from Anvil default account...")?;

        let result = funder
            .fund_wallets(&ecosystem_wallets, &chain_wallets)
            .await
            .wrap_err("Anvil funding failed")?;

        ui::note(
            "Anvil Funding Complete",
            format!(
                "Wallets funded: {}\nWallets skipped (already funded): {}\nTotal gas used: {}",
                ui::green(result.successful),
                already_funded,
                ui::green(result.total_gas_used)
            ),
        )?;
    }

    // Skip deployment if requested
    if args.skip_deployment {
        ui::outro("Skipping contract deployment (--skip-deployment)")?;
        return Ok(());
    }

    // Continue to deployment
    run_ecosystem_deployment(
        args,
        context,
        state_manager,
        ecosystem_name,
        chain_name,
        rpc_url,
        &chain_wallets,
    )
    .await
}

/// Display Anvil funding plan with current balances and status.
fn display_anvil_funding_plan(targets: &[AnvilFundingTarget]) -> Result<()> {
    let mut lines = vec!["Using Anvil default account (account 0)".to_string()];

    for target in targets {
        let current = format_eth(target.current_balance);
        let required = format_eth(target.amount);
        let status = if target.needs_funding {
            format!("{}", ui::yellow("→ Will fund"))
        } else {
            format!("{}", ui::green("✓ Already funded"))
        };
        let label = format!("{} {}", target.source.prefix(), target.role);
        lines.push(format!(
            "{:20} {} ETH (current: {} ETH) {}",
            label,
            ui::green(&required),
            current,
            status
        ));
    }

    let needs_funding = targets.iter().filter(|t| t.needs_funding).count();
    let already_funded = targets.len() - needs_funding;
    lines.push(format!(
        "\nSummary: {} need funding, {} already funded",
        ui::green(needs_funding),
        already_funded
    ));

    ui::note("Anvil Funding Plan", lines.join("\n"))?;
    Ok(())
}

/// Display funding plan with current balances (normal network).
///
/// Matches the Anvil funding plan display format with boxed note.
fn display_funding_plan(
    funder_address: Address,
    targets: &[FundingTargetStatus],
    token_symbol: Option<&str>,
) -> Result<()> {
    let mut lines = vec![format!("Funder: {}", ui::green(funder_address))];

    for target in targets {
        let current = format_eth(target.current_eth);
        let required = format_eth(target.required_eth);
        let status = if target.needs_eth_funding {
            format!("{}", ui::yellow("→ Will fund"))
        } else {
            format!("{}", ui::green("✓ Already funded"))
        };
        let label = format!("{} {}", target.source.prefix(), target.role);
        lines.push(format!(
            "{:20} {} ETH (current: {} ETH) {}",
            label,
            ui::green(&required),
            current,
            status
        ));

        // Add token line if applicable
        if let (Some(req_tok), Some(cur_tok)) = (target.required_token, target.current_token) {
            let symbol = token_symbol.unwrap_or("CGT");
            let tok_status = if target.needs_token_funding {
                format!("{}", ui::yellow("→ Will fund"))
            } else {
                format!("{}", ui::green("✓ Already funded"))
            };
            lines.push(format!(
                "{:20} {} {} (current: {} {}) {}",
                "",
                ui::green(format_token(req_tok)),
                symbol,
                format_token(cur_tok),
                symbol,
                tok_status
            ));
        }
    }

    let needs_funding = targets
        .iter()
        .filter(|t| t.needs_eth_funding || t.needs_token_funding)
        .count();
    let already_funded = targets.len() - needs_funding;
    lines.push(format!(
        "\nSummary: {} need funding, {} already funded",
        ui::green(needs_funding),
        already_funded
    ));

    ui::note("Funding Plan", lines.join("\n"))?;
    Ok(())
}

/// Display funding summary in boxed note (transfer counts, gas, balances).
fn display_funding_summary(plan: &adi_funding::FundingPlan, config: &FundingConfig) -> Result<()> {
    let mut lines = vec![
        format!("Transfers needed: {}", ui::green(plan.transfer_count())),
        format!(
            "Total ETH to transfer: {} ETH",
            ui::green(format_eth(plan.total_eth_transfers()))
        ),
    ];

    if !plan.total_token_required.is_zero() {
        let symbol = config.token_symbol.as_deref().unwrap_or("tokens");
        lines.push(format!(
            "Total {} to transfer: {} {}",
            symbol,
            ui::green(format_token(plan.total_token_required)),
            symbol
        ));
    }

    lines.push(format!(
        "Estimated gas cost: {} ETH",
        ui::green(format_eth(plan.total_gas_cost))
    ));
    lines.push(format!(
        "Total ETH required: {} ETH",
        ui::green(format_eth(plan.total_eth_required))
    ));
    lines.push(format!(
        "Funder balance: {} ETH",
        ui::green(format_eth(plan.funder_eth_balance))
    ));

    if let Some(token_balance) = plan.funder_token_balance {
        let symbol = config.token_symbol.as_deref().unwrap_or("tokens");
        lines.push(format!(
            "Funder {} balance: {} {}",
            symbol,
            ui::green(format_token(token_balance)),
            symbol
        ));
    }

    let status = if plan.is_valid() {
        format!("{}", ui::green("Sufficient balance"))
    } else {
        format!("{}", ui::yellow("Insufficient balance"))
    };
    lines.push(format!("Status: {}", status));

    ui::note("Funding Summary", lines.join("\n"))?;
    Ok(())
}

/// Run ecosystem contract deployment and validator role configuration.
///
/// This is the shared deployment logic used by both Anvil and production funding paths.
async fn run_ecosystem_deployment(
    args: &DeployArgs,
    context: &Context,
    state_manager: &StateManager,
    ecosystem_name: &str,
    chain_name: &str,
    rpc_url: &Url,
    chain_wallets: &Wallets,
) -> Result<()> {
    // Validate protocol version is provided for deployment
    let protocol_version_str =
        resolve_protocol_version(args.protocol_version.as_ref(), context.config())?;
    let protocol_version = ProtocolVersion::parse(&protocol_version_str)
        .map_err(|e| eyre::eyre!("Invalid protocol version '{}': {}", protocol_version_str, e))?;
    // Determine if ecosystem contracts already exist
    let deploy_ecosystem = !ecosystem_contracts_deployed(state_manager).await?;

    // Run zkstack ecosystem init
    ui::info(format!(
        "Protocol version: {}",
        ui::green(&protocol_version)
    ))?;

    if deploy_ecosystem {
        ui::section("Deploying Ecosystem Contracts")?;
    } else {
        ui::section("Deploying Chain Contracts")?;
        ui::info("Ecosystem contracts already exist, deploying chain only")?;
    }

    let runner = ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        std::sync::Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;

    let ecosystem_path = context.config().state_dir.join(ecosystem_name);

    // Copy genesis.json to ecosystem directory if not present
    let genesis_src = context.config().state_dir.join(GENESIS_FILENAME);
    let genesis_dst = ecosystem_path.join(GENESIS_FILENAME);
    if !genesis_dst.exists() {
        if !genesis_src.exists() {
            return Err(eyre::eyre!(
                "genesis.json not found.\n\
                 Please place the genesis.json file at: {}",
                genesis_src.display()
            ));
        }
        std::fs::copy(&genesis_src, &genesis_dst)
            .wrap_err("Failed to copy genesis.json to ecosystem directory")?;
        ui::info("Copied genesis.json to ecosystem directory")?;
    }

    // Resolve gas multiplier from args or config
    let gas_multiplier = resolve_gas_multiplier(args, context);

    // Compute gas price: skip for localhost, estimate + apply multiplier for testnets
    let gas_price_wei = if is_localhost_rpc(rpc_url.as_str()) {
        None
    } else {
        // Estimate gas price and apply multiplier
        let provider = adi_funding::FundingProvider::new(rpc_url.as_str())
            .wrap_err("Failed to create provider for gas estimation")?;
        let estimated = provider
            .get_gas_price()
            .await
            .wrap_err("Failed to estimate gas price")?;
        Some(estimated * u128::from(gas_multiplier) / 100)
    };

    let init_msg = if deploy_ecosystem {
        "Running zkstack ecosystem init..."
    } else {
        "Running zkstack chain init..."
    };
    ui::info(init_msg)?;

    // Build verification options from CLI args and config
    let verification = resolve_verification_opts(args, context);

    if verification.enabled {
        ui::info(format!(
            "Verification enabled: {} ({})",
            ui::green(verification.verifier.as_deref().unwrap_or("etherscan")),
            ui::green(verification.verifier_url.as_deref().unwrap_or("auto"))
        ))?;
    }

    let exit_code = runner
        .run_zkstack_ecosystem_init(
            &ecosystem_path,
            rpc_url.as_str(),
            gas_price_wei,
            &protocol_version.to_semver(),
            deploy_ecosystem,
            chain_name,
            &verification,
        )
        .await
        .wrap_err("Failed to run zkstack ecosystem init")?;

    if exit_code != 0 {
        return Err(eyre::eyre!(
            "zkstack ecosystem init failed with exit code {}",
            exit_code
        ));
    }

    let success_msg = if deploy_ecosystem {
        "Ecosystem contracts deployed successfully!"
    } else {
        "Chain contracts deployed successfully!"
    };
    ui::success(success_msg)?;

    // Enrich ecosystem contracts with facet and implementation addresses
    enrich_ecosystem_contracts(context, state_manager, rpc_url).await?;

    // Log all deployment files (with warnings for unhandled ones)
    log_deployment_files(&ecosystem_path, chain_name)?;

    // Re-read chain contracts from state (now populated after deployment)
    let chain_contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts after deployment")?;

    let deployed = DeployedContracts::try_from_chain_contracts(&chain_contracts)
        .wrap_err("Missing required contract addresses after deployment")?;

    ui::note(
        "Deployed Contracts",
        format!(
            "Diamond proxy: {}\nValidator timelock: {}\nChain admin: {}",
            ui::green(deployed.diamond_proxy),
            ui::green(deployed.validator_timelock),
            ui::green(deployed.chain_admin)
        ),
    )?;

    // Get chain governor private key for signing validator role txs
    let governor_key = chain_wallets
        .governor
        .as_ref()
        .ok_or_else(|| eyre::eyre!("Chain governor wallet required for validator role setup"))?
        .private_key
        .clone();

    // Add validator roles
    ui::section("Configuring Validator Roles")?;

    // Normalize URL for host-side connection (host.docker.internal -> localhost)
    let normalized_rpc = normalize_rpc_url(rpc_url.as_str());

    // Pass gas_multiplier (None for localhost to skip multiplier)
    let validator_gas_multiplier = if is_localhost_rpc(rpc_url.as_str()) {
        None
    } else {
        Some(gas_multiplier)
    };

    let tx_hashes = add_validator_roles(
        &normalized_rpc,
        &deployed,
        chain_wallets,
        &governor_key,
        validator_gas_multiplier,
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to add validator roles")?;

    ui::success(format!(
        "Validator roles configured: {} transactions confirmed",
        ui::green(tx_hashes.len())
    ))?;

    // Configure L3 DA mode if requested (disables blobs, uses calldata)
    let is_l3 = resolve_l3_mode(args, context);
    if is_l3 {
        ui::section("Configuring L3 DA Mode")?;

        let l1_da_validator = get_l1_da_validator_address(state_manager, chain_name)
            .await
            .wrap_err("Failed to get L1 DA validator address")?;

        let tx_hash = configure_l3_da(
            &normalized_rpc,
            deployed.chain_admin,
            deployed.diamond_proxy,
            l1_da_validator,
            &governor_key,
            validator_gas_multiplier,
            context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to configure L3 DA mode")?;

        ui::success(format!(
            "L3 DA mode configured (calldata): {}",
            ui::green(tx_hash)
        ))?;
    }

    // Final success message
    ui::note(
        "Deployment Summary",
        format!(
            "Ecosystem: {}\nChain: {}\nDiamond proxy: {}",
            ui::green(ecosystem_name),
            ui::green(chain_name),
            ui::green(deployed.diamond_proxy)
        ),
    )?;
    ui::outro("Deployment complete! You can now start containers and operate the rollup.")?;

    Ok(())
}

/// Enrich ecosystem contracts with facet and implementation addresses.
///
/// Reads diamond_cut_data from state and parses facet addresses.
/// Reads implementation addresses from proxy contracts via RPC.
/// Saves enriched contracts back to state for future use (e.g., verification).
async fn enrich_ecosystem_contracts(
    context: &Context,
    state_manager: &StateManager,
    rpc_url: &Url,
) -> Result<()> {
    context
        .logger()
        .debug("Enriching ecosystem contracts with facet and implementation addresses");

    // Load ecosystem contracts from state
    let mut ecosystem_contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts for enrichment")?;

    // Track if any changes were made
    let mut enriched = false;

    // 1. Parse diamond_cut_data for facet addresses
    if let Some(ref mut ctm) = ecosystem_contracts.zksync_os_ctm {
        if ctm.admin_facet_addr.is_none() {
            if let Some(ref diamond_cut_data) = ctm.diamond_cut_data {
                match parse_diamond_cut_data(diamond_cut_data) {
                    Ok(facets) => {
                        ctm.admin_facet_addr = facets.admin_facet;
                        ctm.executor_facet_addr = facets.executor_facet;
                        ctm.mailbox_facet_addr = facets.mailbox_facet;
                        ctm.getters_facet_addr = facets.getters_facet;
                        ctm.diamond_init_addr = facets.diamond_init;
                        enriched = true;
                        context
                            .logger()
                            .debug("Extracted facet addresses from diamond_cut_data");
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

    // 2. Read implementation addresses from proxy contracts via RPC
    if ecosystem_contracts
        .zksync_os_ctm
        .as_ref()
        .is_some_and(|ctm| ctm.bridgehub_impl_addr.is_none())
    {
        // Normalize URL for host-side connection
        let normalized_rpc = normalize_rpc_url(rpc_url.as_str());
        let provider = adi_funding::FundingProvider::new(&normalized_rpc)
            .wrap_err("Failed to create provider for implementation address reading")?;

        let impls = read_all_implementations(
            provider.inner(),
            &ecosystem_contracts,
            std::sync::Arc::clone(context.logger()),
        )
        .await;

        apply_implementations(&mut ecosystem_contracts, &impls);
        enriched = true;
        context
            .logger()
            .debug("Read implementation addresses from proxy contracts");
    }

    // 3. Save enriched contracts back to state
    if enriched {
        state_manager
            .ecosystem()
            .update_contracts(&ecosystem_contracts)
            .await
            .wrap_err("Failed to save enriched ecosystem contracts")?;
        ui::info("Enriched contracts with facet and implementation addresses")?;
    }

    Ok(())
}

/// Resolve ecosystem name from args or config.
fn resolve_ecosystem_name(args: &DeployArgs, context: &Context) -> Result<String> {
    args.ecosystem_name
        .clone()
        .or_else(|| Some(context.config().ecosystem.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem name required: use --ecosystem-name or set in config")
        })
}

/// Resolve chain name from args or config.
fn resolve_chain_name(args: &DeployArgs, context: &Context) -> Result<String> {
    args.chain_name
        .clone()
        .or_else(|| Some(context.config().ecosystem.chain_name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| eyre::eyre!("Chain name required: use --chain-name or set in config"))
}

/// Create state manager for the ecosystem with optional S3 sync.
async fn create_state_manager(ecosystem_name: &str, context: &Context) -> Result<StateManager> {
    let (state_manager, _control) = create_state_manager_with_s3(ecosystem_name, context).await?;
    Ok(state_manager)
}

/// Validate that ecosystem state exists.
async fn validate_ecosystem_exists(
    state_manager: &StateManager,
    ecosystem_name: &str,
) -> Result<()> {
    if !state_manager.exists().await? {
        return Err(eyre::eyre!(
            "Ecosystem '{}' not found. Run 'adi init' first.",
            ecosystem_name
        ));
    }
    Ok(())
}

/// Validate that chain exists within ecosystem.
async fn validate_chain_exists(
    state_manager: &StateManager,
    chain_name: &str,
    ecosystem_name: &str,
) -> Result<()> {
    if !state_manager.chain(chain_name).exists().await? {
        return Err(eyre::eyre!(
            "Chain '{}' not found in ecosystem '{}'. Initialize chain first.",
            chain_name,
            ecosystem_name
        ));
    }
    Ok(())
}

/// Resolve funder private key from args or config.
fn resolve_funder_key(args: &DeployArgs, context: &Context) -> Result<SecretString> {
    // Try args first
    if let Some(key) = &args.funder_key {
        if !key.is_empty() {
            return Ok(SecretString::from(key.clone()));
        }
    }

    // Try config (which includes env var overrides via ADI__FUNDING__FUNDER_KEY)
    if let Some(key) = &context.config().funding.funder_key {
        return Ok(key.clone());
    }

    Err(eyre::eyre!(
        "Funder key required: use --funder-key, ADI_FUNDER_KEY env var, or set funding.funder_key in config"
    ))
}

/// Resolve gas multiplier from args or config.
///
/// Priority: CLI arg > top-level config > funding config (backward compat) > default (120)
fn resolve_gas_multiplier(args: &DeployArgs, context: &Context) -> u64 {
    args.gas_multiplier
        .unwrap_or_else(|| context.config().gas_multiplier)
}

/// Resolve L3 mode from args or config.
///
/// Priority: CLI arg > config file
fn resolve_l3_mode(args: &DeployArgs, context: &Context) -> bool {
    args.l3 || context.config().ecosystem.l3
}

/// Resolve verification options from CLI args and config.
///
/// Priority: CLI --no-verify > CLI flags > config
fn resolve_verification_opts(args: &DeployArgs, context: &Context) -> VerificationOpts {
    use secrecy::ExposeSecret;
    let cfg = &context.config().verification;

    // --no-verify takes absolute precedence
    if args.no_verify {
        return VerificationOpts::default();
    }

    // Determine if enabled: CLI --verify > CLI --explorer-url > config
    let enabled = args.verify || args.explorer_url.is_some() || cfg.explorer_url.is_some();

    if !enabled {
        return VerificationOpts::default();
    }

    // Build opts with CLI overrides > config fallbacks
    VerificationOpts {
        enabled: true,
        verifier: args
            .explorer
            .map(|e| e.to_string())
            .or_else(|| cfg.explorer.clone()),
        verifier_url: args
            .explorer_url
            .as_ref()
            .map(|u| u.to_string())
            .or_else(|| cfg.explorer_url.as_ref().map(|u| u.to_string())),
        api_key: args
            .explorer_api_key
            .clone()
            .or_else(|| cfg.api_key.as_ref().map(|s| s.expose_secret().to_string())),
    }
}

/// Get L1 DA validator address from state.
///
/// Looks in chain contracts first (l1.rollup_l1_da_validator_addr),
/// then falls back to ecosystem contracts (ecosystem_contracts.rollup_l1_da_validator_addr
/// or zksync_os_ctm.rollup_l1_da_validator_addr).
async fn get_l1_da_validator_address(
    state_manager: &StateManager,
    chain_name: &str,
) -> Result<Address> {
    // Try chain contracts first
    let chain_contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts")?;

    if let Some(l1) = &chain_contracts.l1 {
        if let Some(addr) = l1.rollup_l1_da_validator_addr {
            return Ok(addr);
        }
    }

    // Fall back to chain's ecosystem_contracts reference
    if let Some(eco) = &chain_contracts.ecosystem_contracts {
        if let Some(addr) = eco.rollup_l1_da_validator_addr {
            return Ok(addr);
        }
    }

    // Try ecosystem-level contracts
    let eco_contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")?;

    if let Some(ctm) = &eco_contracts.zksync_os_ctm {
        if let Some(addr) = ctm.rollup_l1_da_validator_addr {
            return Ok(addr);
        }
    }

    Err(eyre::eyre!(
        "L1 DA validator address not found in state. \
         Ensure deployment completed successfully."
    ))
}

/// Build FundingConfig from ecosystem metadata.
///
/// Token address is read from chain's base_token config.
/// Token symbol is queried via RPC if not provided.
/// Funding amounts are resolved from: CLI args > config file > library defaults.
async fn build_funding_config(
    args: &DeployArgs,
    context: &Context,
    rpc_url: &Url,
    state_manager: &StateManager,
    chain_name: &str,
    provider: &adi_funding::FundingProvider,
) -> Result<FundingConfig> {
    let funding_defaults = &context.config().funding;

    let mut config = FundingConfig::new(rpc_url.as_str());

    // Set gas multiplier (args > config > default)
    let multiplier = args
        .gas_multiplier
        .unwrap_or(context.config().gas_multiplier);
    config = config.with_gas_multiplier(multiplier);
    context
        .logger()
        .debug(&format!("Gas multiplier: {}%", multiplier));

    // Build custom amounts (args > config > library defaults)
    let amounts = build_default_amounts(args, funding_defaults);
    config = config.with_amounts(amounts);

    // Get base token from chain metadata
    let base_token = state_manager
        .chain(chain_name)
        .metadata()
        .await
        .wrap_err_with(|| format!("Failed to load chain '{}' metadata", chain_name))?
        .base_token;

    // Determine token address to use (with fallback to config if chain has ETH but config has custom)
    let ecosystem_defaults = &context.config().ecosystem;
    let token_address = if base_token.is_eth() {
        // Chain metadata has ETH - check if config specifies a custom token
        let config_token = ecosystem_defaults.base_token_address;
        if config_token != adi_types::ETH_TOKEN_ADDRESS {
            // Config has custom token but chain metadata has ETH
            // This happens when zkstack ignores --base-token-address
            context.logger().warning(&format!(
                "Chain metadata has ETH as base_token, but config specifies {}. \
                 Using config value (zkstack may have ignored --base-token-address).",
                config_token
            ));
            Some(config_token)
        } else {
            None // Both are ETH, no custom token
        }
    } else {
        Some(base_token.address) // Chain has custom token
    };

    // If custom token, configure it
    if let Some(address) = token_address {
        let symbol = match adi_funding::get_token_symbol(provider, address).await {
            Ok(s) => Some(s),
            Err(e) => {
                ui::warning(format!("Failed to query token symbol: {}", e))?;
                None
            }
        };
        config = config.with_token(address, symbol.clone());
        ui::info(format!(
            "Custom gas token: {} ({})",
            ui::green(address),
            ui::green(symbol.as_deref().unwrap_or("unknown"))
        ))?;
    }

    Ok(config)
}

/// Build DefaultAmounts from CLI args and config, falling back to library defaults.
///
/// Priority: CLI args > config file > library defaults
fn build_default_amounts(
    args: &DeployArgs,
    config: &crate::config::FundingDefaults,
) -> DefaultAmounts {
    let defaults = DefaultAmounts::default();

    DefaultAmounts {
        deployer_eth: args
            .deployer_eth
            .map(eth_to_wei)
            .or_else(|| config.deployer_eth.map(eth_to_wei))
            .unwrap_or(defaults.deployer_eth),
        governor_eth: args
            .governor_eth
            .map(eth_to_wei)
            .or_else(|| config.governor_eth.map(eth_to_wei))
            .unwrap_or(defaults.governor_eth),
        governor_cgt_units: args
            .governor_cgt_units
            .or(config.governor_cgt_units)
            .unwrap_or(defaults.governor_cgt_units),
        operator_eth: args
            .operator_eth
            .map(eth_to_wei)
            .or_else(|| config.operator_eth.map(eth_to_wei))
            .unwrap_or(defaults.operator_eth),
        // Not configurable - use library defaults
        blob_operator_eth: defaults.blob_operator_eth,
        prove_operator_eth: args
            .prove_operator_eth
            .map(eth_to_wei)
            .or_else(|| config.prove_operator_eth.map(eth_to_wei))
            .unwrap_or(defaults.prove_operator_eth),
        execute_operator_eth: args
            .execute_operator_eth
            .map(eth_to_wei)
            .or_else(|| config.execute_operator_eth.map(eth_to_wei))
            .unwrap_or(defaults.execute_operator_eth),
        // Not configurable - use library defaults
        fee_account_eth: defaults.fee_account_eth,
        token_multiplier_setter_eth: defaults.token_multiplier_setter_eth,
    }
}

/// Count non-None wallets in Wallets struct.
fn count_wallets(wallets: &Wallets) -> usize {
    let mut count = 0;
    if wallets.deployer.is_some() {
        count += 1;
    }
    if wallets.operator.is_some() {
        count += 1;
    }
    if wallets.blob_operator.is_some() {
        count += 1;
    }
    if wallets.prove_operator.is_some() {
        count += 1;
    }
    if wallets.execute_operator.is_some() {
        count += 1;
    }
    if wallets.fee_account.is_some() {
        count += 1;
    }
    if wallets.governor.is_some() {
        count += 1;
    }
    if wallets.token_multiplier_setter.is_some() {
        count += 1;
    }
    count
}

/// Display detailed funding plan (for dry-run mode).
fn display_plan_details(plan: &adi_funding::FundingPlan) -> Result<()> {
    ui::info("Planned Transfers:")?;
    for (i, transfer) in plan.transfers.iter().enumerate() {
        let amount_str = match &transfer.transfer_type {
            adi_funding::TransferType::Eth { amount } => {
                format!("{} {}", ui::green(format_eth(*amount)), ui::green("ETH"))
            }
            adi_funding::TransferType::Token { amount, symbol, .. } => {
                format!("{} {}", ui::green(format_token(*amount)), ui::green(symbol))
            }
        };
        ui::info(format!(
            "  [{}] {:24} -> {}  ({})",
            i + 1,
            transfer.role,
            ui::green(transfer.to),
            amount_str
        ))?;
    }
    Ok(())
}

/// Format U256 as ETH (with 4 decimal places).
fn format_eth(wei: U256) -> String {
    let eth_str = wei.to_string();
    let len = eth_str.len();

    if len <= 18 {
        // Less than 1 ETH
        let padded = format!("{:0>18}", eth_str);
        let decimal = padded.trim_start_matches('0');
        if decimal.is_empty() {
            "0.0000".to_string()
        } else {
            format!("0.{:.4}", &padded[..4.min(padded.len())])
        }
    } else {
        // 1 ETH or more
        let whole = &eth_str[..len - 18];
        let decimal = &eth_str[len - 18..len - 14];
        format!("{}.{}", whole, decimal)
    }
}

/// Format U256 as token amount (assuming 18 decimals).
fn format_token(amount: U256) -> String {
    format_eth(amount) // Same format for now
}

/// Convert ETH amount (f64) to wei (U256).
///
/// Handles negative values by clamping to zero.
/// Precision limited by f64 mantissa (~15-17 significant digits).
fn eth_to_wei(eth: f64) -> U256 {
    if eth <= 0.0 {
        return U256::ZERO;
    }
    let wei = eth * 1e18;
    // Clamp to u128 max to avoid overflow (handles ~3.4e20 ETH max)
    let clamped = wei.min(u128::MAX as f64);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let wei_u128 = clamped as u128;
    U256::from(wei_u128)
}

/// Config files that CLI actively parses and uses at ecosystem level.
const KNOWN_ECOSYSTEM_FILES: &[&str] = &[
    "ZkStack.yaml",
    "genesis.json",
    "configs/contracts.yaml",
    "configs/wallets.yaml",
    "configs/initial_deployments.yaml",
    "configs/apps/portal.config.json",
];

/// Config files that CLI actively parses and uses at chain level.
const KNOWN_CHAIN_FILES: &[&str] = &[
    "ZkStack.yaml",
    "configs/contracts.yaml",
    "configs/wallets.yaml",
    "configs/genesis.yaml",
    "configs/genesis.json",
    "configs/general.yaml",
    "configs/secrets.yaml",
    "configs/external_node.yaml",
];

/// Recursively collect all file paths from a directory.
fn collect_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path));
        } else if path.is_file() {
            files.push(path);
        }
    }
    files
}

/// Log all deployment files and warn about unhandled ones.
///
/// Scans the state directory for config files (yaml, yml, json) and logs:
/// - files that CLI actively parses as success
/// - files that are saved but not processed by CLI as warning
fn log_deployment_files(state_path: &Path, chain_name: &str) -> Result<()> {
    let mut known_files = Vec::new();
    let mut unknown_files = Vec::new();

    for path in collect_files(state_path)
        .into_iter()
        .filter(|p| is_config_file(p))
    {
        let Ok(relative) = path.strip_prefix(state_path) else {
            continue;
        };
        let rel_str = relative.to_string_lossy();

        // Check if this is a known file
        let chain_prefix = format!("chains/{}/", chain_name);
        let is_known = if let Some(chain_rel) = rel_str.strip_prefix(&chain_prefix) {
            KNOWN_CHAIN_FILES.contains(&chain_rel)
        } else {
            KNOWN_ECOSYSTEM_FILES.contains(&rel_str.as_ref())
        };

        if is_known {
            known_files.push(relative.display().to_string());
        } else {
            unknown_files.push(relative.display().to_string());
        }
    }

    // Format all files as a single note
    let mut content = known_files.join("\n");
    if !unknown_files.is_empty() {
        if !content.is_empty() {
            content.push_str("\n\nNot processed by CLI:\n");
        }
        content.push_str(&unknown_files.join("\n"));
    }

    ui::note("Deployment files", content)?;
    Ok(())
}

/// Check if file is a config file (yaml, yml, json).
fn is_config_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("yaml" | "yml" | "json")
    )
}

/// Check if ecosystem contracts have already been deployed.
///
/// Returns `true` if contracts.yaml exists and contains bridgehub_proxy_addr,
/// indicating ecosystem contracts were deployed in a previous run.
async fn ecosystem_contracts_deployed(state_manager: &StateManager) -> Result<bool> {
    if !state_manager.ecosystem().contracts_exist().await? {
        return Ok(false);
    }

    match state_manager.ecosystem().contracts().await {
        Ok(contracts) => Ok(contracts.bridgehub_addr().is_some()),
        Err(_) => Ok(false),
    }
}
