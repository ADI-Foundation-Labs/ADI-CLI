//! Core ecosystem and chain deployment orchestration.

use adi_ecosystem::verification::{
    apply_implementations, parse_diamond_cut_data, read_all_implementations,
};
use adi_ecosystem::DeployedContracts;
use adi_funding::{is_localhost_rpc, normalize_rpc_url, FundingProvider};
use adi_state::StateManager;
use adi_toolkit::{EcosystemInitParams, ProtocolVersion, ToolkitRunner};
use adi_types::Wallets;
use std::sync::Arc;
use url::Url;

use crate::commands::helpers::{resolve_gas_multiplier, resolve_protocol_version};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use super::args::DeployArgs;
use super::da_config::{configure_calldata_da, configure_validium_da, resolve_da_mode, DAMode};
use super::display::display_deployment_summary;
use super::files::log_deployment_files;
use super::ownership::run_post_deploy_ownership;
use super::validators::configure_validator_roles;

/// Run ecosystem contract deployment and validator role configuration.
///
/// Shared deployment logic used by both Anvil and production funding paths.
pub async fn run_ecosystem_deployment(
    args: &DeployArgs,
    context: &Context,
    state_manager: &StateManager,
    ecosystem_name: &str,
    chain_name: &str,
    rpc_url: &Url,
    chain_wallets: &Wallets,
) -> Result<()> {
    let protocol_version_str =
        resolve_protocol_version(args.protocol_version.as_ref(), context.config())?;
    let protocol_version = ProtocolVersion::parse(&protocol_version_str)
        .map_err(|e| eyre::eyre!("Invalid protocol version '{}': {}", protocol_version_str, e))?;

    let deploy_ecosystem = !ecosystem_contracts_deployed(state_manager).await?;

    ui::info(format!(
        "Protocol version: {}",
        ui::green(&protocol_version)
    ))?;

    if deploy_ecosystem {
        ui::section("Deploying Ecosystem Contracts")?;
    } else {
        ui::section("Deploying Chain Contracts")?;
        ui::info("Ecosystem contracts already exist, deploying chain only")?;
    }

    let gas_multiplier = run_zkstack_init(
        context,
        args,
        ecosystem_name,
        chain_name,
        rpc_url,
        &protocol_version,
        deploy_ecosystem,
    )
    .await?;

    enrich_ecosystem_contracts(context, state_manager, rpc_url).await?;

    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    log_deployment_files(&ecosystem_path, chain_name)?;

    let deployed = read_deployed_contracts(state_manager, chain_name).await?;

    let governor_key = chain_wallets
        .governor
        .as_ref()
        .ok_or_else(|| eyre::eyre!("Chain governor wallet required for validator role setup"))?
        .private_key
        .clone();

    let normalized_rpc = normalize_rpc_url(rpc_url.as_str());

    // Skip gas multiplier on localhost to avoid inflated gas prices
    let validator_gas_multiplier = if is_localhost_rpc(rpc_url.as_str()) {
        None
    } else {
        Some(gas_multiplier)
    };

    configure_validator_roles(
        context,
        state_manager,
        chain_name,
        &normalized_rpc,
        &deployed,
        &governor_key,
        validator_gas_multiplier,
    )
    .await?;

    let da_mode = resolve_da_mode(args, context, chain_name);
    match da_mode {
        DAMode::Blobs => {
            context.logger().info("Using Blobs for DA (L2 mode)");
        }
        DAMode::Calldata => {
            configure_calldata_da(
                context,
                state_manager,
                chain_name,
                &normalized_rpc,
                &deployed,
                &governor_key,
                validator_gas_multiplier,
            )
            .await?;
        }
        DAMode::Validium => {
            configure_validium_da(
                context,
                state_manager,
                chain_name,
                &normalized_rpc,
                &deployed,
                &governor_key,
                validator_gas_multiplier,
            )
            .await?;
        }
    }

    let ownership_result = run_post_deploy_ownership(
        &normalized_rpc,
        state_manager,
        chain_name,
        validator_gas_multiplier,
        context,
    )
    .await?;

    // Deploy FeeAdjusterConfig on L1; opt out via `fee_adjuster.enabled: false` in .adi.yml.
    let fee_adjuster_address = if context.config().fee_adjuster.enabled {
        let fee_adjuster_gas_price = compute_fee_adjuster_gas_price(rpc_url, gas_multiplier)
            .await
            .wrap_err("Failed to estimate gas price for fee-adjuster deployment")?;
        let addr = super::fee_adjuster::deploy_fee_adjuster(
            super::fee_adjuster::DeployFeeAdjusterParams {
                context,
                state_manager,
                chain_name,
                ecosystem_name,
                rpc_url,
                protocol_version: &protocol_version,
                gas_price_wei: fee_adjuster_gas_price,
            },
        )
        .await?;
        Some(addr)
    } else {
        ui::info("Skipping FeeAdjusterConfig deployment (fee_adjuster.enabled = false)")?;
        None
    };

    display_deployment_summary(
        ecosystem_name,
        chain_name,
        &deployed,
        &ownership_result,
        fee_adjuster_address,
    )?;

    Ok(())
}

/// Estimate gas price for the fee-adjuster forge script (skipped on localhost).
async fn compute_fee_adjuster_gas_price(
    rpc_url: &Url,
    gas_multiplier: u64,
) -> Result<Option<u128>> {
    if is_localhost_rpc(rpc_url.as_str()) {
        return Ok(None);
    }
    let provider = FundingProvider::new(rpc_url.as_str())
        .wrap_err("Failed to create provider for gas estimation")?;
    let estimated = provider
        .get_gas_price()
        .await
        .wrap_err("Failed to estimate gas price")?;
    Ok(Some(estimated * u128::from(gas_multiplier) / 100))
}

/// Run zkstack ecosystem or chain init with gas price estimation.
///
/// Returns the gas multiplier for reuse in downstream operations.
async fn run_zkstack_init(
    context: &Context,
    args: &DeployArgs,
    ecosystem_name: &str,
    chain_name: &str,
    rpc_url: &Url,
    protocol_version: &ProtocolVersion,
    deploy_ecosystem: bool,
) -> Result<u64> {
    let runner = ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;

    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    let gas_multiplier = resolve_gas_multiplier(args.gas_multiplier, context.config());

    let gas_price_wei = if is_localhost_rpc(rpc_url.as_str()) {
        None
    } else {
        let provider = FundingProvider::new(rpc_url.as_str())
            .wrap_err("Failed to create provider for gas estimation")?;
        let estimated = provider
            .get_gas_price()
            .await
            .wrap_err("Failed to estimate gas price")?;
        Some(estimated * u128::from(gas_multiplier) / 100)
    };

    let init_msg = if deploy_ecosystem {
        "Running zkstack ecosystem init..."
    } else {
        "Running zkstack chain init..."
    };
    ui::info(init_msg)?;

    let exit_code = runner
        .run_zkstack_ecosystem_init(&EcosystemInitParams {
            ecosystem_dir: &ecosystem_path,
            l1_rpc_url: rpc_url.as_str(),
            gas_price_wei,
            protocol_version: &protocol_version.to_semver(),
            deploy_ecosystem,
            chain_name,
        })
        .await
        .wrap_err("Failed to run zkstack ecosystem init")?;

    if exit_code != 0 {
        return Err(eyre::eyre!(
            "zkstack ecosystem init failed with exit code {}",
            exit_code
        ));
    }

    let success_msg = if deploy_ecosystem {
        "Ecosystem contracts deployed successfully!"
    } else {
        "Chain contracts deployed successfully!"
    };
    ui::success(success_msg)?;

    Ok(gas_multiplier)
}

/// Read deployed contract addresses from state and display them.
async fn read_deployed_contracts(
    state_manager: &StateManager,
    chain_name: &str,
) -> Result<DeployedContracts> {
    let chain_contracts = state_manager
        .chain(chain_name)
        .contracts()
        .await
        .wrap_err("Failed to load chain contracts after deployment")?;

    let deployed = DeployedContracts::try_from_chain_contracts(&chain_contracts)
        .wrap_err("Missing required contract addresses after deployment")?;

    ui::note(
        "Deployed Contracts",
        format!(
            "Diamond proxy: {}\nValidator timelock: {}\nChain admin: {}",
            ui::green(deployed.diamond_proxy),
            ui::green(deployed.validator_timelock),
            ui::green(deployed.chain_admin)
        ),
    )?;

    Ok(deployed)
}

/// Enrich ecosystem contracts with facet and implementation addresses.
///
/// Reads diamond_cut_data from state and parses facet addresses.
/// Reads implementation addresses from proxy contracts via RPC.
/// Saves enriched contracts back to state for future use (e.g., verification).
async fn enrich_ecosystem_contracts(
    context: &Context,
    state_manager: &StateManager,
    rpc_url: &Url,
) -> Result<()> {
    context
        .logger()
        .debug("Enriching ecosystem contracts with facet and implementation addresses");

    let mut ecosystem_contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts for enrichment")?;

    let mut enriched = false;

    if let Some(ref mut ctm) = ecosystem_contracts.zksync_os_ctm {
        if ctm.admin_facet_addr.is_none() {
            if let Some(ref diamond_cut_data) = ctm.diamond_cut_data {
                match parse_diamond_cut_data(diamond_cut_data) {
                    Ok(facets) => {
                        ctm.admin_facet_addr = facets.admin_facet;
                        ctm.executor_facet_addr = facets.executor_facet;
                        ctm.mailbox_facet_addr = facets.mailbox_facet;
                        ctm.getters_facet_addr = facets.getters_facet;
                        ctm.diamond_init_addr = facets.diamond_init;
                        enriched = true;
                        context
                            .logger()
                            .debug("Extracted facet addresses from diamond_cut_data");
                    }
                    Err(e) => {
                        context
                            .logger()
                            .warning(&format!("Could not parse diamond_cut_data: {}", e));
                    }
                }
            }
        }
    }

    if ecosystem_contracts
        .zksync_os_ctm
        .as_ref()
        .is_some_and(|ctm| ctm.bridgehub_impl_addr.is_none())
    {
        let normalized_rpc = normalize_rpc_url(rpc_url.as_str());
        let provider = FundingProvider::new(&normalized_rpc)
            .wrap_err("Failed to create provider for implementation address reading")?;

        let impls = read_all_implementations(
            provider.inner(),
            &ecosystem_contracts,
            Arc::clone(context.logger()),
        )
        .await;

        apply_implementations(&mut ecosystem_contracts, &impls);
        enriched = true;
        context
            .logger()
            .debug("Read implementation addresses from proxy contracts");
    }

    if enriched {
        state_manager
            .ecosystem()
            .update_contracts(&ecosystem_contracts)
            .await
            .wrap_err("Failed to save enriched ecosystem contracts")?;
        ui::info("Enriched contracts with facet and implementation addresses")?;
    }

    Ok(())
}

/// Check if ecosystem contracts have already been deployed.
///
/// Returns `true` if contracts.yaml exists and contains bridgehub_proxy_addr.
async fn ecosystem_contracts_deployed(state_manager: &StateManager) -> Result<bool> {
    if !state_manager.ecosystem().contracts_exist().await? {
        return Ok(false);
    }

    match state_manager.ecosystem().contracts().await {
        Ok(contracts) => Ok(contracts.bridgehub_addr().is_some()),
        Err(_) => Ok(false),
    }
}
