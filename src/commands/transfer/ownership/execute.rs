//! Execution phases for the transfer ownership command.
//!
//! Contains ownership status checks, confirmation prompt, accept/transfer
//! execution, and result display logic.

use adi_ecosystem::{
    accept_all_ownership, accept_chain_ownership, check_chain_ownership_status,
    check_ecosystem_ownership_status, transfer_all_ownership, transfer_chain_ownership,
    OwnershipSummary,
};

use crate::commands::helpers::{
    derive_address_from_key, display_ownership_status, display_summary,
};
use crate::error::{Result, WrapErr};
use crate::ui;

use super::TransferConfig;

/// Aggregated results from accept and transfer phases.
pub(in crate::commands::transfer::ownership) struct TransferSummaries {
    pub(in crate::commands::transfer::ownership) ecosystem_accept: Option<OwnershipSummary>,
    pub(in crate::commands::transfer::ownership) ecosystem_transfer: Option<OwnershipSummary>,
    pub(in crate::commands::transfer::ownership) chain_accept: Option<OwnershipSummary>,
    pub(in crate::commands::transfer::ownership) chain_transfer: Option<OwnershipSummary>,
}

/// Check and display ecosystem/chain ownership statuses, warn on pending transfers.
pub(super) async fn check_ownership_statuses(config: &TransferConfig<'_>) -> Result<()> {
    let mut total_pending: usize = 0;

    if let Some((ref contracts, ref governor)) = config.ecosystem_data {
        let governor_address = derive_address_from_key(&governor.private_key)?;
        ui::info("Checking ecosystem ownership status...")?;
        let status = check_ecosystem_ownership_status(
            config.rpc_url.as_str(),
            contracts,
            governor_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check ecosystem ownership status")?;
        display_ownership_status("Ecosystem contracts", &status)?;
        total_pending += status.pending_count();
    }

    if let (Some(ref name), Some((ref contracts, ref governor))) =
        (&config.chain_name, &config.chain_data)
    {
        let governor_address = derive_address_from_key(&governor.private_key)?;
        ui::info(format!("Checking chain '{}' ownership status...", name))?;
        let status = check_chain_ownership_status(
            config.rpc_url.as_str(),
            contracts,
            governor_address,
            config.context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to check chain ownership status")?;
        display_ownership_status(&format!("Chain '{}' contracts", name), &status)?;
        total_pending += status.pending_count();
    }

    if total_pending > 0 {
        ui::warning(format!(
            "{} contract(s) have pending ownership transfers.",
            total_pending
        ))?;
    }

    Ok(())
}

/// Build confirmation message and prompt the user. Returns `true` if confirmed.
pub(super) fn confirm_transfer(config: &TransferConfig<'_>, skip_confirm: bool) -> Result<bool> {
    let msg = match (config.ecosystem_new_owner, config.chain_new_owner) {
        (Some(eco), Some(chain)) if eco == chain => {
            format!("Proceed with ownership transfer to {}?", ui::green(eco))
        }
        (Some(eco), Some(chain)) => {
            format!(
                "Proceed with ownership transfer?\n  Ecosystem → {}\n  Chain → {}",
                ui::green(eco),
                ui::green(chain)
            )
        }
        (Some(eco), None) => {
            format!(
                "Proceed with ecosystem ownership transfer to {}?",
                ui::green(eco)
            )
        }
        (None, Some(chain)) => {
            format!(
                "Proceed with chain ownership transfer to {}?",
                ui::green(chain)
            )
        }
        (None, None) => return Err(eyre::eyre!("No new owner specified for transfer")),
    };

    if skip_confirm {
        return Ok(true);
    }

    ui::confirm(msg)
        .initial_value(true)
        .interact()
        .wrap_err("Failed to get confirmation")
}

/// Run the accept phase then the transfer phase.
pub(super) async fn execute_phases(config: &TransferConfig<'_>) -> Result<TransferSummaries> {
    let mut summaries = TransferSummaries {
        ecosystem_accept: None,
        ecosystem_transfer: None,
        chain_accept: None,
        chain_transfer: None,
    };

    // Accept phase
    ui::section("Accept Phase")?;

    if let Some((ref contracts, ref governor)) = config.ecosystem_data {
        ui::info("Processing ecosystem contracts...")?;
        let summary = accept_all_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Ecosystem Accept Summary", &summary)?;
        summaries.ecosystem_accept = Some(summary);
    }

    if let Some((ref contracts, ref governor)) = config.chain_data {
        ui::info("Processing chain contracts...")?;
        let summary = accept_chain_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Chain Accept Summary", &summary)?;
        summaries.chain_accept = Some(summary);
    }

    // Transfer phase
    ui::section("Transfer Phase")?;

    if let (Some((ref contracts, ref governor)), Some(new_owner)) =
        (&config.ecosystem_data, config.ecosystem_new_owner)
    {
        ui::info("Transferring ecosystem contracts...")?;
        let summary = transfer_all_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            new_owner,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Ecosystem Transfer Summary", &summary)?;
        summaries.ecosystem_transfer = Some(summary);
    }

    if let (Some((ref contracts, ref governor)), Some(new_owner)) =
        (&config.chain_data, config.chain_new_owner)
    {
        ui::info("Transferring chain contracts...")?;
        let summary = transfer_chain_ownership(
            config.rpc_url.as_str(),
            contracts,
            &governor.private_key,
            new_owner,
            config.gas_multiplier,
            config.context.logger().as_ref(),
        )
        .await;
        display_summary("Chain Transfer Summary", &summary)?;
        summaries.chain_transfer = Some(summary);
    }

    Ok(summaries)
}
