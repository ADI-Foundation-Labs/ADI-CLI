//! Configuration resolution for the verify command.

use adi_ecosystem::verification::{
    ContractRegistry, ExplorerClient, ExplorerConfig, VerificationTarget,
};
use alloy_provider::Provider;
use std::sync::Arc;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_name, resolve_ecosystem_name,
    resolve_explorer_api_key, resolve_explorer_type, resolve_explorer_url, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use super::VerifyArgs;

/// Resolved configuration bundling all data needed by check and submit phases.
pub(super) struct VerifyConfig<'a> {
    pub ecosystem_name: String,
    pub effective_chain_name: Option<String>,
    pub explorer_client: Arc<ExplorerClient>,
    pub targets: Vec<VerificationTarget>,
    pub context: &'a Context,
}

/// Resolve all configuration from args and context.
/// Returns `None` if verification should be skipped (local network).
pub(super) async fn resolve_config<'a>(
    args: &VerifyArgs,
    context: &'a Context,
) -> Result<Option<VerifyConfig<'a>>> {
    // Resolve the RPC URL once (arg > ecosystem.rpc_url).
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config()).ok();

    // Early exit for local networks (verification is unsupported there).
    if let Some(ref url) = rpc_url {
        if is_local_network_url(url) {
            ui::outro_cancel(
                "Contract verification is not available for local networks (Anvil, Hardhat, etc.)",
            )?;
            return Ok(None);
        }
    }

    // Load contracts
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let state_manager = create_state_manager_with_context(&ecosystem_name, context)?;

    let mut ecosystem_contracts =
        super::contracts::load_ecosystem_contracts(&state_manager, context.logger()).await?;

    let effective_chain_name = args
        .chain
        .clone()
        .or_else(|| resolve_chain_name(args.chain.as_ref(), context.config()).ok());

    let mut chain_contracts = super::contracts::load_chain_contracts(
        effective_chain_name.as_deref(),
        &state_manager,
        context.logger(),
    )
    .await;

    // RPC enhancement (non-local networks only).
    if let Some(ref url) = rpc_url {
        if !is_local_network_url(url) {
            super::contracts::enhance_from_rpc(
                url,
                &mut ecosystem_contracts,
                &mut chain_contracts,
                Arc::clone(context.logger()),
            )
            .await;
        }
    }

    // Resolve explorer configuration
    let chain_id = resolve_chain_id(args, context).await?;
    let explorer_type = resolve_explorer_type(args.explorer, context.config());
    let api_key = resolve_explorer_api_key(args.api_key.as_deref(), context.config());
    let explorer_url = resolve_explorer_url(
        args.explorer_url.as_ref(),
        explorer_type,
        chain_id,
        context.config(),
    )?;

    // Build targets
    let targets = build_targets(args, &ecosystem_contracts, chain_contracts.as_ref())?;

    // Create explorer client
    let explorer_config = ExplorerConfig::new(explorer_type, explorer_url, api_key, chain_id);
    let explorer_client = ExplorerClient::new(explorer_config, Arc::clone(context.logger()))
        .map_err(|e| eyre::eyre!("Failed to create explorer client: {}", e))?;

    Ok(Some(VerifyConfig {
        ecosystem_name,
        effective_chain_name,
        explorer_client: Arc::new(explorer_client),
        targets,
        context,
    }))
}

/// Display the resolved verification configuration.
pub(super) fn display_config(config: &VerifyConfig<'_>, args: &VerifyArgs) -> Result<()> {
    ui::note(
        "Verification configuration",
        format!(
            "Ecosystem: {}\nChain: {}\nExplorer: {}\nAPI URL: {}\nChain ID: {}\nMode: {}",
            ui::green(&config.ecosystem_name),
            config
                .effective_chain_name
                .as_ref()
                .map_or_else(|| ui::dim("not specified"), |n| ui::green(n)),
            ui::green(&config.explorer_client.config().explorer_type.to_string()),
            ui::green(&config.explorer_client.config().api_url.to_string()),
            ui::green(config.explorer_client.config().chain_id),
            if args.submit {
                ui::cyan("submit")
            } else {
                ui::dim("status check")
            }
        ),
    )?;
    Ok(())
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

/// Resolve chain ID from args or RPC.
async fn resolve_chain_id(args: &VerifyArgs, context: &Context) -> Result<u64> {
    if let Some(chain_id) = args.chain_id {
        return Ok(chain_id);
    }

    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())
        .map_err(|_| eyre::eyre!("Chain ID required. Provide --chain-id or --rpc-url"))?;
    fetch_chain_id(&rpc_url, "RPC", context).await
}

/// Fetch chain ID from a provider, logging the source.
async fn fetch_chain_id(rpc_url: &Url, source: &str, context: &Context) -> Result<u64> {
    context
        .logger()
        .debug(&format!("Fetching chain ID from {}...", source));
    let provider = alloy_provider::ProviderBuilder::new().connect_http(rpc_url.clone());
    let chain_id = provider
        .get_chain_id()
        .await
        .wrap_err(format!("Failed to get chain ID from {}", source))?;
    context.logger().debug(&format!("Chain ID: {}", chain_id));
    Ok(chain_id)
}

/// Build verification targets based on command flags.
fn build_targets(
    args: &VerifyArgs,
    ecosystem_contracts: &adi_types::EcosystemContracts,
    chain_contracts: Option<&adi_types::ChainContracts>,
) -> Result<Vec<VerificationTarget>> {
    let mut targets = Vec::new();

    if args.ecosystem || args.chain.is_none() {
        targets.extend(ContractRegistry::build_ecosystem_targets(
            ecosystem_contracts,
        ));
    }

    if let Some(chain) = chain_contracts {
        targets.extend(ContractRegistry::build_chain_targets(chain));
    }

    Ok(targets)
}
