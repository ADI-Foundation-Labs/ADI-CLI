//! State backend abstraction.

mod filesystem;

#[cfg(feature = "s3")]
mod s3_sync;

pub use filesystem::FilesystemBackend;

#[cfg(feature = "s3")]
pub use s3_sync::S3SyncBackend;

use crate::error::Result;
use adi_types::{
    Apps, ChainContracts, ChainMetadata, EcosystemContracts, EcosystemMetadata, Erc20Deployments,
    InitialDeployments, Logger, Wallets,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// Backend type for state storage.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// Filesystem-based storage (default).
    #[default]
    Filesystem,
    /// Filesystem with S3 sync on writes.
    #[cfg(feature = "s3")]
    #[serde(rename = "filesystem_s3_sync")]
    FilesystemWithS3Sync,
}

/// Abstract state storage backend with typed operations.
///
/// Provides typed read/write operations for domain types.
/// Serialization format is implementation-defined:
/// - `FilesystemBackend`: YAML files
/// - `DatabaseBackend`: native JSON/binary storage (future)
///
/// # Example
///
/// ```rust,ignore
/// use adi_state::backend::{StateBackend, FilesystemBackend};
/// use std::path::Path;
///
/// # async fn example() -> adi_state::Result<()> {
/// let backend = FilesystemBackend::new(Path::new("/home/user/.adi_cli/state/my_ecosystem"));
///
/// // Read typed data
/// let metadata = backend.read_ecosystem_metadata().await?;
///
/// // Check if file exists
/// let exists = backend.exists("ZkStack.yaml").await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait StateBackend: Send + Sync {
    // ========== RAW OPERATIONS ==========

    /// Read raw content from the given key (relative path).
    ///
    /// # Arguments
    ///
    /// * `key` - Relative path within state directory (e.g., "configs/wallets.yaml").
    ///
    /// # Errors
    ///
    /// Returns `StateError::NotFound` if the file does not exist.
    /// Returns `StateError::ReadFailed` for I/O errors.
    async fn read_raw(&self, key: &str) -> Result<String>;

    /// Write raw content to the given key (relative path).
    ///
    /// # Arguments
    ///
    /// * `key` - Relative path within state directory.
    /// * `content` - Raw content to write.
    ///
    /// # Errors
    ///
    /// Returns `StateError::NotFound` if the file does not exist.
    /// Returns `StateError::WriteFailed` for I/O errors.
    async fn write_raw(&self, key: &str, content: &str) -> Result<()>;

    /// Create a new raw entry at the given key (relative path).
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - Relative path within state directory.
    /// * `content` - Raw content to write.
    ///
    /// # Errors
    ///
    /// Returns `StateError::AlreadyExists` if the file already exists.
    /// Returns `StateError::WriteFailed` for I/O errors.
    async fn create_raw(&self, key: &str, content: &str) -> Result<()>;

    /// Check if a key (relative path) exists.
    ///
    /// # Arguments
    ///
    /// * `key` - Relative path within state directory.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// List all entries in a directory prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Directory prefix to list (e.g., "chains").
    ///
    /// Returns list of relative paths (directory names only for directories).
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Delete an entry at the given key (relative path).
    ///
    /// # Arguments
    ///
    /// * `key` - Relative path within state directory.
    ///
    /// # Errors
    ///
    /// Returns `StateError::NotFound` if the file does not exist.
    /// Returns `StateError::DeleteFailed` for I/O errors.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Delete a directory and all its contents recursively.
    ///
    /// # Arguments
    ///
    /// * `key` - Relative path to directory within state directory.
    ///
    /// # Errors
    ///
    /// Returns `StateError::NotFound` if the directory does not exist.
    /// Returns `StateError::DeleteFailed` for I/O errors.
    async fn delete_dir(&self, key: &str) -> Result<()>;

    // ========== ECOSYSTEM METADATA ==========

    /// Read ecosystem metadata (ZkStack.yaml).
    async fn read_ecosystem_metadata(&self) -> Result<EcosystemMetadata>;

    /// Write ecosystem metadata (ZkStack.yaml).
    async fn write_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()>;

    /// Create ecosystem metadata (ZkStack.yaml).
    async fn create_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()>;

    // ========== ECOSYSTEM WALLETS ==========

    /// Read ecosystem wallets (configs/wallets.yaml).
    async fn read_ecosystem_wallets(&self) -> Result<Wallets>;

    /// Write ecosystem wallets (configs/wallets.yaml).
    async fn write_ecosystem_wallets(&self, data: &Wallets) -> Result<()>;

    /// Create ecosystem wallets (configs/wallets.yaml).
    async fn create_ecosystem_wallets(&self, data: &Wallets) -> Result<()>;

    // ========== ECOSYSTEM CONTRACTS ==========

    /// Read ecosystem contracts (configs/contracts.yaml).
    async fn read_ecosystem_contracts(&self) -> Result<EcosystemContracts>;

    /// Write ecosystem contracts (configs/contracts.yaml).
    async fn write_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()>;

    /// Create ecosystem contracts (configs/contracts.yaml).
    async fn create_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()>;

    // ========== INITIAL DEPLOYMENTS ==========

    /// Read initial deployments (configs/initial_deployments.yaml).
    async fn read_initial_deployments(&self) -> Result<InitialDeployments>;

    /// Write initial deployments (configs/initial_deployments.yaml).
    async fn write_initial_deployments(&self, data: &InitialDeployments) -> Result<()>;

    /// Create initial deployments (configs/initial_deployments.yaml).
    async fn create_initial_deployments(&self, data: &InitialDeployments) -> Result<()>;

    // ========== ERC20 DEPLOYMENTS ==========

    /// Read ERC20 deployments (configs/erc20_deployments.yaml).
    async fn read_erc20_deployments(&self) -> Result<Erc20Deployments>;

    /// Write ERC20 deployments (configs/erc20_deployments.yaml).
    async fn write_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()>;

    /// Create ERC20 deployments (configs/erc20_deployments.yaml).
    async fn create_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()>;

    // ========== APPS ==========

    /// Read apps config (configs/apps.yaml).
    async fn read_apps(&self) -> Result<Apps>;

    /// Write apps config (configs/apps.yaml).
    async fn write_apps(&self, data: &Apps) -> Result<()>;

    /// Create apps config (configs/apps.yaml).
    async fn create_apps(&self, data: &Apps) -> Result<()>;

    // ========== CHAIN METADATA ==========

    /// Read chain metadata (chains/{chain}/ZkStack.yaml).
    async fn read_chain_metadata(&self, chain: &str) -> Result<ChainMetadata>;

    /// Write chain metadata (chains/{chain}/ZkStack.yaml).
    async fn write_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()>;

    /// Create chain metadata (chains/{chain}/ZkStack.yaml).
    async fn create_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()>;

    // ========== CHAIN WALLETS ==========

    /// Read chain wallets (chains/{chain}/configs/wallets.yaml).
    async fn read_chain_wallets(&self, chain: &str) -> Result<Wallets>;

    /// Write chain wallets (chains/{chain}/configs/wallets.yaml).
    async fn write_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()>;

    /// Create chain wallets (chains/{chain}/configs/wallets.yaml).
    async fn create_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()>;

    // ========== CHAIN CONTRACTS ==========

    /// Read chain contracts (chains/{chain}/configs/contracts.yaml).
    async fn read_chain_contracts(&self, chain: &str) -> Result<ChainContracts>;

    /// Write chain contracts (chains/{chain}/configs/contracts.yaml).
    async fn write_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()>;

    /// Create chain contracts (chains/{chain}/configs/contracts.yaml).
    async fn create_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()>;
}

/// Create a backend instance based on backend type.
///
/// # Arguments
///
/// * `backend_type` - The type of backend to create.
/// * `base_path` - Base path for the backend (interpretation depends on type).
/// * `logger` - Logger for debug messages.
///
/// # Panics
///
/// Panics if `FilesystemWithS3Sync` is requested. Use `create_s3_sync_backend` instead.
#[must_use]
pub fn create_backend(
    backend_type: BackendType,
    base_path: &Path,
    logger: Arc<dyn Logger>,
) -> Box<dyn StateBackend> {
    match backend_type {
        BackendType::Filesystem => Box::new(FilesystemBackend::new(base_path, logger)),
        #[cfg(feature = "s3")]
        BackendType::FilesystemWithS3Sync => {
            // S3SyncBackend requires async initialization
            // Use create_s3_sync_backend() instead
            unreachable!("Use create_s3_sync_backend() for FilesystemWithS3Sync backend")
        }
    }
}

/// Create an S3-synchronized backend.
///
/// # Arguments
///
/// * `base_path` - Ecosystem directory path.
/// * `ecosystem_name` - Name for the S3 archive.
/// * `config` - S3 configuration.
/// * `logger` - Logger for debug messages.
///
/// # Errors
///
/// Returns error if S3 client initialization fails.
#[cfg(feature = "s3")]
pub async fn create_s3_sync_backend(
    base_path: &Path,
    ecosystem_name: &str,
    config: crate::s3::S3Config,
    logger: Arc<dyn Logger>,
) -> crate::Result<Box<dyn StateBackend>> {
    let backend = S3SyncBackend::new(base_path, ecosystem_name, config, logger).await?;
    Ok(Box::new(backend))
}
