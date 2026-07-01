//! Data Availability mode configuration.

use adi_ecosystem::{configure_l3_da, DeployedContracts, L3DaConfig, PubdataSource};
use adi_state::StateManager;
use alloy_primitives::Address;
use secrecy::SecretString;

use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use super::args::DeployArgs;

/// Data Availability modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DAMode {
    /// Use blob-based pubdata (EIP-4844) - L2 behavior.
    Blobs,
    /// Use calldata for pubdata - L3 behavior.
    Calldata,
    /// No DA - Validium behavior.
    Validium,
}

/// Resolve DA mode from CLI args, falling back to chain config.
pub fn resolve_da_mode(args: &DeployArgs, context: &Context, chain_name: &str) -> DAMode {
    if let Some(true) = args.validium {
        return DAMode::Validium;
    }
    if let Some(blobs) = args.blobs {
        return if blobs {
            DAMode::Blobs
        } else {
            DAMode::Calldata
        };
    }

    context
        .config()
        .ecosystem
        .get_chain(chain_name)
        .map(|c| {
            if c.validium {
                DAMode::Validium
            } else if c.blobs {
                DAMode::Blobs
            } else {
                DAMode::Calldata
            }
        })
        .unwrap_or(DAMode::Calldata)
}

/// Configure calldata DA mode for L3 chains settling on L2.
pub async fn configure_calldata_da(
    context: &Context,
    state_manager: &StateManager,
    chain_name: &str,
    rpc_url: &str,
    deployed: &DeployedContracts,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
) -> Result<()> {
    ui::section("Configuring Calldata DA Mode")?;

    let l1_da_validator = get_l1_da_validator_address(state_manager, chain_name)
        .await
        .wrap_err("Failed to get L1 DA validator address")?;

    let tx_hash = configure_l3_da(
        L3DaConfig {
            rpc_url,
            chain_admin: deployed.chain_admin,
            diamond_proxy: deployed.diamond_proxy,
            l1_da_validator,
            pubdata_source: PubdataSource::PubdataKeccak256,
            governor_key,
            gas_multiplier,
        },
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to configure calldata DA mode")?;

    ui::success(format!(
        "Calldata DA mode configured: {}",
        ui::green(tx_hash)
    ))?;

    Ok(())
}

/// Configure Validium DA mode (no DA) on L1.
pub async fn configure_validium_da(
    context: &Context,
    state_manager: &StateManager,
    chain_name: &str,
    rpc_url: &str,
    deployed: &DeployedContracts,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
) -> Result<()> {
    ui::info("Configuring Validium mode (no DA)...")?;

    let da_validator = get_l1_da_validator_address(state_manager, chain_name).await?;

    let tx_hash = configure_l3_da(
        L3DaConfig {
            rpc_url,
            chain_admin: deployed.chain_admin,
            diamond_proxy: deployed.diamond_proxy,
            l1_da_validator: da_validator,
            pubdata_source: PubdataSource::EmptyNoDa,
            governor_key,
            gas_multiplier,
        },
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to configure Validium mode")?;

    ui::success(format!(
        "Validium mode (no DA) configured: {}",
        ui::green(tx_hash)
    ))?;

    Ok(())
}

/// Get L1 DA validator address from state.
///
/// Checks chain contracts first, then falls back to ecosystem-level contracts.
async fn get_l1_da_validator_address(
    state_manager: &StateManager,
    chain_name: &str,
) -> Result<Address> {
    let chain_contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts")?;

    if let Some(l1) = &chain_contracts.l1 {
        if let Some(addr) = l1.rollup_l1_da_validator_addr {
            return Ok(addr);
        }
    }

    if let Some(eco) = &chain_contracts.ecosystem_contracts {
        if let Some(addr) = eco.rollup_l1_da_validator_addr {
            return Ok(addr);
        }
    }

    let eco_contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")?;

    if let Some(ctm) = &eco_contracts.zksync_os_ctm {
        if let Some(addr) = ctm.rollup_l1_da_validator_addr {
            return Ok(addr);
        }
    }

    Err(eyre::eyre!(
        "L1 DA validator address not found in state. \
         Ensure deployment completed successfully."
    ))
}
