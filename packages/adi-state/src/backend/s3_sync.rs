//! S3-synchronized filesystem backend.
//!
//! Wraps `FilesystemBackend` and syncs to S3 on write operations.

use crate::backend::{FilesystemBackend, StateBackend};
use crate::error::Result;
use crate::s3::{create_tar_gz, S3Client, S3Config};
use crate::s3::{NoOpS3EventHandler, S3SyncEvent, S3SyncEventHandler};
use adi_types::Logger;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Filesystem backend with S3 synchronization.
///
/// Delegates all operations to inner `FilesystemBackend`,
/// then syncs ecosystem directory to S3 on write operations.
pub struct S3SyncBackend {
    inner: FilesystemBackend,
    s3_client: S3Client,
    base_path: PathBuf,
    ecosystem_name: String,
    logger: Arc<dyn Logger>,
    event_handler: Arc<dyn S3SyncEventHandler>,
    auto_sync: AtomicBool,
}

impl S3SyncBackend {
    /// Create a new S3-synchronized backend with no-op event handler.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Ecosystem directory path
    /// * `ecosystem_name` - Name for the S3 archive
    /// * `config` - S3 configuration
    /// * `logger` - Logger instance
    ///
    /// # Errors
    ///
    /// Returns error if S3 client initialization fails.
    pub async fn new(
        base_path: &Path,
        ecosystem_name: &str,
        config: S3Config,
        logger: Arc<dyn Logger>,
    ) -> Result<Self> {
        Self::with_event_handler(
            base_path,
            ecosystem_name,
            config,
            logger,
            Arc::new(NoOpS3EventHandler),
        )
        .await
    }

    /// Create a new S3-synchronized backend with custom event handler.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Ecosystem directory path
    /// * `ecosystem_name` - Name for the S3 archive
    /// * `config` - S3 configuration
    /// * `logger` - Logger instance
    /// * `event_handler` - Handler for receiving sync progress events
    ///
    /// # Errors
    ///
    /// Returns error if S3 client initialization fails.
    pub async fn with_event_handler(
        base_path: &Path,
        ecosystem_name: &str,
        config: S3Config,
        logger: Arc<dyn Logger>,
        event_handler: Arc<dyn S3SyncEventHandler>,
    ) -> Result<Self> {
        let inner = FilesystemBackend::new(base_path, Arc::clone(&logger));
        let s3_client = S3Client::new(config).await?;

        Ok(Self {
            inner,
            s3_client,
            base_path: base_path.to_path_buf(),
            ecosystem_name: ecosystem_name.to_string(),
            logger,
            event_handler,
            auto_sync: AtomicBool::new(true),
        })
    }

    /// Enable or disable automatic sync after write operations.
    pub fn set_auto_sync(&self, enabled: bool) {
        self.auto_sync.store(enabled, Ordering::SeqCst);
    }

    /// Force sync to S3 regardless of auto_sync setting.
    ///
    /// # Errors
    ///
    /// Returns error if archive creation or S3 upload fails.
    pub async fn sync_now(&self) -> Result<()> {
        self.do_sync_to_s3().await
    }

    /// Sync current state to S3 if auto_sync is enabled.
    async fn sync_to_s3(&self) -> Result<()> {
        if !self.auto_sync.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.do_sync_to_s3().await
    }

    /// Perform actual sync to S3.
    async fn do_sync_to_s3(&self) -> Result<()> {
        self.event_handler
            .on_event(S3SyncEvent::SyncStarted {
                ecosystem_name: self.ecosystem_name.clone(),
            })
            .await;

        self.logger.debug(&format!(
            "Creating archive from {}",
            self.base_path.display()
        ));

        let archive_data = create_tar_gz(&self.base_path).await?;

        self.event_handler
            .on_event(S3SyncEvent::ArchiveCreated {
                size_bytes: archive_data.len(),
            })
            .await;

        let key = format!("{}.tar.gz", self.ecosystem_name);
        self.logger.debug(&format!(
            "Uploading to S3: {}{}",
            self.s3_client.key_prefix(),
            key
        ));

        self.s3_client.upload(&key, archive_data).await?;

        let full_key = format!("{}{}", self.s3_client.key_prefix(), key);
        self.event_handler
            .on_event(S3SyncEvent::UploadComplete { key: full_key })
            .await;

        self.event_handler.on_event(S3SyncEvent::SyncComplete).await;

        self.logger.debug("State synced to S3 successfully");
        Ok(())
    }
}

/// Control handle for S3 sync operations.
///
/// Allows disabling auto-sync for batch operations and
/// triggering manual sync when ready.
#[derive(Clone)]
pub struct S3SyncControl {
    backend: Arc<S3SyncBackend>,
}

impl S3SyncControl {
    /// Create control handle from backend.
    pub fn new(backend: Arc<S3SyncBackend>) -> Self {
        Self { backend }
    }

    /// Disable automatic sync after each write.
    pub fn disable_auto_sync(&self) {
        self.backend.set_auto_sync(false);
    }

    /// Enable automatic sync after each write.
    #[allow(dead_code)]
    pub fn enable_auto_sync(&self) {
        self.backend.set_auto_sync(true);
    }

    /// Manually trigger sync to S3.
    ///
    /// # Errors
    ///
    /// Returns error if archive creation or S3 upload fails.
    pub async fn sync_now(&self) -> Result<()> {
        self.backend.sync_now().await
    }
}

#[async_trait]
impl StateBackend for S3SyncBackend {
    async fn read_raw(&self, key: &str) -> Result<String> {
        self.inner.read_raw(key).await
    }

    async fn write_raw(&self, key: &str, content: &str) -> Result<()> {
        self.inner.write_raw(key, content).await?;
        self.sync_to_s3().await
    }

    async fn create_raw(&self, key: &str, content: &str) -> Result<()> {
        self.inner.create_raw(key, content).await?;
        self.sync_to_s3().await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.inner.exists(key).await
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        self.inner.list(prefix).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.inner.delete(key).await?;
        self.sync_to_s3().await
    }

    async fn delete_dir(&self, key: &str) -> Result<()> {
        self.inner.delete_dir(key).await?;
        self.sync_to_s3().await
    }
}
