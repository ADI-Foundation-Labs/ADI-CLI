//! Result display and next-step instructions for the transfer ownership command.

use crate::error::Result;
use crate::ui;

use super::execute::TransferSummaries;
use super::TransferConfig;

/// Aggregate results and display final status with next-step instructions.
pub(super) fn display_results(
    config: &TransferConfig<'_>,
    summaries: &TransferSummaries,
) -> Result<()> {
    let total_successes = [
        &summaries.ecosystem_accept,
        &summaries.chain_accept,
        &summaries.ecosystem_transfer,
        &summaries.chain_transfer,
    ]
    .iter()
    .filter_map(|s| s.as_ref())
    .map(|s| s.successful_count())
    .sum::<usize>();

    let total_results: usize = [
        &summaries.ecosystem_accept,
        &summaries.chain_accept,
        &summaries.ecosystem_transfer,
        &summaries.chain_transfer,
    ]
    .iter()
    .filter_map(|s| s.as_ref())
    .map(|s| s.results.len())
    .sum();

    if total_results == 0 {
        ui::outro("No contracts were processed")?;
        return Ok(());
    }

    if total_successes == 0 {
        return Err(eyre::eyre!("All ownership operations failed"));
    }

    display_next_steps(config)?;

    ui::outro(format!(
        "Transfer complete! {} operation(s) processed.",
        total_successes
    ))?;
    Ok(())
}

/// Display next-step instructions based on the transfer scope.
fn display_next_steps(config: &TransferConfig<'_>) -> Result<()> {
    let msg = match (config.ecosystem_new_owner, config.chain_new_owner) {
        (Some(eco), Some(chain)) if eco == chain => {
            format!(
                "New owner {} must accept ownership:\n\n  {}",
                ui::green(eco),
                ui::cyan("adi accept --private-key <NEW_OWNER_PRIVATE_KEY>")
            )
        }
        (Some(eco), Some(chain)) => {
            format!(
                "New owners must accept ownership:\n\n  Ecosystem ({}): {}\n  Chain ({}): {}",
                ui::green(eco),
                ui::cyan("adi accept --scope ecosystem --private-key <KEY>"),
                ui::green(chain),
                ui::cyan("adi accept --scope chain --private-key <KEY>")
            )
        }
        (Some(eco), None) => {
            format!(
                "New owner {} must accept ecosystem ownership:\n\n  {}",
                ui::green(eco),
                ui::cyan("adi accept --scope ecosystem --private-key <NEW_OWNER_PRIVATE_KEY>")
            )
        }
        (None, Some(chain)) => {
            format!(
                "New owner {} must accept chain ownership:\n\n  {}",
                ui::green(chain),
                ui::cyan("adi accept --scope chain --private-key <NEW_OWNER_PRIVATE_KEY>")
            )
        }
        (None, None) => return Ok(()),
    };

    ui::note("Next step", msg)?;
    Ok(())
}
