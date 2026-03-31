//! Spinner-based event handler for transfer progress.

use super::{FundingEvent, FundingEventHandler};
use cliclack::ProgressBar;
use console::style;
use std::sync::Mutex;

/// Event handler that shows transfer progress with cliclack spinners.
///
/// Each transfer gets its own spinner that shows progress and completion status.
pub struct SpinnerEventHandler {
    spinner: Mutex<Option<ProgressBar>>,
}

impl SpinnerEventHandler {
    /// Create a new SpinnerEventHandler.
    pub fn new() -> Self {
        Self {
            spinner: Mutex::new(None),
        }
    }
}

impl Default for SpinnerEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl FundingEventHandler for SpinnerEventHandler {
    async fn on_event(&self, event: FundingEvent) {
        match event {
            FundingEvent::TransferStarted {
                index,
                total,
                transfer,
            } => {
                let spinner = cliclack::spinner();
                spinner.start(format!(
                    "{} {} to {} ({})",
                    style(format!("[{}/{}]", index + 1, total)).magenta(),
                    style(transfer.amount_description()).green(),
                    style(transfer.to).green(),
                    style(transfer.role).cyan()
                ));
                if let Ok(mut guard) = self.spinner.lock() {
                    *guard = Some(spinner);
                }
            }
            FundingEvent::TransferConfirmed {
                index,
                tx_hash,
                gas_used,
            } => {
                if let Ok(mut guard) = self.spinner.lock() {
                    if let Some(spinner) = guard.take() {
                        spinner.stop(format!(
                            "{} Confirmed: {} (gas: {})",
                            style(format!("[{}]", index + 1)).magenta(),
                            style(tx_hash).green(),
                            style(gas_used).green()
                        ));
                    }
                }
            }
            // Ignore other events - they're handled by the UI layer
            _ => {}
        }
    }
}
