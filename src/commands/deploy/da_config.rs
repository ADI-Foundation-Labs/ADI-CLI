//! Data Availability mode configuration.

use adi_ecosystem::{
    configure_da_validator_pair, DaValidatorPairConfig, DeployedContracts, L2DACommitmentScheme,
    PubdataMode,
};
use adi_state::StateManager;
use alloy_primitives::Address;
use secrecy::SecretString;

use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use super::args::DeployArgs;

/// Resolve the pubdata mode from CLI args, falling back to chain config.
pub fn resolve_pubdata_mode(args: &DeployArgs, context: &Context, chain_name: &str) -> PubdataMode {
    if let Some(mode) = args.pubdata_mode {
        return mode;
    }

    context
        .config()
        .ecosystem
        .get_chain(chain_name)
        .map(|c| c.pubdata_mode)
        .unwrap_or_default()
}

/// Configure calldata DA mode (rollup pubdata posted as calldata).
///
/// Pairs the rollup L1 DA validator with the rollup commitment scheme
/// (`BlobsAndPubdataKeccak256`, scheme 3). The server posts pubdata as calldata
/// via the per-batch transport flag. Applies to both L2 (settling on L1) and L3
/// (settling on a gateway).
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

    let l1_da_validator =
        get_l1_da_validator_address(state_manager, chain_name, PubdataMode::Calldata)
            .await
            .wrap_err("Failed to get L1 DA validator address")?;

    let tx_hash = configure_da_validator_pair(
        DaValidatorPairConfig {
            rpc_url,
            chain_admin: deployed.chain_admin,
            diamond_proxy: deployed.diamond_proxy,
            l1_da_validator,
            commitment_scheme: L2DACommitmentScheme::BlobsAndPubdataKeccak256,
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

/// Configure custom (external) DA mode, e.g. Avail.
///
/// Pairs the Avail L1 DA validator with the `PubdataKeccak256` commitment scheme
/// (scheme 2): only a keccak commitment of the pubdata is stored on L1, while the
/// data itself lives on the external DA layer. The server must additionally run
/// in `External` pubdata mode with its `external_da_*` settings populated.
pub async fn configure_custom_da(
    context: &Context,
    state_manager: &StateManager,
    chain_name: &str,
    rpc_url: &str,
    deployed: &DeployedContracts,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
) -> Result<()> {
    ui::section("Configuring Custom (External) DA Mode")?;

    let l1_da_validator =
        get_l1_da_validator_address(state_manager, chain_name, PubdataMode::CustomDa)
            .await
            .wrap_err("Failed to get Avail L1 DA validator address")?;

    let tx_hash = configure_da_validator_pair(
        DaValidatorPairConfig {
            rpc_url,
            chain_admin: deployed.chain_admin,
            diamond_proxy: deployed.diamond_proxy,
            l1_da_validator,
            commitment_scheme: L2DACommitmentScheme::PubdataKeccak256,
            governor_key,
            gas_multiplier,
        },
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to configure custom DA mode")?;

    ui::success(format!("Custom DA mode configured: {}", ui::green(tx_hash)))?;

    Ok(())
}

/// Get the L1 DA validator address for the given pubdata mode from state.
///
/// Checks chain contracts first, then falls back to ecosystem-level contracts.
/// Blobs uses the rollup validator here for completeness, but the blobs mode is
/// the registration default and never calls `setDAValidatorPair`.
async fn get_l1_da_validator_address(
    state_manager: &StateManager,
    chain_name: &str,
    mode: PubdataMode,
) -> Result<Address> {
    let chain_contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts")?;

    let l1 = chain_contracts.l1.as_ref();
    let chain_ecosystem = chain_contracts.ecosystem_contracts.as_ref();

    let (l1_addr, chain_ecosystem_addr) = match mode {
        PubdataMode::CustomDa => (
            l1.and_then(|l1| l1.avail_l1_da_validator_addr),
            chain_ecosystem.and_then(|eco| eco.avail_l1_da_validator_addr),
        ),
        PubdataMode::Blobs | PubdataMode::Calldata => (
            l1.and_then(|l1| l1.rollup_l1_da_validator_addr),
            chain_ecosystem.and_then(|eco| eco.rollup_l1_da_validator_addr),
        ),
    };

    if let Some(addr) = l1_addr.or(chain_ecosystem_addr) {
        return Ok(addr);
    }

    let eco_contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")?;

    let ctm = eco_contracts.zksync_os_ctm.as_ref();

    let ctm_addr = match mode {
        PubdataMode::CustomDa => ctm.and_then(|ctm| ctm.avail_l1_da_validator_addr),
        PubdataMode::Blobs | PubdataMode::Calldata => {
            ctm.and_then(|ctm| ctm.rollup_l1_da_validator_addr)
        }
    };

    let addr = ctm_addr.ok_or_else(|| {
        eyre::eyre!(
            "L1 DA validator address for mode {:?} not found in state. \
             Ensure deployment completed successfully.",
            mode
        )
    })?;

    Ok(addr)
}
