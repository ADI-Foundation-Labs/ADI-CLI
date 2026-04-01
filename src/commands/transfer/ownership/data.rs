//! Data loading helpers for the transfer ownership command.
//!
//! Loads ecosystem and chain contracts and governor wallets from state.

use adi_types::{ChainContracts, EcosystemContracts, Wallet};

use crate::error::{Result, WrapErr};

/// Load ecosystem contracts and governor wallet from state.
pub(super) async fn load_ecosystem_data(
    state_manager: &adi_state::StateManager,
    ecosystem_name: &str,
) -> Result<(EcosystemContracts, Wallet)> {
    let contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")?;

    let wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;

    let governor = wallets.governor.ok_or_else(|| {
        eyre::eyre!(
            "Governor wallet not found in ecosystem wallets for '{}'",
            ecosystem_name
        )
    })?;

    Ok((contracts, governor))
}

/// Load chain contracts and governor wallet from state.
pub(super) async fn load_chain_data(
    state_manager: &adi_state::StateManager,
    chain_name: &str,
) -> Result<(ChainContracts, Wallet)> {
    let contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err(format!(
            "Failed to load chain contracts for '{}'",
            chain_name
        ))?;

    let wallets = state_manager
        .chain(chain_name)
        .wallets()
        .await
        .wrap_err(format!("Failed to load chain wallets for '{}'", chain_name))?;

    let governor = wallets.governor.ok_or_else(|| {
        eyre::eyre!(
            "Governor wallet not found in chain wallets for '{}'",
            chain_name
        )
    })?;

    Ok((contracts, governor))
}
