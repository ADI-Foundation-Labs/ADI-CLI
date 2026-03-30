//! Execution logic for the accept ownership command.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, check_ecosystem_ownership_status_for_new_owner,
    collect_all_ownership_calldata, collect_chain_ownership_calldata, OwnershipStatusSummary,
    OwnershipSummary,
};

use crate::commands::helpers::{
    display_calldata_output, display_ownership_status, display_summary,
};
use crate::error::{Result, WrapErr};
use crate::ui;

use super::config::AcceptConfig;

/// Check ecosystem and chain ownership statuses. Returns pending counts.
pub(super) async fn check_statuses(
    config: &AcceptConfig<'_>,
) -> Result<(
    Option<OwnershipStatusSummary>,
    Option<OwnershipStatusSummary>,
)> {
    let ecosystem_status = check_ecosystem_status(config).await?;
    let chain_status = check_chain_status(config).await?;
    Ok((ecosystem_status, chain_status))
}

/// Check ecosystem ownership status using the appropriate mode.
async fn check_ecosystem_status(
    config: &AcceptConfig<'_>,
) -> Result<Option<OwnershipStatusSummary>> {
    let contracts = match config.ecosystem_contracts {
        Some(ref c) => c,
        None => return Ok(None),
    };

    ui::info("Checking ecosystem ownership status...")?;

    let status = if config.is_governor_mode {
        check_ecosystem_ownership_status(
            config.rpc_url.as_str(),
            contracts,
            config.key_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check ecosystem ownership status")?
    } else {
        check_ecosystem_ownership_status_for_new_owner(
            config.rpc_url.as_str(),
            contracts,
            config.key_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check ecosystem ownership status")?
    };

    display_ownership_status("Ecosystem contracts", &status)?;
    Ok(Some(status))
}

/// Check chain ownership status if chain contracts are present.
async fn check_chain_status(config: &AcceptConfig<'_>) -> Result<Option<OwnershipStatusSummary>> {
    let contracts = match config.chain_contracts {
        Some(ref c) => c,
        None => return Ok(None),
    };

    let chain_name = config.chain_name.as_deref().unwrap_or("unknown");
    ui::info(format!(
        "Checking chain '{}' ownership status...",
        chain_name
    ))?;

    let status = check_chain_ownership_status(
        config.rpc_url.as_str(),
        contracts,
        config.key_address,
        config.context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to check chain ownership status")?;

    display_ownership_status(&format!("Chain '{}' contracts", chain_name), &status)?;
    Ok(Some(status))
}

/// Collect and display calldata for ecosystem and chain contracts.
pub(super) async fn collect_calldata(config: &AcceptConfig<'_>) -> Result<()> {
    ui::info("Collecting calldata for pending contracts...")?;

    if let Some(ref contracts) = config.ecosystem_contracts {
        let calldata = collect_all_ownership_calldata(
            config.rpc_url.as_str(),
            contracts,
            config.key_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to collect ecosystem calldata")?;
        display_calldata_output("Ecosystem Calldata", &calldata)?;
    }

    if let Some(ref contracts) = config.chain_contracts {
        let calldata = collect_chain_ownership_calldata(
            config.rpc_url.as_str(),
            contracts,
            config.key_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to collect chain calldata")?;
        display_calldata_output("Chain Calldata", &calldata)?;
    }

    ui::outro("Calldata collection complete")?;
    Ok(())
}

/// Execute ownership acceptance for ecosystem and chain contracts.
pub(super) async fn execute_acceptance(
    config: &AcceptConfig<'_>,
) -> Result<(Option<OwnershipSummary>, Option<OwnershipSummary>)> {
    let ecosystem_summary = if let Some(ref contracts) = config.ecosystem_contracts {
        ui::info("Processing ecosystem contracts...")?;
        let summary = accept_all_ownership(
            config.rpc_url.as_str(),
            contracts,
            &config.private_key,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Ecosystem Summary", &summary)?;
        Some(summary)
    } else {
        None
    };

    let chain_summary = if let Some(ref contracts) = config.chain_contracts {
        ui::info("Processing chain contracts...")?;
        let summary = accept_chain_ownership(
            config.rpc_url.as_str(),
            contracts,
            &config.private_key,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Chain Summary", &summary)?;
        Some(summary)
    } else {
        None
    };

    Ok((ecosystem_summary, chain_summary))
}

/// Evaluate acceptance results and produce final UI output.
pub(super) fn evaluate_results(
    ecosystem_summary: &Option<OwnershipSummary>,
    chain_summary: &Option<OwnershipSummary>,
) -> Result<()> {
    let total_successes = ecosystem_summary
        .as_ref()
        .map_or(0, |s| s.successful_count())
        + chain_summary.as_ref().map_or(0, |s| s.successful_count());

    let total_results = ecosystem_summary.as_ref().map_or(0, |s| s.results.len())
        + chain_summary.as_ref().map_or(0, |s| s.results.len());

    if total_results == 0 {
        ui::outro("No contracts were processed")?;
        return Ok(());
    }

    if total_successes == 0 {
        return Err(eyre::eyre!("All ownership acceptances failed"));
    }

    ui::outro("Ownership acceptance complete!")?;
    Ok(())
}
