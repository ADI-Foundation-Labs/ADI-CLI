//! Event system for S3 sync progress reporting.
//!
//! This module provides an event handler pattern for receiving progress
//! updates during S3 synchronization operations. This allows CLI applications
//! to show spinners or progress indicators without coupling the S3 backend
//! to a specific UI framework.

use async_trait::async_trait;

/// Events emitted during S3 sync operations.
#[derive(Clone, Debug)]
pub enum S3SyncEvent {
    /// Sync operation started (creating archive).
    SyncStarted {
        /// Ecosystem name being synced.
        ecosystem_name: String,
    },
    /// Archive created, uploading to S3.
    ArchiveCreated {
        /// Archive size in bytes.
        size_bytes: usize,
    },
    /// Upload complete.
    UploadComplete {
        /// S3 key where archive was uploaded.
        key: String,
    },
    /// Sync operation completed successfully.
    SyncComplete,
    /// Sync operation failed.
    SyncFailed {
        /// Error message.
        error: String,
    },
}

/// Trait for receiving S3 sync events.
///
/// Implement this trait to receive progress updates during S3 sync operations.
/// This enables CLI applications to show spinners or progress bars without
/// coupling the S3 backend to specific UI frameworks.
///
/// # Example
///
/// ```rust,ignore
/// use adi_state::s3::{S3SyncEvent, S3SyncEventHandler};
///
/// struct MyHandler;
///
/// #[async_trait::async_trait]
/// impl S3SyncEventHandler for MyHandler {
///     async fn on_event(&self, event: S3SyncEvent) {
///         match event {
///             S3SyncEvent::SyncStarted { ecosystem_name } => {
///                 println!("Syncing {}...", ecosystem_name);
///             }
///             S3SyncEvent::SyncComplete => {
///                 println!("Done!");
///             }
///             _ => {}
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait S3SyncEventHandler: Send + Sync {
    /// Handle an S3 sync event.
    async fn on_event(&self, event: S3SyncEvent);
}

/// No-op event handler that discards all events.
///
/// This is the default handler for SDK consumers who don't need UI feedback.
pub struct NoOpS3EventHandler;

#[async_trait]
impl S3SyncEventHandler for NoOpS3EventHandler {
    async fn on_event(&self, _event: S3SyncEvent) {}
}
