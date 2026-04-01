//! Verification status checking with bounded concurrency.

use adi_ecosystem::verification::{ExplorerClient, VerificationStatus, VerificationTarget};
use futures_util::{stream, StreamExt};
use std::sync::Arc;

use crate::error::Result;
use crate::ui;

/// Maximum concurrent verification status checks.
const MAX_CONCURRENT_CHECKS: usize = 5;

/// Result of a verification status check for display purposes.
pub(super) enum CheckResult {
    Verified,
    NotVerified,
    Pending,
    Unknown(String),
    Error(String),
}

/// Counts from a verification status check run.
pub(super) struct StatusCounts {
    pub verified: usize,
    pub unverified: usize,
    pub errors: usize,
}

/// Run concurrent verification status checks with progress bar and Ctrl+C support.
/// Returns `None` if interrupted by the user.
pub(super) async fn check_verification_status(
    targets: &[VerificationTarget],
    explorer_client: &Arc<ExplorerClient>,
) -> Result<Option<(Vec<(String, CheckResult)>, StatusCounts)>> {
    let progress = cliclack::progress_bar(targets.len() as u64);
    progress.start("Checking verification status...");

    let check_futures = targets.iter().enumerate().map(|(idx, target)| {
        let client = Arc::clone(explorer_client);
        let name = target.contract_type.display_name().to_string();
        let address = target.address;

        async move {
            let result = match client.check_verification_status(address).await {
                Ok(VerificationStatus::Verified) => CheckResult::Verified,
                Ok(VerificationStatus::NotVerified) => CheckResult::NotVerified,
                Ok(VerificationStatus::Pending) => CheckResult::Pending,
                Ok(VerificationStatus::Unknown(msg)) => CheckResult::Unknown(msg),
                Err(e) => CheckResult::Error(e.to_string()),
            };
            (idx, name, result)
        }
    });

    let mut check_stream = stream::iter(check_futures).buffer_unordered(MAX_CONCURRENT_CHECKS);
    let mut indexed_results: Vec<(usize, String, CheckResult)> = Vec::with_capacity(targets.len());
    let mut interrupted = false;

    loop {
        tokio::select! {
            biased;

            _ = tokio::signal::ctrl_c() => {
                interrupted = true;
                progress.stop("Interrupted by user");
                break;
            }

            result = check_stream.next() => {
                match result {
                    Some((idx, name, check_result)) => {
                        indexed_results.push((idx, name, check_result));
                        progress.inc(1);
                    }
                    None => break,
                }
            }
        }
    }

    if interrupted {
        return Ok(None);
    }

    progress.stop("Verification status check complete");

    // Sort by original index for consistent display ordering
    indexed_results.sort_by_key(|(idx, _, _)| *idx);

    let mut counts = StatusCounts {
        verified: 0,
        unverified: 0,
        errors: 0,
    };
    let mut results: Vec<(String, CheckResult)> = Vec::new();

    for (_, name, result) in indexed_results {
        match &result {
            CheckResult::Verified => counts.verified += 1,
            CheckResult::NotVerified | CheckResult::Unknown(_) => counts.unverified += 1,
            CheckResult::Error(_) => counts.errors += 1,
            CheckResult::Pending => {}
        }
        results.push((name, result));
    }

    Ok(Some((results, counts)))
}

/// Display status results table and summary counts.
pub(super) fn display_status(
    results: &[(String, CheckResult)],
    counts: &StatusCounts,
) -> Result<()> {
    let results_text = format_check_results(results);
    ui::note("Verification Status", results_text)?;

    ui::note(
        "Status Summary",
        format!(
            "Verified: {}  Unverified: {}  Errors: {}",
            ui::green(counts.verified),
            ui::yellow(counts.unverified),
            if counts.errors > 0 {
                ui::red(counts.errors).to_string()
            } else {
                ui::dim("0").to_string()
            }
        ),
    )?;

    Ok(())
}

/// Format check results for display.
fn format_check_results(results: &[(String, CheckResult)]) -> String {
    results
        .iter()
        .map(|(name, result)| match result {
            CheckResult::Verified => {
                format!("{}  {} -> {}", ui::green("✓"), name, ui::green("Verified"))
            }
            CheckResult::NotVerified => {
                format!(
                    "{}  {} -> {}",
                    ui::yellow("✗"),
                    name,
                    ui::yellow("Not Verified")
                )
            }
            CheckResult::Pending => {
                format!("{}  {} -> {}", ui::cyan("○"), name, ui::cyan("Pending"))
            }
            CheckResult::Unknown(msg) => {
                format!(
                    "{}  {} -> {}",
                    ui::yellow("?"),
                    name,
                    ui::yellow(format!("Unknown: {}", msg))
                )
            }
            CheckResult::Error(msg) => {
                format!(
                    "{}  {} -> {}",
                    ui::red("✗"),
                    name,
                    ui::red(format!("Error: {}", msg))
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
