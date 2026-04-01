//! Configuration resolution for the accept ownership command.

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use secrecy::SecretString;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, derive_address_from_key, resolve_ecosystem_name,
    resolve_rpc_url, select_chain_from_state, OwnershipScope,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use super::AcceptArgs;

/// Intermediate result from private key resolution.
struct ResolvedKey {
    private_key: SecretString,
    address: Address,
    is_governor_mode: bool,
}

/// Resolved accept configuration bundling all data needed by execution phases.
pub(super) struct AcceptConfig<'a> {
    pub rpc_url: Url,
    pub gas_multiplier: Option<u64>,
    pub private_key: SecretString,
    pub key_address: Address,
    pub is_governor_mode: bool,
    pub ecosystem_contracts: Option<EcosystemContracts>,
    pub chain_contracts: Option<ChainContracts>,
    pub chain_name: Option<String>,
    pub context: &'a Context,
}

/// Resolve all configuration: names, URLs, scope, contracts, key, gas.
pub(super) async fn resolve_config<'a>(
    args: &AcceptArgs,
    context: &'a Context,
) -> Result<AcceptConfig<'a>> {
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    let include_ecosystem = matches!(args.scope, OwnershipScope::Ecosystem | OwnershipScope::All);
    let include_chain = matches!(args.scope, OwnershipScope::Chain | OwnershipScope::All);

    let state_manager = create_state_manager_with_context(&ecosystem_name, context)?;

    let ecosystem_contracts = load_ecosystem_contracts(include_ecosystem, &state_manager).await?;
    let chain_name = resolve_chain_name(
        include_chain,
        args.chain.as_ref(),
        &state_manager,
        &ecosystem_name,
    )
    .await?;
    let chain_contracts = load_chain_contracts(chain_name.as_deref(), &state_manager).await?;

    let resolved_key = resolve_private_key(args, &state_manager, context).await?;
    let gas_multiplier = args
        .gas_multiplier
        .or(Some(context.config().gas_multiplier));

    Ok(AcceptConfig {
        rpc_url,
        gas_multiplier,
        private_key: resolved_key.private_key,
        key_address: resolved_key.address,
        is_governor_mode: resolved_key.is_governor_mode,
        ecosystem_contracts,
        chain_contracts,
        chain_name,
        context,
    })
}

/// Display the resolved accept configuration as a UI note.
pub(super) fn display_config(config: &AcceptConfig<'_>, args: &AcceptArgs) -> Result<()> {
    ui::note(
        "Accept configuration",
        format!(
            "Ecosystem: {}\nScope: {}\nRPC URL: {}",
            ui::green(args.ecosystem_name.as_deref().unwrap_or("(from config)")),
            ui::green(&args.scope),
            ui::green(&config.rpc_url)
        ),
    )?;
    Ok(())
}

/// Resolve private key with 4-level priority:
/// 1. `--private-key` argument / `ADI_PRIVATE_KEY` env var (new owner mode)
/// 2. Config `ownership.private_key` (new owner mode)
/// 3. `--use-governor` flag (governor mode)
/// 4. Interactive prompt
async fn resolve_private_key(
    args: &AcceptArgs,
    state_manager: &adi_state::StateManager,
    context: &Context,
) -> Result<ResolvedKey> {
    // Priority 1: CLI argument or env var
    if let Some(ref key_hex) = args.private_key {
        let secret = SecretString::from(key_hex.clone());
        let address = derive_address_from_key(&secret)?;
        ui::info(format!(
            "Using provided private key (address: {})",
            ui::green(address)
        ))?;
        return Ok(ResolvedKey {
            private_key: secret,
            address,
            is_governor_mode: false,
        });
    }

    // Priority 2: Config file
    if let Some(ref config_key) = context.config().ownership.private_key {
        let address = derive_address_from_key(config_key)?;
        ui::info(format!(
            "Using private key from config (address: {})",
            ui::green(address)
        ))?;
        return Ok(ResolvedKey {
            private_key: config_key.clone(),
            address,
            is_governor_mode: false,
        });
    }

    // Priority 3: --use-governor flag
    if args.use_governor {
        let (private_key, address) = load_governor_key(state_manager).await?;
        ui::info(format!(
            "Using governor key (address: {})",
            ui::green(address)
        ))?;
        return Ok(ResolvedKey {
            private_key,
            address,
            is_governor_mode: true,
        });
    }

    // Priority 4: Interactive prompt
    resolve_private_key_interactive(state_manager).await
}

/// Prompt the user to choose governor or provide a private key.
async fn resolve_private_key_interactive(
    state_manager: &adi_state::StateManager,
) -> Result<ResolvedKey> {
    let use_governor = ui::confirm("Accept ownership as governor?")
        .initial_value(true)
        .interact()
        .wrap_err("Failed to get confirmation")?;

    if use_governor {
        let (private_key, address) = load_governor_key(state_manager).await?;
        return Ok(ResolvedKey {
            private_key,
            address,
            is_governor_mode: true,
        });
    }

    let key_hex: String = ui::password("Enter private key (hex):")
        .mask('*')
        .interact()
        .wrap_err("Failed to read private key")?;
    let secret = SecretString::from(key_hex);
    let address = derive_address_from_key(&secret)?;
    ui::info(format!(
        "Using provided key (address: {})",
        ui::green(address)
    ))?;
    Ok(ResolvedKey {
        private_key: secret,
        address,
        is_governor_mode: false,
    })
}

/// Load ecosystem contracts if the scope includes ecosystem.
async fn load_ecosystem_contracts(
    include: bool,
    state_manager: &adi_state::StateManager,
) -> Result<Option<EcosystemContracts>> {
    if !include {
        return Ok(None);
    }
    state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")
        .map(Some)
}

/// Resolve chain name if the scope includes chain.
async fn resolve_chain_name(
    include: bool,
    chain_arg: Option<&String>,
    state_manager: &adi_state::StateManager,
    ecosystem_name: &str,
) -> Result<Option<String>> {
    if !include {
        return Ok(None);
    }
    select_chain_from_state(chain_arg, state_manager, ecosystem_name)
        .await
        .map(Some)
}

/// Load chain contracts if a chain name is provided.
async fn load_chain_contracts(
    chain_name: Option<&str>,
    state_manager: &adi_state::StateManager,
) -> Result<Option<ChainContracts>> {
    let name = match chain_name {
        Some(n) => n,
        None => return Ok(None),
    };
    state_manager
        .chain(name)
        .contracts()
        .await
        .wrap_err(format!("Failed to load chain contracts for '{}'", name))
        .map(Some)
}

/// Load the governor wallet private key and derive its address.
async fn load_governor_key(
    state_manager: &adi_state::StateManager,
) -> Result<(SecretString, Address)> {
    let wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;
    let governor = wallets
        .governor
        .as_ref()
        .ok_or_else(|| eyre::eyre!("Governor wallet not found in ecosystem state"))?;
    let address = derive_address_from_key(&governor.private_key)?;
    Ok((governor.private_key.clone(), address))
}
