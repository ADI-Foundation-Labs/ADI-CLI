//! Refund command — returns ETH and optional ERC20 tokens from ecosystem/chain wallets.

mod display;
mod wallets;

use std::sync::Arc;

use adi_funding::{
    build_refund_plan, execute_refund, FundingEventHandler, RefundConfig, SpinnerEventHandler,
    WalletSource,
};
use alloy_primitives::Address;
use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

use self::display::{display_plan_summary, display_results, BalanceCheckHandler};
use self::wallets::{collect_wallet_entries, count_wallets};
use crate::commands::helpers::{
    create_state_manager_with_s3, derive_address_from_key, resolve_ecosystem_name, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for the refund command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct RefundArgs {
    /// Address to receive refunded funds (falls back to funder address from config).
    #[arg(long)]
    pub receiver: Option<Address>,

    /// Ecosystem name (falls back to config).
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// Settlement layer JSON-RPC URL (falls back to config).
    #[arg(long, env = "ADI_RPC_URL")]
    pub rpc_url: Option<Url>,

    /// Refund specific chain only (default: all chains).
    #[arg(long)]
    pub chain: Option<String>,

    /// ERC20 token address to also refund.
    #[arg(long)]
    pub token_address: Option<Address>,

    /// Gas price multiplier percentage (falls back to config, default: 200 = 2x safety).
    #[arg(long)]
    pub gas_multiplier: Option<u64>,

    /// Skip confirmation prompt.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

/// Execute the refund command.
pub async fn run(args: RefundArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Refund")?;

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    let gas_multiplier = args
        .gas_multiplier
        .unwrap_or_else(|| context.config().gas_multiplier);
    let receiver = resolve_receiver(args.receiver, context)?;

    let logger = context.logger();
    logger.debug(&format!("Ecosystem: {ecosystem_name}"));
    logger.debug(&format!("RPC URL: {rpc_url}"));
    logger.debug(&format!("Receiver: {receiver}"));
    logger.debug(&format!("Gas multiplier: {gas_multiplier}%"));

    // Load state
    let (state_manager, _s3_control) =
        create_state_manager_with_s3(&ecosystem_name, context).await?;

    // Collect wallet entries
    let mut entries = Vec::new();

    let eco_wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;

    logger.debug(&format!(
        "Loaded ecosystem wallets: {} present",
        count_wallets(&eco_wallets)
    ));
    collect_wallet_entries(&eco_wallets, WalletSource::Ecosystem, None, &mut entries);

    // Chain wallets + auto-detect CGT
    let chain_names = resolve_chain_names(&args, &state_manager, &ecosystem_name).await?;
    let mut token_address = args.token_address;

    for chain_name in &chain_names {
        let chain_ops = state_manager.chain(chain_name);

        if token_address.is_none() {
            if let Ok(metadata) = chain_ops.metadata().await {
                if !metadata.base_token.is_eth() {
                    logger.debug(&format!(
                        "Detected custom gas token {} from chain '{chain_name}'",
                        metadata.base_token.address
                    ));
                    token_address = Some(metadata.base_token.address);
                }
            }
        }

        let chain_wallets = chain_ops
            .wallets()
            .await
            .wrap_err_with(|| format!("Failed to load wallets for chain '{chain_name}'"))?;

        logger.debug(&format!(
            "Loaded chain '{chain_name}' wallets: {} present",
            count_wallets(&chain_wallets)
        ));
        collect_wallet_entries(
            &chain_wallets,
            WalletSource::Chain,
            Some(chain_name.as_str()),
            &mut entries,
        );
    }

    if entries.is_empty() {
        ui::warning("No wallets found in ecosystem state")?;
        ui::outro("Nothing to refund")?;
        return Ok(());
    }

    ui::info(format!(
        "Found {} wallet(s) across ecosystem + {} chain(s)",
        entries.len(),
        chain_names.len()
    ))?;

    // Build refund plan
    let provider =
        adi_funding::FundingProvider::new(rpc_url.as_str()).wrap_err("Failed to connect to RPC")?;

    let config = RefundConfig {
        receiver,
        token_address,
        gas_multiplier,
    };

    let balance_handler: Arc<dyn FundingEventHandler> = Arc::new(BalanceCheckHandler::new());

    let plan = build_refund_plan(
        &provider,
        &entries,
        &config,
        logger.as_ref(),
        &balance_handler,
    )
    .await
    .wrap_err("Failed to build refund plan")?;

    if plan.is_empty() {
        ui::warning("All wallets have zero or insufficient balance to cover gas")?;
        ui::outro("Nothing to refund")?;
        return Ok(());
    }

    display_plan_summary(&plan)?;

    if !args.yes {
        let confirmed = ui::confirm("Proceed with refund?")
            .initial_value(true)
            .interact()?;
        if !confirmed {
            ui::outro_cancel("Refund cancelled")?;
            return Ok(());
        }
    }

    // Execute
    let event_handler: Arc<dyn FundingEventHandler> = Arc::new(SpinnerEventHandler::new());

    let result = execute_refund(rpc_url.as_str(), &plan, &event_handler, logger.as_ref())
        .await
        .wrap_err("Refund execution failed")?;

    display_results(&result, &plan)?;

    if result.tx_hashes.is_empty() {
        ui::outro("No transfers were executed")?;
    } else {
        ui::outro("Refund complete")?;
    }

    Ok(())
}

/// Resolve receiver address from CLI arg or funder key in config.
fn resolve_receiver(arg_value: Option<Address>, context: &Context) -> Result<Address> {
    if let Some(addr) = arg_value {
        return Ok(addr);
    }

    let funder_key = context
        .config()
        .funding
        .funder_key
        .clone()
        .ok_or_else(|| {
            eyre::eyre!(
                "Receiver required: use --receiver flag, or set funding.funder_key / ADI_FUNDER_KEY so the funder address is used"
            )
        })?;

    derive_address_from_key(&funder_key).wrap_err("Failed to derive funder address from key")
}

/// Resolve chain names from args or state.
async fn resolve_chain_names(
    args: &RefundArgs,
    state_manager: &adi_state::StateManager,
    ecosystem_name: &str,
) -> Result<Vec<String>> {
    if let Some(ref chain) = args.chain {
        let chains = state_manager.list_chains().await?;
        if !chains.contains(chain) {
            return Err(eyre::eyre!(
                "Chain '{}' not found in ecosystem '{}'. Available: {}",
                chain,
                ecosystem_name,
                chains.join(", ")
            ));
        }
        return Ok(vec![chain.clone()]);
    }

    Ok(state_manager.list_chains().await?)
}
