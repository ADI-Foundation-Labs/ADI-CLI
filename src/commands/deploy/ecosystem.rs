//! Ecosystem deployment command implementation.
//!
//! This command:
//! 1. Funds ecosystem and chain wallets
//! 2. Deploys ecosystem contracts via zkstack
//! 3. Configures validator roles for operators

use adi_ecosystem::{add_validator_roles, DeployedContracts};
use adi_funding::{
    get_wallet_balance, is_localhost_rpc, normalize_rpc_url, AnvilFunder, AnvilFundingTarget,
    DefaultAmounts, FundingConfig, FundingError, FundingExecutor, FundingPlanBuilder,
    LoggingEventHandler,
};
use adi_state::StateManager;
use adi_toolkit::{ProtocolVersion, ToolkitRunner, GENESIS_FILENAME};
use adi_types::Wallets;
use alloy_primitives::{Address, U256};
use clap::Args;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use url::Url;
use walkdir::WalkDir;

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

    /// Custom gas price in wei (for non-local networks). If not set, gas price is estimated
    #[arg(long, help = "Custom gas price in wei (for non-local networks)")]
    pub gas_price_wei: Option<u128>,

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
    let state_manager = create_state_manager(&ecosystem_name, context)?;
    validate_ecosystem_exists(&state_manager, &ecosystem_name).await?;

    // 4. Validate chain exists
    validate_chain_exists(&state_manager, &chain_name, &ecosystem_name).await?;

    // 5. Skip funding if requested
    if args.skip_funding {
        ui::info("Skipping wallet funding (--skip-funding)")?;
        ui::outro("Ecosystem deployment complete (funding skipped)")?;
        return Ok(());
    }

    // 6. Resolve RPC URL (args > config)
    let rpc_url = resolve_rpc_url(&args, context)?;

    // Display deployment info
    ui::info(format!(
        "Ecosystem: {}\n\
         Chain: {}\n\
         Settlement layer RPC: {}",
        ui::green(&ecosystem_name),
        ui::green(&chain_name),
        ui::green(&rpc_url)
    ))?;

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

    // 9. Create executor with logging handler (needed for provider)
    let executor = FundingExecutor::new(rpc_url.as_str(), &funder_key)
        .wrap_err("Failed to create funding executor")?
        .with_event_handler(Arc::new(LoggingEventHandler::new(Arc::clone(
            context.logger(),
        ))));

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

    // 11. Display current wallet balances
    display_wallet_balances(
        executor.provider(),
        &ecosystem_wallets,
        &chain_wallets,
        &chain_name,
        funding_config.token_address,
        funding_config.token_symbol.as_deref(),
    )
    .await?;

    // 12. Build funding plan
    ui::info("Building funding plan...")?;
    let plan_result = FundingPlanBuilder::new(executor.provider(), &funding_config, funder_address)
        .with_ecosystem_wallets(&ecosystem_wallets)
        .with_chain_wallets(&chain_wallets)
        .build()
        .await;

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
        ui::info("============================================================")?;
        ui::info("Funding Plan Summary")?;
        ui::info("============================================================")?;
        ui::info(format!(
            "  Transfers needed: {}",
            ui::green(plan.transfer_count())
        ))?;
        ui::info(format!(
            "  Total ETH to transfer: {} {}",
            ui::green(format_eth(plan.total_eth_transfers())),
            ui::green("ETH")
        ))?;
        if !plan.total_token_required.is_zero() {
            let symbol = funding_config.token_symbol.as_deref().unwrap_or("tokens");
            ui::info(format!(
                "  Total {} to transfer: {} {}",
                symbol,
                ui::green(format_token(plan.total_token_required)),
                ui::green(symbol)
            ))?;
        }
        ui::info(format!(
            "  Estimated gas cost: {} {}",
            ui::green(format_eth(plan.total_gas_cost)),
            ui::green("ETH")
        ))?;
        ui::info(format!(
            "  Total ETH required: {} {}",
            ui::green(format_eth(plan.total_eth_required)),
            ui::green("ETH")
        ))?;
        ui::info(format!(
            "  Funder balance: {} {}",
            ui::green(format_eth(plan.funder_eth_balance)),
            ui::green("ETH")
        ))?;
        if let Some(token_balance) = plan.funder_token_balance {
            let symbol = funding_config.token_symbol.as_deref().unwrap_or("tokens");
            ui::info(format!(
                "  Funder {} balance: {} {}",
                symbol,
                ui::green(format_token(token_balance)),
                ui::green(symbol)
            ))?;
        }

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
        ui::success("  Status: Sufficient balance")?;
        ui::info("============================================================")?;

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

        // Execute funding
        ui::info("Executing funding transfers...")?;
        let result = executor
            .execute(&plan)
            .await
            .wrap_err("Funding execution failed")?;

        ui::info("============================================================")?;
        ui::success("Funding Complete!")?;
        ui::info("============================================================")?;
        ui::info(format!(
            "  Successful transfers: {}",
            ui::green(result.successful)
        ))?;
        ui::info(format!(
            "  Total gas used: {}",
            ui::green(result.total_gas_used)
        ))?;
        ui::info("============================================================")?;

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
    let targets = funder
        .get_funding_targets(&ecosystem_wallets, &chain_wallets)
        .await
        .wrap_err("Failed to check wallet balances")?;

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

        ui::success(format!(
            "Anvil Funding Complete!\n\
             ============================================================\n\
             Wallets funded: {}\n\
             Wallets skipped (already funded): {}\n\
             Total gas used: {}",
            ui::green(result.successful),
            already_funded,
            ui::green(result.total_gas_used)
        ))?;
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
    let mut lines = vec![
        "Anvil Funding Plan".to_string(),
        "============================================================".to_string(),
        "Using Anvil default account (account 0)".to_string(),
    ];

    for target in targets {
        let current = format_eth(target.current_balance);
        let required = format_eth(target.amount);
        let status = if target.needs_funding {
            format!("{}", ui::yellow("→ Will fund"))
        } else {
            format!("{}", ui::green("✓ Already funded"))
        };
        lines.push(format!(
            "{:20} {} ETH (current: {} ETH) {}",
            target.role.to_string(),
            ui::green(&required),
            current,
            status
        ));
    }

    let needs_funding = targets.iter().filter(|t| t.needs_funding).count();
    let already_funded = targets.len() - needs_funding;
    lines.push("============================================================".to_string());
    lines.push(format!(
        "Summary: {} need funding, {} already funded",
        ui::green(needs_funding),
        already_funded
    ));

    ui::info(lines.join("\n"))?;
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
    let protocol_version_str = args.protocol_version.as_ref().ok_or_else(|| {
        eyre::eyre!(
            "Protocol version required for deployment. Use --protocol-version (e.g., v30.0.2)"
        )
    })?;
    let protocol_version = ProtocolVersion::parse(protocol_version_str)
        .map_err(|e| eyre::eyre!("Invalid protocol version '{}': {}", protocol_version_str, e))?;
    // Run zkstack ecosystem init
    ui::info(format!(
        "Protocol version: {}\n\
         ============================================================\n\
         Deploying Ecosystem Contracts\n\
         ============================================================",
        ui::green(&protocol_version)
    ))?;

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

    ui::info("Running zkstack ecosystem init...")?;

    let exit_code = runner
        .run_zkstack_ecosystem_init(
            &ecosystem_path,
            rpc_url.as_str(),
            args.gas_price_wei,
            &protocol_version.to_semver(),
        )
        .await
        .wrap_err("Failed to run zkstack ecosystem init")?;

    if exit_code != 0 {
        return Err(eyre::eyre!(
            "zkstack ecosystem init failed with exit code {}",
            exit_code
        ));
    }

    ui::success("Ecosystem contracts deployed successfully!")?;

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
    ui::info("============================================================")?;
    ui::info("Configuring Validator Roles")?;
    ui::info("============================================================")?;

    // Normalize URL for host-side connection (host.docker.internal -> localhost)
    let normalized_rpc = normalize_rpc_url(rpc_url.as_str());
    let tx_hashes = add_validator_roles(
        &normalized_rpc,
        &deployed,
        chain_wallets,
        &governor_key,
        args.gas_price_wei,
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to add validator roles")?;

    ui::success(format!(
        "Validator roles configured: {} transactions confirmed",
        ui::green(tx_hashes.len())
    ))?;

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

/// Create state manager for the ecosystem.
fn create_state_manager(ecosystem_name: &str, context: &Context) -> Result<StateManager> {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    Ok(StateManager::with_backend_type_and_logger(
        context.config().state_backend.clone(),
        &ecosystem_path,
        std::sync::Arc::clone(context.logger()),
    ))
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

/// Resolve RPC URL from args or config.
fn resolve_rpc_url(args: &DeployArgs, context: &Context) -> Result<Url> {
    // Try args first (CLI flag or ADI_RPC_URL env var)
    if let Some(url) = &args.rpc_url {
        return Ok(url.clone());
    }

    // Try config (which includes ADI__FUNDING__RPC_URL env var)
    if let Some(url) = &context.config().funding.rpc_url {
        return Ok(url.clone());
    }

    Err(eyre::eyre!(
        "RPC URL required: use --rpc-url, ADI_RPC_URL env var, or set funding.rpc_url in config"
    ))
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
        .unwrap_or(funding_defaults.gas_multiplier);
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

/// Display current wallet balances.
async fn display_wallet_balances(
    provider: &adi_funding::FundingProvider,
    ecosystem_wallets: &Wallets,
    chain_wallets: &Wallets,
    chain_name: &str,
    token_address: Option<Address>,
    token_symbol: Option<&str>,
) -> Result<()> {
    ui::info("============================================================")?;
    ui::info("Current Wallet Balances")?;
    ui::info("============================================================")?;

    // Ecosystem wallets
    ui::info("Ecosystem Wallets:")?;
    display_wallet_balance(
        provider,
        "deployer",
        ecosystem_wallets.deployer.as_ref(),
        token_address,
        token_symbol,
    )
    .await?;
    display_wallet_balance(
        provider,
        "governor",
        ecosystem_wallets.governor.as_ref(),
        token_address,
        token_symbol,
    )
    .await?;

    // Chain wallets (only those that will be funded)
    ui::info(format!("Chain Wallets ({}):", chain_name))?;
    display_wallet_balance(
        provider,
        "governor",
        chain_wallets.governor.as_ref(),
        token_address,
        token_symbol,
    )
    .await?;
    display_wallet_balance(
        provider,
        "operator",
        chain_wallets.operator.as_ref(),
        token_address,
        token_symbol,
    )
    .await?;
    display_wallet_balance(
        provider,
        "prove_operator",
        chain_wallets.prove_operator.as_ref(),
        token_address,
        token_symbol,
    )
    .await?;
    display_wallet_balance(
        provider,
        "execute_operator",
        chain_wallets.execute_operator.as_ref(),
        token_address,
        token_symbol,
    )
    .await?;

    ui::info("============================================================")?;

    Ok(())
}

/// Display a single wallet's balance.
async fn display_wallet_balance(
    provider: &adi_funding::FundingProvider,
    role: &str,
    wallet: Option<&adi_types::Wallet>,
    token_address: Option<Address>,
    token_symbol: Option<&str>,
) -> Result<()> {
    let Some(w) = wallet else {
        return Ok(());
    };

    let balance = get_wallet_balance(provider, w.address, token_address)
        .await
        .wrap_err_with(|| format!("Failed to get balance for {}", role))?;

    let eth_str = ui::green(format_eth(balance.eth_balance));
    let symbol = token_symbol.unwrap_or("tokens");
    let token_str = balance
        .token_balance
        .map(|t| format!(" + {} {}", ui::green(format_token(t)), ui::green(symbol)))
        .unwrap_or_default();

    ui::info(format!(
        "  {:24} ({}): {} {}{}",
        role,
        ui::green(w.address),
        eth_str,
        ui::green("ETH"),
        token_str
    ))?;

    Ok(())
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

/// Log all deployment files and warn about unhandled ones.
///
/// Scans the state directory for config files (yaml, yml, json) and logs:
/// - files that CLI actively parses as success
/// - files that are saved but not processed by CLI as warning
fn log_deployment_files(state_path: &Path, chain_name: &str) -> Result<()> {
    ui::info("Deployment files:")?;

    for entry in WalkDir::new(state_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_config_file(e.path()))
    {
        let Ok(relative) = entry.path().strip_prefix(state_path) else {
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
            ui::success(format!("  {}", relative.display()))?;
        } else {
            ui::warning(format!("  {} (not processed by CLI)", relative.display()))?;
        }
    }
    Ok(())
}

/// Check if file is a config file (yaml, yml, json).
fn is_config_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("yaml" | "yml" | "json")
    )
}
