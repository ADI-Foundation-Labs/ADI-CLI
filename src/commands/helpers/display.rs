//! Display helpers for ownership status and summary output.

use adi_ecosystem::{CalldataOutput, OwnershipState, OwnershipStatusSummary, OwnershipSummary};

use super::categorize_result;
use super::ResultCategory;
use crate::error::Result;
use crate::ui;

/// Display the ownership summary in a note box.
pub fn display_summary(title: &str, summary: &OwnershipSummary) -> Result<()> {
    let mut lines = vec![
        format!(
            "Successful: {}  Skipped: {}  Failed: {}",
            ui::green(summary.successful_count()),
            ui::cyan(summary.skipped_count()),
            ui::yellow(summary.failed_count())
        ),
        String::new(),
    ];

    for result in &summary.results {
        let line = match categorize_result(result) {
            ResultCategory::SuccessWithTx(tx) => {
                format!("{}: {}", result.name, ui::green(tx))
            }
            ResultCategory::SuccessNoTx => {
                format!("{}: {}", result.name, ui::green("success"))
            }
            ResultCategory::Skipped(reason) => {
                format!("{}: {}", result.name, ui::cyan(reason))
            }
            ResultCategory::Failed(error) => {
                format!("{}: {}", result.name, ui::yellow(error))
            }
        };
        lines.push(line);
    }

    ui::note(title, lines.join("\n"))?;
    Ok(())
}

/// Display ownership status for contracts in a note box.
pub fn display_ownership_status(title: &str, summary: &OwnershipStatusSummary) -> Result<()> {
    let lines: Vec<String> = summary
        .statuses
        .iter()
        .map(|status| match (status.address, status.state) {
            (Some(addr), OwnershipState::Pending) => {
                format!(
                    "{}: {} {}",
                    status.name,
                    ui::green(addr),
                    ui::yellow("(pending)")
                )
            }
            (Some(addr), OwnershipState::Accepted) => {
                format!(
                    "{}: {} {}",
                    status.name,
                    ui::green(addr),
                    ui::green("(accepted)")
                )
            }
            (Some(addr), OwnershipState::NotTransferred) => {
                format!(
                    "{}: {} {}",
                    status.name,
                    ui::green(addr),
                    ui::cyan("(no pending transfer)")
                )
            }
            (None, _) => {
                format!("{}: {}", status.name, ui::cyan("not configured"))
            }
        })
        .collect();

    ui::note(title, lines.join("\n"))?;
    Ok(())
}

/// Display calldata output for external submission.
pub fn display_calldata_output(title: &str, output: &CalldataOutput) -> Result<()> {
    if output.is_empty() {
        ui::note(title, "No pending ownership transfers")?;
        return Ok(());
    }

    let mut lines = Vec::new();
    for entry in &output.entries {
        lines.push(format!("{}", ui::cyan(&entry.name)));
        lines.push(format!("  To:       {}", ui::green(entry.to)));
        lines.push(format!("  Call:     {}", entry.description));
        lines.push(format!("  Calldata: {}", entry.calldata));
        lines.push(String::new());
    }

    // Remove trailing empty line
    if lines.last().is_some_and(|s| s.is_empty()) {
        lines.pop();
    }

    ui::note(title, lines.join("\n"))?;
    Ok(())
}
