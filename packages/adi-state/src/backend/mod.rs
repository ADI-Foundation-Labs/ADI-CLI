//! State backend abstraction.

mod filesystem;
mod s3_sync;

pub use filesystem::FilesystemBackend;
pub use s3_sync::{S3SyncBackend, S3SyncControl};

use crate::error::{Result, StateError};
use crate::paths;
use adi_types::{
    Apps, ChainContracts, ChainMetadata, EcosystemContracts, EcosystemMetadata, Erc20Deployments,
    InitialDeployments, Logger, Operators, Wallets,
};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Serialize a value to YAML string with path context for errors.
fn serialize_yaml<T: Serialize>(value: &T, key: &str) -> Result<String> {
    serde_yaml::to_string(value).map_err(|e| StateError::YamlSerializeFailed {
        path: PathBuf::from(key),
        source: e,
    })
}

/// Deserialize a YAML string to a typed value with path context for errors.
fn deserialize_yaml<T: DeserializeOwned>(content: &str, key: &str) -> Result<T> {
    serde_yaml::from_str(content).map_err(|e| StateError::YamlParseFailed {
        path: PathBuf::from(key),
        source: e,
    })
}

/// Backend type for state storage.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// Filesystem-based storage (default).
    #[default]
    Filesystem,
    /// Filesystem with S3 sync on writes.
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
    async fn read_ecosystem_metadata(&self) -> Result<EcosystemMetadata> {
        let content = self.read_raw(paths::ECOSYSTEM_METADATA).await?;
        deserialize_yaml(&content, paths::ECOSYSTEM_METADATA)
    }

    /// Write ecosystem metadata (ZkStack.yaml).
    async fn write_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()> {
        let yaml = serialize_yaml(data, paths::ECOSYSTEM_METADATA)?;
        self.write_raw(paths::ECOSYSTEM_METADATA, &yaml).await
    }

    /// Create ecosystem metadata (ZkStack.yaml).
    async fn create_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()> {
        let yaml = serialize_yaml(data, paths::ECOSYSTEM_METADATA)?;
        self.create_raw(paths::ECOSYSTEM_METADATA, &yaml).await
    }

    // ========== ECOSYSTEM WALLETS ==========

    /// Read ecosystem wallets (configs/wallets.yaml).
    async fn read_ecosystem_wallets(&self) -> Result<Wallets> {
        let key = paths::ecosystem_wallets_path();
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write ecosystem wallets (configs/wallets.yaml).
    async fn write_ecosystem_wallets(&self, data: &Wallets) -> Result<()> {
        let key = paths::ecosystem_wallets_path();
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create ecosystem wallets (configs/wallets.yaml).
    async fn create_ecosystem_wallets(&self, data: &Wallets) -> Result<()> {
        let key = paths::ecosystem_wallets_path();
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== ECOSYSTEM CONTRACTS ==========

    /// Read ecosystem contracts (configs/contracts.yaml).
    async fn read_ecosystem_contracts(&self) -> Result<EcosystemContracts> {
        let key = paths::ecosystem_contracts_path();
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write ecosystem contracts (configs/contracts.yaml).
    async fn write_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()> {
        let key = paths::ecosystem_contracts_path();
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create ecosystem contracts (configs/contracts.yaml).
    async fn create_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()> {
        let key = paths::ecosystem_contracts_path();
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== INITIAL DEPLOYMENTS ==========

    /// Read initial deployments (configs/initial_deployments.yaml).
    async fn read_initial_deployments(&self) -> Result<InitialDeployments> {
        let key = paths::initial_deployments_path();
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write initial deployments (configs/initial_deployments.yaml).
    async fn write_initial_deployments(&self, data: &InitialDeployments) -> Result<()> {
        let key = paths::initial_deployments_path();
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create initial deployments (configs/initial_deployments.yaml).
    async fn create_initial_deployments(&self, data: &InitialDeployments) -> Result<()> {
        let key = paths::initial_deployments_path();
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== ERC20 DEPLOYMENTS ==========

    /// Read ERC20 deployments (configs/erc20_deployments.yaml).
    async fn read_erc20_deployments(&self) -> Result<Erc20Deployments> {
        let key = paths::erc20_deployments_path();
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write ERC20 deployments (configs/erc20_deployments.yaml).
    async fn write_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()> {
        let key = paths::erc20_deployments_path();
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create ERC20 deployments (configs/erc20_deployments.yaml).
    async fn create_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()> {
        let key = paths::erc20_deployments_path();
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== APPS ==========

    /// Read apps config (configs/apps.yaml).
    async fn read_apps(&self) -> Result<Apps> {
        let key = paths::apps_path();
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write apps config (configs/apps.yaml).
    async fn write_apps(&self, data: &Apps) -> Result<()> {
        let key = paths::apps_path();
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create apps config (configs/apps.yaml).
    async fn create_apps(&self, data: &Apps) -> Result<()> {
        let key = paths::apps_path();
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== CHAIN METADATA ==========

    /// Read chain metadata (chains/{chain}/ZkStack.yaml).
    async fn read_chain_metadata(&self, chain: &str) -> Result<ChainMetadata> {
        let key = paths::chain_metadata_path(chain);
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write chain metadata (chains/{chain}/ZkStack.yaml).
    async fn write_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()> {
        let key = paths::chain_metadata_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create chain metadata (chains/{chain}/ZkStack.yaml).
    async fn create_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()> {
        let key = paths::chain_metadata_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== CHAIN WALLETS ==========

    /// Read chain wallets (chains/{chain}/configs/wallets.yaml).
    async fn read_chain_wallets(&self, chain: &str) -> Result<Wallets> {
        let key = paths::chain_wallets_path(chain);
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write chain wallets (chains/{chain}/configs/wallets.yaml).
    async fn write_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()> {
        let key = paths::chain_wallets_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create chain wallets (chains/{chain}/configs/wallets.yaml).
    async fn create_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()> {
        let key = paths::chain_wallets_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== CHAIN CONTRACTS ==========

    /// Read chain contracts (chains/{chain}/configs/contracts.yaml).
    async fn read_chain_contracts(&self, chain: &str) -> Result<ChainContracts> {
        let key = paths::chain_contracts_path(chain);
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write chain contracts (chains/{chain}/configs/contracts.yaml).
    async fn write_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()> {
        let key = paths::chain_contracts_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create chain contracts (chains/{chain}/configs/contracts.yaml).
    async fn create_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()> {
        let key = paths::chain_contracts_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== CHAIN OPERATORS ==========

    /// Read chain operators (chains/{chain}/configs/operators.yaml).
    async fn read_chain_operators(&self, chain: &str) -> Result<Operators> {
        let key = paths::chain_operators_path(chain);
        let content = self.read_raw(&key).await?;
        deserialize_yaml(&content, &key)
    }

    /// Write chain operators (chains/{chain}/configs/operators.yaml).
    async fn write_chain_operators(&self, chain: &str, data: &Operators) -> Result<()> {
        let key = paths::chain_operators_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    /// Create chain operators (chains/{chain}/configs/operators.yaml).
    async fn create_chain_operators(&self, chain: &str, data: &Operators) -> Result<()> {
        let key = paths::chain_operators_path(chain);
        let yaml = serialize_yaml(data, &key)?;
        self.create_raw(&key, &yaml).await
    }
}

/// Create a backend instance based on backend type.
///
/// # Arguments
///
/// * `backend_type` - The type of backend to create.
/// * `base_path` - Base path for the backend (interpretation depends on type).
/// * `logger` - Logger for debug messages.
///
/// # Errors
///
/// Returns `StateError::InvalidConfig` if `FilesystemWithS3Sync` is requested.
/// Use `create_s3_sync_backend` instead for S3-synchronized backends.
pub fn create_backend(
    backend_type: BackendType,
    base_path: &Path,
    logger: Arc<dyn Logger>,
) -> Result<Box<dyn StateBackend>> {
    match backend_type {
        BackendType::Filesystem => Ok(Box::new(FilesystemBackend::new(base_path, logger))),
        BackendType::FilesystemWithS3Sync => Err(StateError::InvalidConfig(
            "Use create_s3_sync_backend() for FilesystemWithS3Sync backend".to_string(),
        )),
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
pub async fn create_s3_sync_backend(
    base_path: &Path,
    ecosystem_name: &str,
    config: crate::s3::S3Config,
    logger: Arc<dyn Logger>,
) -> crate::Result<Box<dyn StateBackend>> {
    let backend = S3SyncBackend::new(base_path, ecosystem_name, config, logger).await?;
    Ok(Box::new(backend))
}

/// Create an S3-synchronized backend with custom event handler.
///
/// Use this to receive progress events for showing spinners or progress bars.
///
/// # Arguments
///
/// * `base_path` - Ecosystem directory path.
/// * `ecosystem_name` - Name for the S3 archive.
/// * `config` - S3 configuration.
/// * `logger` - Logger for debug messages.
/// * `event_handler` - Handler for receiving sync progress events.
///
/// # Errors
///
/// Returns error if S3 client initialization fails.
pub async fn create_s3_sync_backend_with_handler(
    base_path: &Path,
    ecosystem_name: &str,
    config: crate::s3::S3Config,
    logger: Arc<dyn Logger>,
    event_handler: Arc<dyn crate::s3::S3SyncEventHandler>,
) -> crate::Result<Box<dyn StateBackend>> {
    let backend =
        S3SyncBackend::with_event_handler(base_path, ecosystem_name, config, logger, event_handler)
            .await?;
    Ok(Box::new(backend))
}

/// Create S3 sync backend with control handle for batch operations.
///
/// Returns both the backend (as `Arc<dyn StateBackend>`) and a control handle
/// that allows disabling auto-sync and triggering manual sync.
///
/// # Arguments
///
/// * `base_path` - Ecosystem directory path.
/// * `ecosystem_name` - Name for the S3 archive.
/// * `config` - S3 configuration.
/// * `logger` - Logger for debug messages.
/// * `event_handler` - Handler for receiving sync progress events.
///
/// # Errors
///
/// Returns error if S3 client initialization fails.
pub async fn create_s3_sync_backend_with_control(
    base_path: &Path,
    ecosystem_name: &str,
    config: crate::s3::S3Config,
    logger: Arc<dyn Logger>,
    event_handler: Arc<dyn crate::s3::S3SyncEventHandler>,
) -> crate::Result<(Arc<dyn StateBackend>, S3SyncControl)> {
    let backend = Arc::new(
        S3SyncBackend::with_event_handler(base_path, ecosystem_name, config, logger, event_handler)
            .await?,
    );
    let control = S3SyncControl::new(Arc::clone(&backend));
    Ok((backend, control))
}
