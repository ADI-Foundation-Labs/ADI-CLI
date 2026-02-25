//! Display server parameters for Docker Compose configuration.

use adi_types::{ChainContracts, Wallets};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_name, resolve_ecosystem_name,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for `server-params` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct ServerParamsArgs {
    /// Ecosystem name (falls back to config if not provided).
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// Chain name (falls back to config if not provided).
    #[arg(long)]
    pub chain: Option<String>,
}

/// Server parameter with its environment variable name and value.
struct ServerParam {
    env_name: &'static str,
    value: Option<String>,
    description: &'static str,
}

/// Execute the server-params command.
pub async fn run(args: &ServerParamsArgs, context: &Context) -> Result<()> {
    ui::intro("Server Parameters")?;

    // Resolve configuration
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let chain_name = resolve_chain_name(args.chain.as_ref(), context.config())?;

    ui::info(format!("Ecosystem: {}", ui::green(&ecosystem_name)))?;
    ui::info(format!("Chain: {}", ui::green(&chain_name)))?;

    // Create state manager
    let state_manager = create_state_manager_with_context(&ecosystem_name, context);

    // Check if ecosystem exists
    if !state_manager
        .exists()
        .await
        .wrap_err("Failed to check ecosystem")?
    {
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
        ui::warning(format!(
            "No contracts deployed for chain '{}'. Run 'adi deploy' first.",
            chain_name
        ))?;
        ui::outro("")?;
        return Ok(());
    }

    // Load chain contracts and wallets
    let contracts = chain_ops
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts")?;

    let wallets = chain_ops
        .wallets()
        .await
        .wrap_err("Failed to load chain wallets")?;

    // Extract and display parameters
    let params = extract_server_params(&contracts, &wallets);
    let output = format_params(&params);

    ui::note("Docker Compose Environment Variables", &output)?;
    ui::outro("")?;

    Ok(())
}

/// Extract server parameters from contracts and wallets.
fn extract_server_params(contracts: &ChainContracts, wallets: &Wallets) -> Vec<ServerParam> {
    vec![
        ServerParam {
            env_name: "genesis_bridgehub_address",
            value: contracts
                .ecosystem_contracts
                .as_ref()
                .and_then(|c| c.bridgehub_proxy_addr)
                .map(|addr| format!("{}", addr)),
            description: "Bridgehub proxy contract address",
        },
        ServerParam {
            env_name: "genesis_bytecode_supplier_address",
            value: contracts
                .ecosystem_contracts
                .as_ref()
                .and_then(|c| c.l1_bytecodes_supplier_addr)
                .map(|addr| format!("{}", addr)),
            description: "L1 bytecodes supplier contract address",
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
    ]
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
