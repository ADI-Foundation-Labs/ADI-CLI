//! Display ecosystem and chain information with deployed contracts.

mod counting;
mod display;

use adi_state::StateManager;
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_ecosystem_name, select_chain_from_state,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;
use adi_types::EcosystemContracts;

use counting::{count_chain_contracts_with_ctm, count_ecosystem_contracts};
use display::{
    format_chain_contracts_with_ctm, format_chain_metadata, format_ecosystem_contracts,
    format_ecosystem_metadata,
};

/// Arguments for `ecosystem` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct EcosystemArgs {
    /// Ecosystem name (falls back to config file if not provided).
    #[arg(long, help = "Ecosystem name (falls back to config if not provided)")]
    pub ecosystem_name: Option<String>,

    /// Chain name to display chain-level information.
    #[arg(long, help = "Chain name to display chain-level information")]
    pub chain: Option<String>,
}

/// Execute the ecosystem command.
pub async fn run(args: &EcosystemArgs, context: &Context) -> Result<()> {
    ui::intro("ADI CLI")?;

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let state_manager = create_state_manager_with_context(&ecosystem_name, context)?;

    // Check if ecosystem exists
    if !ecosystem_exists(&state_manager).await? {
        ui::warning(format!(
            "Ecosystem '{}' not found.\nRun 'adi init' first to initialize the ecosystem.",
            ecosystem_name
        ))?;
        ui::outro("")?;
        return Ok(());
    }

    // Load and display ecosystem metadata
    let eco_metadata = state_manager
        .ecosystem()
        .metadata()
        .await
        .wrap_err("Failed to load ecosystem metadata")?;

    let initial_deployments = state_manager.ecosystem().initial_deployments().await.ok();

    let meta_display = format_ecosystem_metadata(&eco_metadata, initial_deployments.as_ref());
    ui::note(
        format!("Ecosystem '{}'", ui::green(&ecosystem_name)),
        meta_display,
    )?;

    // Load and display ecosystem contracts if they exist
    let ecosystem_contracts = if contracts_exist(&state_manager).await? {
        let ecosystem_contracts = state_manager
            .ecosystem()
            .contracts()
            .await
            .wrap_err("Failed to load ecosystem contracts")?;

        let count = count_ecosystem_contracts(&ecosystem_contracts);
        let contracts_display = format_ecosystem_contracts(&ecosystem_contracts);
        ui::note("Ecosystem Contracts (L1)", contracts_display)?;
        Some((ecosystem_contracts, count))
    } else {
        ui::info("No contracts deployed yet. Run 'adi deploy' to deploy.")?;
        None
    };
    let contract_count = ecosystem_contracts.as_ref().map(|(_, count)| *count);

    // Load and display chain information.
    // Use --chain if provided; auto-select when a single chain exists;
    // otherwise prompt the user to pick one.
    let chains = state_manager
        .list_chains()
        .await
        .wrap_err("Failed to list chains")?;
    let chain_contract_count = if chains.is_empty() {
        ui::info("No chains found. Run 'adi add' to add a chain.")?;
        None
    } else {
        let chain_to_display =
            select_chain_from_state(args.chain.as_ref(), &state_manager, &ecosystem_name).await?;
        display_chain_info(
            &state_manager,
            &chain_to_display,
            ecosystem_contracts.as_ref().map(|(contracts, _)| contracts),
        )
        .await?
    };

    // Display summary (ecosystem + chain contracts)
    let summary = match (contract_count, chain_contract_count) {
        (Some(eco), Some(chain)) => {
            format!(
                "Total contracts: {} (ecosystem: {}, chain: {})",
                eco + chain,
                eco,
                chain
            )
        }
        (Some(eco), None) => format!("Total contracts: {}", eco),
        _ => "No contracts deployed".to_string(),
    };
    ui::outro(summary)?;
    Ok(())
}

/// Display chain information including metadata and contracts.
///
/// Returns the count of unique chain contracts if they exist.
async fn display_chain_info(
    state_manager: &StateManager,
    chain_name: &str,
    ecosystem_contracts: Option<&EcosystemContracts>,
) -> Result<Option<usize>> {
    let chain_ops = state_manager.chain(chain_name);

    // Check if chain exists
    if !chain_ops.exists().await? {
        ui::warning(format!("Chain '{}' not found.", chain_name))?;
        return Ok(None);
    }

    // Load and display chain metadata
    let chain_metadata = chain_ops
        .metadata()
        .await
        .wrap_err("Failed to load chain metadata")?;

    let meta_display = format_chain_metadata(&chain_metadata);
    ui::note(format!("Chain '{}'", ui::green(chain_name)), meta_display)?;

    // Load and display chain contracts if they exist
    if chain_ops.contracts_exist().await? {
        let chain_contracts = chain_ops
            .contracts()
            .await
            .wrap_err("Failed to load chain contracts")?;

        let ctm = ecosystem_contracts.and_then(|contracts| contracts.zksync_os_ctm.as_ref());
        let count = count_chain_contracts_with_ctm(&chain_contracts, ctm);
        let contracts_display = format_chain_contracts_with_ctm(&chain_contracts, ctm);
        ui::note(
            format!("Chain '{}' Contracts", chain_name),
            contracts_display,
        )?;
        Ok(Some(count))
    } else {
        ui::info(format!(
            "No contracts deployed for chain '{}' yet.",
            chain_name
        ))?;
        Ok(None)
    }
}

/// Check if ecosystem metadata exists.
async fn ecosystem_exists(state_manager: &StateManager) -> Result<bool> {
    state_manager
        .exists()
        .await
        .wrap_err("Failed to check if ecosystem exists")
}

/// Check if ecosystem contracts exist.
async fn contracts_exist(state_manager: &StateManager) -> Result<bool> {
    state_manager
        .ecosystem()
        .contracts_exist()
        .await
        .wrap_err("Failed to check if contracts exist")
}
