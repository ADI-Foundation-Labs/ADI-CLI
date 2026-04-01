//! Configuration resolution for the verify command.

use adi_ecosystem::verification::{
    ContractRegistry, ExplorerClient, ExplorerConfig, ExplorerType, VerificationTarget,
};
use alloy_provider::Provider;
use std::sync::Arc;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_name, resolve_ecosystem_name,
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
    // Early check for local network
    let rpc_url = args
        .rpc_url
        .as_ref()
        .or(context.config().funding.rpc_url.as_ref());

    if let Some(url) = rpc_url {
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

    // RPC enhancement
    let rpc_url = get_rpc_url(args, context);
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
    let explorer_type = resolve_explorer_type(args, context);
    let api_key = resolve_api_key(args, context);
    let explorer_url = resolve_explorer_url(args, explorer_type, chain_id, context)?;

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

/// Get RPC URL from args or config.
fn get_rpc_url(args: &VerifyArgs, context: &Context) -> Option<Url> {
    args.rpc_url
        .clone()
        .or_else(|| context.config().ecosystem.rpc_url.clone())
        .or_else(|| context.config().funding.rpc_url.clone())
}

/// Resolve chain ID from args or RPC.
async fn resolve_chain_id(args: &VerifyArgs, context: &Context) -> Result<u64> {
    if let Some(chain_id) = args.chain_id {
        return Ok(chain_id);
    }

    // Try CLI arg RPC
    if let Some(ref rpc_url) = args.rpc_url {
        return fetch_chain_id(rpc_url, "RPC", context).await;
    }

    // Try ecosystem config RPC
    if let Some(ref rpc_url) = context.config().ecosystem.rpc_url {
        return fetch_chain_id(rpc_url, "ecosystem config RPC", context).await;
    }

    // Try funding config RPC (backward compatibility)
    if let Some(ref rpc_url) = context.config().funding.rpc_url {
        return fetch_chain_id(rpc_url, "funding config RPC", context).await;
    }

    Err(eyre::eyre!(
        "Chain ID required. Provide --chain-id or --rpc-url"
    ))
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

/// Resolve API key from args, env, or config.
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
    // CLI arg takes priority
    if let Some(ref url) = args.explorer_url {
        validate_blockscout_url(url, explorer_type)?;
        return Ok(url.clone());
    }

    // Fall back to config
    if let Some(ref url) = context.config().verification.explorer_url {
        return Ok(url.clone());
    }

    // Default URL for known explorers
    if let Some(url) = ExplorerConfig::default_api_url(explorer_type, chain_id) {
        return Ok(url);
    }

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

/// Validate Blockscout URLs to catch common mistakes.
fn validate_blockscout_url(url: &Url, explorer_type: ExplorerType) -> Result<()> {
    if explorer_type != ExplorerType::Blockscout {
        return Ok(());
    }

    let url_str = url.as_str();
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

    Ok(())
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
