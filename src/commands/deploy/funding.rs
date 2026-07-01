//! Wallet funding flows for ecosystem deployment.

use adi_ecosystem::ChainFundingDefaults;
use adi_funding::{
    build_funding_target_statuses, get_token_symbol, normalize_rpc_url, AnvilFunder,
    DefaultAmounts, FundingConfig, FundingError, FundingExecutor, FundingPlanBuilder,
    LoggingEventHandler, SpinnerEventHandler,
};
use adi_state::StateManager;
use adi_types::{Wallets, ETH_TOKEN_ADDRESS};
use alloy_primitives::U256;
use std::sync::Arc;
use url::Url;

use crate::commands::helpers::{resolve_funder_key, resolve_gas_multiplier};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use super::args::DeployArgs;
use super::deployment::run_ecosystem_deployment;
use super::display::{
    display_anvil_funding_plan, display_funding_plan, display_funding_summary,
    display_plan_details, format_eth,
};

/// Run Anvil-specific funding flow.
///
/// Uses the well-known Anvil default account (first account) to fund
/// wallets. Checks current balances and only funds wallets that need more ETH.
pub async fn run_anvil_funding(
    args: &DeployArgs,
    context: &Context,
    state_manager: &StateManager,
    ecosystem_name: &str,
    chain_name: &str,
    rpc_url: &Url,
) -> Result<()> {
    ui::info("Using Anvil funding mode (localhost detected)")?;

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

    let funding_defaults = &context.config().funding;
    let chain_funding = context
        .config()
        .ecosystem
        .get_chain(chain_name)
        .map(|c| &c.funding);
    let amounts = build_default_amounts(args, funding_defaults, chain_funding);

    let funder = AnvilFunder::with_rpc(&normalize_rpc_url(rpc_url.as_str()))?
        .with_amounts(amounts)
        .with_event_handler(Arc::new(LoggingEventHandler::new(Arc::clone(
            context.logger(),
        ))));

    let spinner = cliclack::spinner();
    spinner.start("Checking wallet balances...");
    let targets = funder
        .get_funding_targets(&ecosystem_wallets, &chain_wallets)
        .await
        .wrap_err("Failed to check wallet balances")?;
    spinner.stop("Wallet balances checked");

    display_anvil_funding_plan(&targets)?;

    let needs_funding = targets.iter().filter(|t| t.needs_funding).count();
    let already_funded = targets.len() - needs_funding;

    if args.dry_run {
        ui::outro("Dry-run mode: funding plan created but not executed")?;
        return Ok(());
    }

    if needs_funding == 0 {
        ui::success("All wallets already funded, skipping funding step.")?;
    } else {
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

    if args.skip_deployment {
        ui::outro("Skipping contract deployment (--skip-deployment)")?;
        return Ok(());
    }

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

/// Run production (non-Anvil) funding and deployment flow.
pub async fn run_production_funding(
    args: &DeployArgs,
    context: &Context,
    state_manager: &StateManager,
    ecosystem_name: &str,
    chain_name: &str,
    rpc_url: &Url,
) -> Result<()> {
    let funder_key = resolve_funder_key(args.funder_key.as_deref(), context.config())?;
    context.logger().debug("Funder key resolved");

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

    let operators = state_manager
        .chain(chain_name)
        .operators()
        .await
        .unwrap_or_default();

    ui::info(format!(
        "Loaded wallets: ecosystem={}, chain={}",
        ui::green(count_wallets(&ecosystem_wallets)),
        ui::green(count_wallets(&chain_wallets))
    ))?;

    let executor = FundingExecutor::new(rpc_url.as_str(), &funder_key)
        .wrap_err("Failed to create funding executor")?
        .with_event_handler(Arc::new(SpinnerEventHandler::new()));

    let funder_address = executor.funder_address();
    ui::info(format!("Funder address: {}", ui::green(funder_address)))?;

    let funding_config = build_funding_config(
        args,
        context,
        rpc_url,
        state_manager,
        chain_name,
        executor.provider(),
    )
    .await?;

    let spinner = cliclack::spinner();
    spinner.start("Checking wallet balances...");
    let target_statuses = build_funding_target_statuses(
        executor.provider(),
        &funding_config,
        &ecosystem_wallets,
        &chain_wallets,
        Some(&operators),
    )
    .await
    .wrap_err("Failed to get funding target statuses")?;
    spinner.stop("Wallet balances checked");

    display_funding_plan(
        funder_address,
        &target_statuses,
        funding_config.token_symbol.as_deref(),
    )?;

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

    if let Some(plan) = plan {
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

        if args.dry_run {
            display_plan_details(&plan)?;
            ui::outro("Dry-run mode: funding plan created but not executed")?;
            return Ok(());
        }

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

    if args.skip_deployment {
        ui::outro("Skipping contract deployment (--skip-deployment)")?;
        return Ok(());
    }

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

/// Build `FundingConfig` from ecosystem metadata.
///
/// Token address is read from chain's base_token config.
/// Token symbol is queried via RPC if not provided.
/// Funding amounts are resolved from: CLI args > config file > library defaults.
pub async fn build_funding_config(
    args: &DeployArgs,
    context: &Context,
    rpc_url: &Url,
    state_manager: &StateManager,
    chain_name: &str,
    provider: &adi_funding::FundingProvider,
) -> Result<FundingConfig> {
    let funding_defaults = &context.config().funding;

    let mut config = FundingConfig::new(rpc_url.as_str());

    let multiplier = resolve_gas_multiplier(args.gas_multiplier, context.config());
    config = config.with_gas_multiplier(multiplier);
    context
        .logger()
        .debug(&format!("Gas multiplier: {}%", multiplier));

    let chain_funding = context
        .config()
        .ecosystem
        .get_chain(chain_name)
        .map(|c| &c.funding);
    let amounts = build_default_amounts(args, funding_defaults, chain_funding);
    config = config.with_amounts(amounts);

    let base_token = state_manager
        .chain(chain_name)
        .metadata()
        .await
        .wrap_err_with(|| format!("Failed to load chain '{}' metadata", chain_name))?
        .base_token;

    let chain_defaults = context.config().ecosystem.get_chain(chain_name);
    let token_address = if base_token.is_eth() {
        let config_token = chain_defaults.and_then(|c| c.base_token_address);
        if let Some(token) = config_token {
            if token != ETH_TOKEN_ADDRESS {
                // Config has custom token but chain metadata has ETH.
                // This happens when zkstack ignores --base-token-address.
                context.logger().warning(&format!(
                    "Chain metadata has ETH as base_token, but config specifies {}. \
                     Using config value (zkstack may have ignored --base-token-address).",
                    token
                ));
                Some(token)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        Some(base_token.address)
    };

    if let Some(address) = token_address {
        let symbol = match get_token_symbol(provider, address).await {
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

/// Build `DefaultAmounts` from CLI args, chain config, and ecosystem config.
///
/// Priority: CLI args > chain funding config > ecosystem funding config > library defaults.
pub fn build_default_amounts(
    args: &DeployArgs,
    config: &crate::config::FundingDefaults,
    chain_funding: Option<&ChainFundingDefaults>,
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
            .or_else(|| chain_funding.and_then(|cf| cf.operator_eth).map(eth_to_wei))
            .unwrap_or(defaults.operator_eth),
        blob_operator_eth: defaults.blob_operator_eth,
        prove_operator_eth: args
            .prove_operator_eth
            .map(eth_to_wei)
            .or_else(|| {
                chain_funding
                    .and_then(|cf| cf.prove_operator_eth)
                    .map(eth_to_wei)
            })
            .unwrap_or(defaults.prove_operator_eth),
        execute_operator_eth: args
            .execute_operator_eth
            .map(eth_to_wei)
            .or_else(|| {
                chain_funding
                    .and_then(|cf| cf.execute_operator_eth)
                    .map(eth_to_wei)
            })
            .unwrap_or(defaults.execute_operator_eth),
        fee_account_eth: defaults.fee_account_eth,
        token_multiplier_setter_eth: defaults.token_multiplier_setter_eth,
    }
}

/// Count non-None wallets in a `Wallets` struct.
pub fn count_wallets(wallets: &Wallets) -> usize {
    [
        wallets.deployer.is_some(),
        wallets.fee_account.is_some(),
        wallets.governor.is_some(),
        wallets.token_multiplier_setter.is_some(),
    ]
    .iter()
    .filter(|&&b| b)
    .count()
}

/// Convert ETH amount (f64) to wei (U256).
///
/// Handles negative values by clamping to zero.
pub fn eth_to_wei(eth: f64) -> U256 {
    if eth <= 0.0 {
        return U256::ZERO;
    }
    let wei = eth * 1e18;
    let clamped = wei.min(u128::MAX as f64);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let wei_u128 = clamped as u128;
    U256::from(wei_u128)
}
