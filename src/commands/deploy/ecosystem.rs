//! Ecosystem deployment command implementation.
//!
//! This command funds ecosystem and chain wallets before deployment.
//! Actual contract deployment will be added in a future implementation.

use adi_funding::{
    get_wallet_balance, DefaultAmounts, FundingConfig, FundingError, FundingExecutor,
    FundingPlanBuilder, LoggingEventHandler,
};
use adi_state::StateManager;
use adi_types::Wallets;
use alloy_primitives::{Address, U256};
use clap::Args;
use colored::Colorize;
use dialoguer::Confirm;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;

use crate::context::Context;
use crate::error::{Result, WrapErr};

/// Arguments for `deploy ecosystem` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct EcosystemDeployArgs {
    /// Ecosystem name (from config if not provided).
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// Chain name to fund (from config if not provided).
    #[arg(long)]
    pub chain_name: Option<String>,

    /// RPC URL for the settlement layer.
    #[arg(long, env = "ADI_RPC_URL")]
    pub rpc_url: Option<Url>,

    /// Funder wallet private key.
    #[arg(long, env = "ADI_FUNDER_KEY")]
    pub funder_key: Option<String>,

    /// Gas price multiplier percentage (default: 120 = 20% buffer).
    #[arg(long)]
    pub gas_multiplier: Option<u64>,

    /// Deployer ETH amount (overrides config default of 1.0).
    #[arg(long)]
    pub deployer_eth: Option<f64>,

    /// Governor ETH amount (overrides config default of 1.0).
    #[arg(long)]
    pub governor_eth: Option<f64>,

    /// Governor custom gas token amount (overrides config default of 5.0).
    #[arg(long)]
    pub governor_cgt_units: Option<f64>,

    /// Operator ETH amount (overrides config default of 5.0).
    #[arg(long)]
    pub operator_eth: Option<f64>,

    /// Prove operator ETH amount (overrides config default of 5.0).
    #[arg(long)]
    pub prove_operator_eth: Option<f64>,

    /// Execute operator ETH amount (overrides config default of 5.0).
    #[arg(long)]
    pub execute_operator_eth: Option<f64>,

    /// Skip wallet funding step.
    #[arg(long)]
    pub skip_funding: bool,

    /// Dry-run: show funding plan without executing.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompt.
    #[arg(long, short = 'y')]
    pub yes: bool,
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
pub async fn run(args: EcosystemDeployArgs, context: &Context) -> Result<()> {
    log::debug!("Starting ecosystem deployment");

    // 1. Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(&args, context)?;
    log::info!("Deploying ecosystem: {}", ecosystem_name.green());

    // 2. Resolve chain name
    let chain_name = resolve_chain_name(&args, context)?;
    log::info!("Chain: {}", chain_name.green());

    // 3. Create state manager and validate ecosystem exists
    let state_manager = create_state_manager(&ecosystem_name, context)?;
    validate_ecosystem_exists(&state_manager, &ecosystem_name).await?;

    // 4. Validate chain exists
    validate_chain_exists(&state_manager, &chain_name, &ecosystem_name).await?;

    // 5. Skip funding if requested
    if args.skip_funding {
        log::info!("Skipping wallet funding (--skip-funding)");
        log::info!("Ecosystem deployment complete (funding skipped)");
        log::info!("Note: Contract deployment not yet implemented");
        return Ok(());
    }

    // 6. Resolve RPC URL (args > config)
    let rpc_url = resolve_rpc_url(&args, context)?;
    log::info!("Settlement layer RPC: {}", rpc_url.to_string().green());

    // 7. Get funder key (args > config)
    let funder_key = resolve_funder_key(&args, context)?;
    log::debug!("Funder key resolved");

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

    log::info!(
        "Loaded wallets: ecosystem={}, chain={}",
        count_wallets(&ecosystem_wallets),
        count_wallets(&chain_wallets)
    );

    // 9. Create executor with logging handler (needed for provider)
    let executor = FundingExecutor::new(rpc_url.as_str(), &funder_key)
        .wrap_err("Failed to create funding executor")?
        .with_event_handler(Arc::new(LoggingEventHandler));

    let funder_address = executor.funder_address();
    log::info!("Funder address: {}", green_address(funder_address));

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
    log::info!("Building funding plan...");
    let plan_result = FundingPlanBuilder::new(executor.provider(), &funding_config, funder_address)
        .with_ecosystem_wallets(&ecosystem_wallets)
        .with_chain_wallets(&chain_wallets)
        .build()
        .await;

    let plan = match plan_result {
        Ok(p) => p,
        Err(FundingError::NoFundingRequired) => {
            log::info!("All wallets already funded - no funding required!");
            log::info!("Ecosystem deployment complete (no funding needed)");
            log::info!("Note: Contract deployment not yet implemented");
            return Ok(());
        }
        Err(e) => return Err(e).wrap_err("Failed to build funding plan"),
    };

    // 13. Display plan summary
    log::info!("");
    log::info!("============================================================");
    log::info!("Funding Plan Summary");
    log::info!("============================================================");
    log::info!("  Transfers needed: {}", plan.transfer_count());
    log::info!(
        "  Total ETH to transfer: {}",
        green_eth(plan.total_eth_transfers())
    );
    if !plan.total_token_required.is_zero() {
        let symbol = funding_config.token_symbol.as_deref().unwrap_or("tokens");
        log::info!(
            "  Total {} to transfer: {}",
            symbol,
            green_token(plan.total_token_required, symbol)
        );
    }
    log::info!("  Estimated gas cost: {}", green_eth(plan.total_gas_cost));
    log::info!(
        "  Total ETH required: {}",
        green_eth(plan.total_eth_required)
    );
    log::info!("");
    log::info!("  Funder balance: {}", green_eth(plan.funder_eth_balance));
    if let Some(token_balance) = plan.funder_token_balance {
        let symbol = funding_config.token_symbol.as_deref().unwrap_or("tokens");
        log::info!(
            "  Funder {} balance: {}",
            symbol,
            green_token(token_balance, symbol)
        );
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
    log::info!("  Status: Sufficient balance");
    log::info!("============================================================");

    // 14. Dry-run mode - show plan without executing
    if args.dry_run {
        log::info!("");
        log::info!("Dry-run mode: funding plan created but not executed");
        display_plan_details(&plan);
        return Ok(());
    }

    // 15. Confirmation prompt (unless --yes)
    if !args.yes {
        log::info!("");
        let confirmed = Confirm::new()
            .with_prompt("Proceed with funding?")
            .default(false)
            .interact()
            .wrap_err("Failed to read confirmation")?;

        if !confirmed {
            log::info!("Funding cancelled by user");
            return Ok(());
        }
    }

    // 16. Execute funding
    log::info!("");
    log::info!("Executing funding transfers...");
    let result = executor
        .execute(&plan)
        .await
        .wrap_err("Funding execution failed")?;

    log::info!("");
    log::info!("============================================================");
    log::info!("Funding Complete!");
    log::info!("============================================================");
    log::info!("  Successful transfers: {}", result.successful);
    log::info!("  Total gas used: {}", result.total_gas_used);
    log::info!("============================================================");

    log::info!("");
    log::info!("Ecosystem wallets funded successfully!");
    log::info!("Note: Contract deployment not yet implemented");

    Ok(())
}

/// Resolve ecosystem name from args or config.
fn resolve_ecosystem_name(args: &EcosystemDeployArgs, context: &Context) -> Result<String> {
    args.ecosystem_name
        .clone()
        .or_else(|| Some(context.config().ecosystem.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem name required: use --ecosystem-name or set in config")
        })
}

/// Resolve chain name from args or config.
fn resolve_chain_name(args: &EcosystemDeployArgs, context: &Context) -> Result<String> {
    args.chain_name
        .clone()
        .or_else(|| Some(context.config().ecosystem.chain_name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| eyre::eyre!("Chain name required: use --chain-name or set in config"))
}

/// Create state manager for the ecosystem.
fn create_state_manager(ecosystem_name: &str, context: &Context) -> Result<StateManager> {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    Ok(StateManager::with_backend_type(
        context.config().state_backend.clone(),
        &ecosystem_path,
    ))
}

/// Validate that ecosystem state exists.
async fn validate_ecosystem_exists(
    state_manager: &StateManager,
    ecosystem_name: &str,
) -> Result<()> {
    if !state_manager.exists().await? {
        return Err(eyre::eyre!(
            "Ecosystem '{}' not found. Run 'adi init ecosystem' first.",
            ecosystem_name
        ));
    }
    log::debug!("Ecosystem '{}' exists", ecosystem_name);
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
            "Chain '{}' not found in ecosystem '{}'. Run 'adi init chain' first.",
            chain_name,
            ecosystem_name
        ));
    }
    log::debug!("Chain '{}' exists", chain_name);
    Ok(())
}

/// Resolve RPC URL from args or config.
fn resolve_rpc_url(args: &EcosystemDeployArgs, context: &Context) -> Result<Url> {
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
fn resolve_funder_key(args: &EcosystemDeployArgs, context: &Context) -> Result<SecretString> {
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
    args: &EcosystemDeployArgs,
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
    log::debug!("Gas multiplier: {}%", multiplier);

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
            log::warn!(
                "Chain metadata has ETH as base_token, but config specifies {}. \
                 Using config value (zkstack may have ignored --base-token-address).",
                config_token
            );
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
                log::warn!("Failed to query token symbol: {}", e);
                None
            }
        };
        config = config.with_token(address, symbol.clone());
        log::info!(
            "Custom gas token: {} ({})",
            green_address(address),
            symbol.as_deref().unwrap_or("unknown").green()
        );
    }

    Ok(config)
}

/// Build DefaultAmounts from CLI args and config, falling back to library defaults.
///
/// Priority: CLI args > config file > library defaults
fn build_default_amounts(
    args: &EcosystemDeployArgs,
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
    log::info!("");
    log::info!("============================================================");
    log::info!("Current Wallet Balances");
    log::info!("============================================================");

    // Ecosystem wallets
    log::info!("Ecosystem Wallets:");
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
    log::info!("");
    log::info!("Chain Wallets ({}):", chain_name);
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

    log::info!("============================================================");

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

    let eth_str = format_eth(balance.eth_balance).green();
    let symbol = token_symbol.unwrap_or("tokens");
    let token_str = balance
        .token_balance
        .map(|t| format!(" + {} {}", format_token(t).green(), symbol.green()))
        .unwrap_or_default();

    log::info!(
        "  {:24} ({}): {} {}{}",
        role,
        green_address(w.address),
        eth_str,
        "ETH".green(),
        token_str
    );

    Ok(())
}

/// Display detailed funding plan (for dry-run mode).
fn display_plan_details(plan: &adi_funding::FundingPlan) {
    log::info!("");
    log::info!("Planned Transfers:");
    for (i, transfer) in plan.transfers.iter().enumerate() {
        let amount_str = match &transfer.transfer_type {
            adi_funding::TransferType::Eth { amount } => green_eth(*amount),
            adi_funding::TransferType::Token { amount, symbol, .. } => green_token(*amount, symbol),
        };
        log::info!(
            "  [{}] {:24} -> {}  ({})",
            i + 1,
            transfer.role,
            green_address(transfer.to),
            amount_str
        );
    }
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

/// Format ETH amount with green color.
fn green_eth(wei: U256) -> String {
    format!("{} {}", format_eth(wei).green(), "ETH".green())
}

/// Format token amount with green color.
fn green_token(amount: U256, symbol: &str) -> String {
    format!("{} {}", format_token(amount).green(), symbol.green())
}

/// Format address with green color.
fn green_address(address: Address) -> String {
    address.to_string().green().to_string()
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
