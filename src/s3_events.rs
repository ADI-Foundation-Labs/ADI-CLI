//! Spinner-based S3 event handler for CLI.
//!
//! This module provides an implementation of `S3SyncEventHandler` that
//! displays animated spinners using cliclack for S3 sync operations.

use adi_state::{S3SyncEvent, S3SyncEventHandler};
use async_trait::async_trait;
use cliclack::ProgressBar;
use console::style;
use tokio::sync::Mutex;

/// Event handler that shows S3 sync progress with cliclack spinners.
///
/// Creates a spinner when sync starts and updates it as the operation progresses.
pub struct SpinnerS3EventHandler {
    spinner: Mutex<Option<ProgressBar>>,
}

impl SpinnerS3EventHandler {
    /// Create a new spinner-based event handler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spinner: Mutex::new(None),
        }
    }
}

impl Default for SpinnerS3EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl S3SyncEventHandler for SpinnerS3EventHandler {
    async fn on_event(&self, event: S3SyncEvent) {
        match event {
            S3SyncEvent::SyncStarted { ecosystem_name } => {
                let spinner = cliclack::spinner();
                spinner.start(format!(
                    "Syncing {} to S3...",
                    style(&ecosystem_name).cyan()
                ));
                let mut guard = self.spinner.lock().await;
                *guard = Some(spinner);
            }
            S3SyncEvent::ArchiveCreated { size_bytes } => {
                let size_mb = crate::ui::bytes_to_mb(size_bytes);
                let guard = self.spinner.lock().await;
                if let Some(ref spinner) = *guard {
                    spinner.set_message(format!("Uploading to S3 ({size_mb:.2} MB)..."));
                }
            }
            S3SyncEvent::UploadComplete { key: _ } => {
                // Will be followed by SyncComplete
            }
            S3SyncEvent::SyncComplete => {
                let mut guard = self.spinner.lock().await;
                if let Some(spinner) = guard.take() {
                    spinner.stop(format!("{}", style("State synced to S3").green()));
                }
            }
            S3SyncEvent::SyncFailed { error } => {
                let mut guard = self.spinner.lock().await;
                if let Some(spinner) = guard.take() {
                    spinner.stop(format!("{}: {}", style("S3 sync failed").red(), error));
                }
            }
        }
    }
}
