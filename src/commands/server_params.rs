//! Display server parameters for Docker Compose configuration.

use std::collections::HashMap;

use adi_types::{ChainContracts, ChainMetadata, Wallets};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_name, resolve_ecosystem_name,
};
use crate::config::Config;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Fusaka upgrade timestamp (hardcoded).
const FUSAKA_UPGRADE_TIMESTAMP: u64 = 1771883505;

/// Arguments for `server-params` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct ServerParamsArgs {
    /// Ecosystem name (falls back to config if not provided).
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// Chain name (falls back to config if not provided).
    #[arg(long)]
    pub chain: Option<String>,

    /// Output as JSON instead of formatted text.
    #[arg(long)]
    pub json: bool,
}

/// Server parameter with its environment variable name and value.
struct ServerParam {
    env_name: &'static str,
    value: Option<String>,
    description: &'static str,
}

/// Execute the server-params command.
pub async fn run(args: &ServerParamsArgs, context: &Context) -> Result<()> {
    if !args.json {
        ui::intro("Server Parameters")?;
    }

    // Resolve configuration
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let chain_name = resolve_chain_name(args.chain.as_ref(), context.config())?;

    if !args.json {
        ui::info(format!("Ecosystem: {}", ui::green(&ecosystem_name)))?;
        ui::info(format!("Chain: {}", ui::green(&chain_name)))?;
    }

    // Create state manager
    let state_manager = create_state_manager_with_context(&ecosystem_name, context)?;

    // Check if ecosystem exists
    if !state_manager
        .exists()
        .await
        .wrap_err("Failed to check ecosystem")?
    {
        if args.json {
            return Err(eyre::eyre!(
                "Ecosystem '{}' not found. Run 'adi init' first.",
                ecosystem_name
            ));
        }
        ui::warning(format!(
            "Ecosystem '{}' not found. Run 'adi init' first.",
            ecosystem_name
        ))?;
        ui::outro("")?;
        return Ok(());
    }

    // Check if chain exists
    let chain_ops = state_manager.chain(&chain_name);
    if !chain_ops.exists().await.wrap_err("Failed to check chain")? {
        if args.json {
            return Err(eyre::eyre!("Chain '{}' not found.", chain_name));
        }
        ui::warning(format!("Chain '{}' not found.", chain_name))?;
        ui::outro("")?;
        return Ok(());
    }

    // Check if chain contracts exist
    if !chain_ops
        .contracts_exist()
        .await
        .wrap_err("Failed to check chain contracts")?
    {
        if args.json {
            return Err(eyre::eyre!(
                "No contracts deployed for chain '{}'. Run 'adi deploy' first.",
                chain_name
            ));
        }
        ui::warning(format!(
            "No contracts deployed for chain '{}'. Run 'adi deploy' first.",
            chain_name
        ))?;
        ui::outro("")?;
        return Ok(());
    }

    // Load chain contracts, wallets, and metadata
    let contracts = chain_ops
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts")?;

    let wallets = chain_ops
        .wallets()
        .await
        .wrap_err("Failed to load chain wallets")?;

    let chain_metadata = chain_ops
        .metadata()
        .await
        .wrap_err("Failed to load chain metadata")?;

    // Resolve RPC URL from config
    let rpc_url = resolve_rpc_url(context.config());

    // Determine blobs mode from config
    let blobs = context
        .config()
        .ecosystem
        .get_chain(&chain_name)
        .map(|c| c.blobs)
        .unwrap_or(false);

    // Extract parameters
    let params = extract_server_params(
        &contracts,
        &wallets,
        &chain_metadata,
        rpc_url.as_deref(),
        blobs,
    );

    if args.json {
        // Output as JSON
        let json_output: HashMap<&str, Option<String>> = params
            .iter()
            .map(|p| (p.env_name, p.value.clone()))
            .collect();
        let json_str =
            serde_json::to_string_pretty(&json_output).wrap_err("Failed to serialize to JSON")?;
        println!("{json_str}");
    } else {
        // Output formatted text
        let output = format_params(&params);
        ui::note("Docker Compose Environment Variables", &output)?;
        ui::outro("")?;
    }

    Ok(())
}

/// Resolve RPC URL from config with fallback.
///
/// Priority: ecosystem.rpc_url > funding.rpc_url
fn resolve_rpc_url(config: &Config) -> Option<String> {
    config
        .ecosystem
        .rpc_url
        .as_ref()
        .or(config.funding.rpc_url.as_ref())
        .map(|url| url.to_string())
}

/// Extract server parameters from contracts, wallets, and metadata.
fn extract_server_params(
    contracts: &ChainContracts,
    wallets: &Wallets,
    chain_metadata: &ChainMetadata,
    rpc_url: Option<&str>,
    blobs: bool,
) -> Vec<ServerParam> {
    let mut params = vec![
        // General parameters
        ServerParam {
            env_name: "general_l1_rpc_url",
            value: rpc_url.map(String::from),
            description: "Settlement layer RPC URL",
        },
        // Genesis parameters
        ServerParam {
            env_name: "genesis_chain_id",
            value: Some(chain_metadata.chain_id.to_string()),
            description: "Chain ID",
        },
        ServerParam {
            env_name: "genesis_bridgehub_address",
            value: contracts
                .ecosystem_contracts
                .as_ref()
                .and_then(|c| c.bridgehub_proxy_addr)
                .map(|addr| format!("{addr}")),
            description: "Bridgehub proxy contract address",
        },
        ServerParam {
            env_name: "genesis_bytecode_supplier_address",
            value: contracts
                .ecosystem_contracts
                .as_ref()
                .and_then(|c| c.l1_bytecodes_supplier_addr)
                .map(|addr| format!("{addr}")),
            description: "L1 bytecodes supplier contract address",
        },
        // L1 sender parameters
        ServerParam {
            env_name: "l1_sender_fusaka_upgrade_timestamp",
            value: Some(FUSAKA_UPGRADE_TIMESTAMP.to_string()),
            description: "Fusaka upgrade timestamp",
        },
        ServerParam {
            env_name: "l1_sender_operator_commit_pk",
            value: wallets
                .operator
                .as_ref()
                .map(|w| w.expose_private_key().to_string()),
            description: "Operator private key (commit batches)",
        },
        ServerParam {
            env_name: "l1_sender_operator_prove_pk",
            value: wallets
                .prove_operator
                .as_ref()
                .map(|w| w.expose_private_key().to_string()),
            description: "Prove operator private key",
        },
        ServerParam {
            env_name: "l1_sender_operator_execute_pk",
            value: wallets
                .execute_operator
                .as_ref()
                .map(|w| w.expose_private_key().to_string()),
            description: "Execute operator private key",
        },
    ];

    // Add calldata-specific parameters when blobs is disabled (L3 mode)
    if !blobs {
        params.extend([
            ServerParam {
                env_name: "l1_sender_pubdata_mode",
                value: Some("Calldata".to_string()),
                description: "Pubdata sending mode (Calldata for L3)",
            },
            ServerParam {
                env_name: "l1_sender_max_fee_per_gas_gwei",
                value: Some("1500".to_string()),
                description: "Max fee per gas in gwei",
            },
            ServerParam {
                env_name: "l1_sender_max_priority_fee_per_gas_gwei",
                value: Some("1500".to_string()),
                description: "Max priority fee per gas in gwei",
            },
            ServerParam {
                env_name: "sequencer_base_fee_override",
                value: Some("0x3e8".to_string()),
                description: "Sequencer base fee override",
            },
            ServerParam {
                env_name: "sequencer_pubdata_price_override",
                value: Some("0x1".to_string()),
                description: "Sequencer pubdata price override",
            },
            ServerParam {
                env_name: "sequencer_native_price_override",
                value: Some("0x1".to_string()),
                description: "Sequencer native price override",
            },
        ]);
    }

    params
}

/// Format parameters for display.
fn format_params(params: &[ServerParam]) -> String {
    params
        .iter()
        .map(|p| {
            let value_display = match &p.value {
                Some(v) => ui::green(v).to_string(),
                None => ui::yellow("not available").to_string(),
            };
            format!(
                "{}: {}\n  # {}",
                ui::cyan(p.env_name),
                value_display,
                p.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
