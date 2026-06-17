//! Configuration resolution for the accept ownership command.

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use secrecy::SecretString;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, derive_address_from_key, resolve_chain_private_key,
    resolve_ecosystem_name, resolve_gas_multiplier, resolve_private_key, resolve_rpc_url,
    select_chain_from_state, OwnershipScope,
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

/// Base accept configuration resolved without any signing key.
///
/// The calldata path uses this directly (no key required); the execute path
/// upgrades it into an [`AcceptConfig`] by resolving signing keys.
pub(super) struct AcceptBase<'a> {
    pub ecosystem_name: String,
    pub rpc_url: Url,
    pub gas_multiplier: Option<u64>,
    pub ecosystem_contracts: Option<EcosystemContracts>,
    pub chain_contracts: Option<ChainContracts>,
    pub chain_name: Option<String>,
    pub state_manager: adi_state::StateManager,
    pub context: &'a Context,
}

/// Resolved accept configuration bundling all data needed by execution phases.
pub(super) struct AcceptConfig<'a> {
    pub rpc_url: Url,
    pub gas_multiplier: Option<u64>,
    pub private_key: SecretString,
    pub key_address: Address,
    /// Signing key for chain-level contracts. In governor mode this is the
    /// chain's own governor (distinct from the ecosystem governor); otherwise
    /// it mirrors `private_key`.
    pub chain_private_key: SecretString,
    pub chain_key_address: Address,
    pub is_governor_mode: bool,
    pub ecosystem_contracts: Option<EcosystemContracts>,
    pub chain_contracts: Option<ChainContracts>,
    pub chain_name: Option<String>,
    pub context: &'a Context,
}

/// Resolve base configuration: names, URLs, scope, contracts, gas. No key.
pub(super) async fn resolve_base<'a>(
    args: &AcceptArgs,
    context: &'a Context,
) -> Result<AcceptBase<'a>> {
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
    let gas_multiplier = Some(resolve_gas_multiplier(
        args.gas_multiplier,
        context.config(),
    ));

    Ok(AcceptBase {
        ecosystem_name,
        rpc_url,
        gas_multiplier,
        ecosystem_contracts,
        chain_contracts,
        chain_name,
        state_manager,
        context,
    })
}

/// Upgrade a base config into a full config by resolving signing keys.
///
/// Used only by the execute path; may prompt interactively for a key.
pub(super) async fn resolve_signing<'a>(
    base: AcceptBase<'a>,
    args: &AcceptArgs,
) -> Result<AcceptConfig<'a>> {
    let resolved_key = resolve_accept_key(args, &base.state_manager, base.context).await?;
    let (chain_private_key, chain_key_address) = resolve_chain_accept_key(
        &resolved_key,
        base.chain_name.as_deref(),
        &base.state_manager,
        base.context,
    )
    .await?;

    Ok(AcceptConfig {
        rpc_url: base.rpc_url,
        gas_multiplier: base.gas_multiplier,
        private_key: resolved_key.private_key,
        key_address: resolved_key.address,
        chain_private_key,
        chain_key_address,
        is_governor_mode: resolved_key.is_governor_mode,
        ecosystem_contracts: base.ecosystem_contracts,
        chain_contracts: base.chain_contracts,
        chain_name: base.chain_name,
        context: base.context,
    })
}

/// Resolve the expected pending-owner addresses for calldata generation.
///
/// No private key is needed: the expected owner is the configured `new_owner`
/// (the multisig/EOA the transfer targeted), falling back to the governor
/// wallet address when no transfer target is configured. Returns `None` for a
/// scope whose contracts are absent.
pub(super) async fn resolve_expected_owners(
    base: &AcceptBase<'_>,
) -> Result<(Option<Address>, Option<Address>)> {
    let ecosystem = match base.ecosystem_contracts {
        Some(_) => Some(resolve_ecosystem_expected_owner(base).await?),
        None => None,
    };
    let chain = match (base.chain_contracts.as_ref(), base.chain_name.as_deref()) {
        (Some(_), Some(name)) => Some(resolve_chain_expected_owner(base, name).await?),
        _ => None,
    };
    Ok((ecosystem, chain))
}

/// Resolve the ecosystem expected owner: configured `new_owner` or governor.
async fn resolve_ecosystem_expected_owner(base: &AcceptBase<'_>) -> Result<Address> {
    if let Some(addr) = base.context.config().ecosystem.ownership.new_owner {
        return Ok(addr);
    }
    let wallets = base
        .state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;
    wallets.governor.as_ref().map(|w| w.address).ok_or_else(|| {
        eyre::eyre!("No ecosystem new_owner configured and governor wallet not found")
    })
}

/// Resolve the chain expected owner: configured `new_owner` or chain governor.
async fn resolve_chain_expected_owner(base: &AcceptBase<'_>, chain_name: &str) -> Result<Address> {
    if let Some(addr) = base
        .context
        .config()
        .ecosystem
        .get_chain(chain_name)
        .and_then(|c| c.ownership.new_owner)
    {
        return Ok(addr);
    }
    let wallets = base
        .state_manager
        .chain(chain_name)
        .wallets()
        .await
        .wrap_err(format!("Failed to load chain wallets for '{}'", chain_name))?;
    wallets.governor.as_ref().map(|w| w.address).ok_or_else(|| {
        eyre::eyre!(
            "No chain new_owner configured and governor wallet not found for '{}'",
            chain_name
        )
    })
}

/// Display the resolved accept configuration as a UI note.
pub(super) fn display_config(base: &AcceptBase<'_>, args: &AcceptArgs) -> Result<()> {
    ui::note(
        "Accept configuration",
        format!(
            "Ecosystem: {}\nScope: {}\nRPC URL: {}",
            ui::green(&base.ecosystem_name),
            ui::green(&args.scope),
            ui::green(&base.rpc_url)
        ),
    )?;
    Ok(())
}

/// Resolve private key with 4-level priority:
/// 1. `--private-key` argument / `ADI_PRIVATE_KEY` env var (new owner mode)
/// 2. Config `ownership.private_key` (new owner mode)
/// 3. `--use-governor` flag (governor mode)
/// 4. Interactive prompt
async fn resolve_accept_key(
    args: &AcceptArgs,
    state_manager: &adi_state::StateManager,
    context: &Context,
) -> Result<ResolvedKey> {
    // Priorities 1 & 2: CLI arg / env var, then config `ownership.private_key`.
    if let Some(secret) = resolve_private_key(args.private_key.as_deref(), context.config()) {
        let address = derive_address_from_key(&secret)?;
        ui::info(format!(
            "Using configured private key (address: {})",
            ui::green(address)
        ))?;
        return Ok(ResolvedKey {
            private_key: secret,
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

/// Resolve the signing key for chain-level contracts.
///
/// Chain contracts (Chain Governance, Chain Admin) are owned by the chain's own
/// governor wallet, not the ecosystem governor — and after a transfer, by the
/// chain's own `new_owner`, which differs from the ecosystem one. So:
/// - governor mode: load the chain governor wallet from chain state.
/// - new-owner mode: use the chain's configured `ownership.private_key`; if the
///   chain has none, reuse the already-resolved ecosystem key.
async fn resolve_chain_accept_key(
    resolved: &ResolvedKey,
    chain_name: Option<&str>,
    state_manager: &adi_state::StateManager,
    context: &Context,
) -> Result<(SecretString, Address)> {
    let Some(name) = chain_name else {
        return Ok((resolved.private_key.clone(), resolved.address));
    };
    if resolved.is_governor_mode {
        return load_chain_governor_key(state_manager, name).await;
    }
    let Some(chain_key) = resolve_chain_private_key(context.config(), name) else {
        return Ok((resolved.private_key.clone(), resolved.address));
    };
    let address = derive_address_from_key(&chain_key)?;
    ui::info(format!(
        "Using chain '{}' configured key (address: {})",
        name,
        ui::green(address)
    ))?;
    Ok((chain_key, address))
}

/// Load the chain governor wallet private key and derive its address.
async fn load_chain_governor_key(
    state_manager: &adi_state::StateManager,
    chain_name: &str,
) -> Result<(SecretString, Address)> {
    let wallets = state_manager
        .chain(chain_name)
        .wallets()
        .await
        .wrap_err(format!("Failed to load chain wallets for '{}'", chain_name))?;
    let governor = wallets
        .governor
        .ok_or_else(|| eyre::eyre!("Governor wallet not found in chain '{}' state", chain_name))?;
    let address = derive_address_from_key(&governor.private_key)?;
    Ok((governor.private_key, address))
}
