//! Display server parameters for Docker Compose configuration.

use std::collections::HashMap;

pub use adi_server_params::ServerVersion;
use adi_server_params::{display_value, extract, ServerParam, ServerParamsInput};
use clap::Args;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_chain_name, resolve_ecosystem_name, resolve_rpc_url,
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

    /// ZkSync OS server version to generate parameters for.
    #[arg(long, value_enum, default_value_t = ServerVersion::V0211)]
    pub server_version: ServerVersion,

    /// Output as JSON instead of formatted text.
    #[arg(long)]
    pub json: bool,

    /// Upload generated parameters to HashiCorp Vault.
    #[arg(long)]
    pub upload: bool,

    /// Vault auth token (prompted interactively if omitted).
    #[arg(long, requires = "upload")]
    #[serde(skip)]
    pub vault_token: Option<SecretString>,

    /// Full Vault API path (e.g. /v1/Adi-chain/data/Adi-chain/adi/devnet1/server).
    /// Prompted interactively if omitted.
    #[arg(long, requires = "upload")]
    pub vault_path: Option<String>,
}

/// Execute the server-params command.
pub async fn run(args: &ServerParamsArgs, context: &Context) -> Result<()> {
    if !args.json {
        ui::intro("Server Parameters")?;
    }

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let chain_name = resolve_chain_name(args.chain.as_ref(), context.config())?;

    if !args.json {
        ui::info(format!("Ecosystem: {}", ui::green(&ecosystem_name)))?;
        ui::info(format!("Chain: {}", ui::green(&chain_name)))?;
        ui::info(format!(
            "Server Version: {}",
            ui::green(&args.server_version.to_string())
        ))?;
    }

    let state_manager = create_state_manager_with_context(&ecosystem_name, context)?;

    if !state_manager
        .exists()
        .await
        .wrap_err("Failed to check ecosystem")?
    {
        return handle_missing(
            args.json,
            &format!(
                "Ecosystem '{}' not found. Run 'adi init' first.",
                ecosystem_name
            ),
        );
    }

    let chain_ops = state_manager.chain(&chain_name);
    if !chain_ops.exists().await.wrap_err("Failed to check chain")? {
        return handle_missing(args.json, &format!("Chain '{}' not found.", chain_name));
    }

    if !chain_ops
        .contracts_exist()
        .await
        .wrap_err("Failed to check chain contracts")?
    {
        return handle_missing(
            args.json,
            &format!(
                "No contracts deployed for chain '{}'. Run 'adi deploy' first.",
                chain_name
            ),
        );
    }

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

    // RPC URL is optional for server params (only emitted when configured).
    let rpc_url = resolve_rpc_url(None, context.config())
        .ok()
        .map(|url| url.to_string());

    let pubdata_mode = context
        .config()
        .ecosystem
        .get_chain(&chain_name)
        .map(|c| c.pubdata_mode)
        .unwrap_or_default();

    // Settlement layer is ecosystem-level (all chains share the settlement RPC).
    // It drives the L1-sender fee tier, orthogonally to the DA pubdata mode.
    let settlement = context.config().ecosystem.settlement;

    let prover_mode = chain_metadata.prover_version;

    let genesis_base64 = if args.json || args.upload {
        chain_ops.genesis_base64().await.ok()
    } else {
        None
    };

    let fee_collector_address = context
        .config()
        .ecosystem
        .get_chain(&chain_name)
        .and_then(|c| c.fee_collector_address)
        .or_else(|| wallets.fee_account.as_ref().map(|w| w.address));

    let base_token_address = context
        .config()
        .ecosystem
        .get_chain(&chain_name)
        .and_then(|c| c.base_token_address);

    let input = ServerParamsInput {
        contracts: &contracts,
        wallets: &wallets,
        chain_metadata: &chain_metadata,
        rpc_url: rpc_url.as_deref(),
        pubdata_mode,
        settlement,
        prover_mode,
        genesis_base64,
        fee_collector_address,
        base_token_address,
        server_version: args.server_version,
    };
    let params_list = extract(&input);

    if args.json {
        let json_output: HashMap<&str, Option<serde_json::Value>> = params_list
            .iter()
            .map(|p| (p.env_name, p.value.clone()))
            .collect();
        let json_str =
            serde_json::to_string_pretty(&json_output).wrap_err("Failed to serialize to JSON")?;
        println!("{json_str}");
    } else {
        let output = format_params(&params_list);
        ui::note("Docker Compose Environment Variables", &output)?;
        ui::outro("")?;
    }

    if args.upload {
        let json_output: HashMap<&str, Option<serde_json::Value>> = params_list
            .iter()
            .map(|p| (p.env_name, p.value.clone()))
            .collect();
        super::vault_upload::run(args, context, &json_output).await?;
    }

    Ok(())
}

/// Handle missing ecosystem/chain/contracts.
fn handle_missing(json: bool, msg: &str) -> Result<()> {
    if json {
        return Err(eyre::eyre!("{}", msg));
    }
    ui::warning(msg)?;
    ui::outro("")?;
    Ok(())
}

/// Format parameters for display.
fn format_params(params: &[ServerParam]) -> String {
    params
        .iter()
        .map(|p| {
            let value_display = if p.env_name == "genesis" {
                ui::green("<base64, hidden>").to_string()
            } else {
                match &p.value {
                    Some(v) => ui::green(&display_value(v)).to_string(),
                    None => ui::yellow("not available").to_string(),
                }
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
