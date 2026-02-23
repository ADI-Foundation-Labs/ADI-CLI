//! S3-synchronized filesystem backend.
//!
//! Wraps `FilesystemBackend` and syncs to S3 on write operations.

use crate::backend::{FilesystemBackend, StateBackend};
use crate::error::Result;
use crate::s3::{create_tar_gz, S3Client, S3Config};
use adi_types::{
    Apps, ChainContracts, ChainMetadata, EcosystemContracts, EcosystemMetadata, Erc20Deployments,
    InitialDeployments, Logger, Wallets,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
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
}

impl S3SyncBackend {
    /// Create a new S3-synchronized backend.
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
        let inner = FilesystemBackend::new(base_path, Arc::clone(&logger));
        let s3_client = S3Client::new(config).await?;

        Ok(Self {
            inner,
            s3_client,
            base_path: base_path.to_path_buf(),
            ecosystem_name: ecosystem_name.to_string(),
            logger,
        })
    }

    /// Sync current state to S3.
    async fn sync_to_s3(&self) -> Result<()> {
        self.logger.debug("Syncing state to S3...");

        // Create tar.gz archive
        let archive_data = create_tar_gz(&self.base_path).await?;

        // Upload to S3
        let key = format!("{}.tar.gz", self.ecosystem_name);
        self.s3_client.upload(&key, archive_data).await?;

        self.logger.debug("State synced to S3 successfully");
        Ok(())
    }
}

#[async_trait]
impl StateBackend for S3SyncBackend {
    // ========== RAW OPERATIONS ==========

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

    // ========== ECOSYSTEM METADATA ==========

    async fn read_ecosystem_metadata(&self) -> Result<EcosystemMetadata> {
        self.inner.read_ecosystem_metadata().await
    }

    async fn write_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()> {
        self.inner.write_ecosystem_metadata(data).await?;
        self.sync_to_s3().await
    }

    async fn create_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()> {
        self.inner.create_ecosystem_metadata(data).await?;
        self.sync_to_s3().await
    }

    // ========== ECOSYSTEM WALLETS ==========

    async fn read_ecosystem_wallets(&self) -> Result<Wallets> {
        self.inner.read_ecosystem_wallets().await
    }

    async fn write_ecosystem_wallets(&self, data: &Wallets) -> Result<()> {
        self.inner.write_ecosystem_wallets(data).await?;
        self.sync_to_s3().await
    }

    async fn create_ecosystem_wallets(&self, data: &Wallets) -> Result<()> {
        self.inner.create_ecosystem_wallets(data).await?;
        self.sync_to_s3().await
    }

    // ========== ECOSYSTEM CONTRACTS ==========

    async fn read_ecosystem_contracts(&self) -> Result<EcosystemContracts> {
        self.inner.read_ecosystem_contracts().await
    }

    async fn write_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()> {
        self.inner.write_ecosystem_contracts(data).await?;
        self.sync_to_s3().await
    }

    async fn create_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()> {
        self.inner.create_ecosystem_contracts(data).await?;
        self.sync_to_s3().await
    }

    // ========== INITIAL DEPLOYMENTS ==========

    async fn read_initial_deployments(&self) -> Result<InitialDeployments> {
        self.inner.read_initial_deployments().await
    }

    async fn write_initial_deployments(&self, data: &InitialDeployments) -> Result<()> {
        self.inner.write_initial_deployments(data).await?;
        self.sync_to_s3().await
    }

    async fn create_initial_deployments(&self, data: &InitialDeployments) -> Result<()> {
        self.inner.create_initial_deployments(data).await?;
        self.sync_to_s3().await
    }

    // ========== ERC20 DEPLOYMENTS ==========

    async fn read_erc20_deployments(&self) -> Result<Erc20Deployments> {
        self.inner.read_erc20_deployments().await
    }

    async fn write_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()> {
        self.inner.write_erc20_deployments(data).await?;
        self.sync_to_s3().await
    }

    async fn create_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()> {
        self.inner.create_erc20_deployments(data).await?;
        self.sync_to_s3().await
    }

    // ========== APPS ==========

    async fn read_apps(&self) -> Result<Apps> {
        self.inner.read_apps().await
    }

    async fn write_apps(&self, data: &Apps) -> Result<()> {
        self.inner.write_apps(data).await?;
        self.sync_to_s3().await
    }

    async fn create_apps(&self, data: &Apps) -> Result<()> {
        self.inner.create_apps(data).await?;
        self.sync_to_s3().await
    }

    // ========== CHAIN METADATA ==========

    async fn read_chain_metadata(&self, chain: &str) -> Result<ChainMetadata> {
        self.inner.read_chain_metadata(chain).await
    }

    async fn write_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()> {
        self.inner.write_chain_metadata(chain, data).await?;
        self.sync_to_s3().await
    }

    async fn create_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()> {
        self.inner.create_chain_metadata(chain, data).await?;
        self.sync_to_s3().await
    }

    // ========== CHAIN WALLETS ==========

    async fn read_chain_wallets(&self, chain: &str) -> Result<Wallets> {
        self.inner.read_chain_wallets(chain).await
    }

    async fn write_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()> {
        self.inner.write_chain_wallets(chain, data).await?;
        self.sync_to_s3().await
    }

    async fn create_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()> {
        self.inner.create_chain_wallets(chain, data).await?;
        self.sync_to_s3().await
    }

    // ========== CHAIN CONTRACTS ==========

    async fn read_chain_contracts(&self, chain: &str) -> Result<ChainContracts> {
        self.inner.read_chain_contracts(chain).await
    }

    async fn write_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()> {
        self.inner.write_chain_contracts(chain, data).await?;
        self.sync_to_s3().await
    }

    async fn create_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()> {
        self.inner.create_chain_contracts(chain, data).await?;
        self.sync_to_s3().await
    }
}
