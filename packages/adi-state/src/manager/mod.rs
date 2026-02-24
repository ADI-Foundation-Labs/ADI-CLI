//! High-level state management.

mod chain;
mod ecosystem;

pub use chain::ChainStateOps;
pub use ecosystem::EcosystemStateOps;

#[cfg(feature = "s3")]
use crate::backend::create_s3_sync_backend;
use crate::backend::{create_backend, BackendType, StateBackend};
use crate::error::Result;
use crate::paths;
#[cfg(feature = "s3")]
use crate::s3::S3Config;
use adi_types::{LogCrateLogger, Logger};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// High-level state manager for ecosystem and chain operations.
///
/// Provides typed access to state files with automatic serialization
/// and merge support.
///
/// # Example
///
/// ```rust,ignore
/// use adi_state::StateManager;
/// use std::path::Path;
///
/// # async fn example() -> adi_state::Result<()> {
/// let manager = StateManager::new(Path::new("/home/user/.adi_cli/state/my_ecosystem"));
///
/// // Access ecosystem-level state
/// let metadata = manager.ecosystem().metadata().await?;
/// let wallets = manager.ecosystem().wallets().await?;
///
/// // Access chain-level state
/// let chain_meta = manager.chain("my_chain").metadata().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct StateManager {
    backend: Arc<dyn StateBackend>,
    base_path: PathBuf,
    logger: Arc<dyn Logger>,
}

impl StateManager {
    /// Create a new state manager with filesystem backend.
    ///
    /// # Arguments
    ///
    /// * `ecosystem_path` - Path to the ecosystem directory.
    #[must_use]
    pub fn new(ecosystem_path: &Path) -> Self {
        Self::with_logger(ecosystem_path, Arc::new(LogCrateLogger))
    }

    /// Create a new state manager with filesystem backend and custom logger.
    ///
    /// # Arguments
    ///
    /// * `ecosystem_path` - Path to the ecosystem directory.
    /// * `logger` - Custom logger implementation.
    #[must_use]
    pub fn with_logger(ecosystem_path: &Path, logger: Arc<dyn Logger>) -> Self {
        logger.debug(&format!(
            "Creating StateManager with filesystem backend at {}",
            ecosystem_path.display()
        ));
        let backend = create_backend(BackendType::Filesystem, ecosystem_path, Arc::clone(&logger));
        Self {
            backend: Arc::from(backend),
            base_path: ecosystem_path.to_path_buf(),
            logger,
        }
    }

    /// Create a state manager with a custom backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - Custom backend implementation.
    /// * `base_path` - Base path for reference.
    #[must_use]
    pub fn with_backend(backend: Arc<dyn StateBackend>, base_path: &Path) -> Self {
        Self {
            backend,
            base_path: base_path.to_path_buf(),
            logger: Arc::new(LogCrateLogger),
        }
    }

    /// Create a state manager with the specified backend type.
    ///
    /// # Arguments
    ///
    /// * `backend_type` - The backend type to use.
    /// * `ecosystem_path` - Path to the ecosystem directory.
    #[must_use]
    pub fn with_backend_type(backend_type: BackendType, ecosystem_path: &Path) -> Self {
        Self::with_backend_type_and_logger(backend_type, ecosystem_path, Arc::new(LogCrateLogger))
    }

    /// Create a state manager with the specified backend type and custom logger.
    ///
    /// # Arguments
    ///
    /// * `backend_type` - The backend type to use.
    /// * `ecosystem_path` - Path to the ecosystem directory.
    /// * `logger` - Custom logger implementation.
    #[must_use]
    pub fn with_backend_type_and_logger(
        backend_type: BackendType,
        ecosystem_path: &Path,
        logger: Arc<dyn Logger>,
    ) -> Self {
        logger.debug(&format!(
            "Creating StateManager with {:?} backend at {}",
            backend_type,
            ecosystem_path.display()
        ));
        let backend = create_backend(backend_type, ecosystem_path, Arc::clone(&logger));
        Self {
            backend: Arc::from(backend),
            base_path: ecosystem_path.to_path_buf(),
            logger,
        }
    }

    /// Create a state manager with S3 synchronization.
    ///
    /// Creates an S3SyncBackend that automatically syncs state to S3
    /// after every write operation.
    ///
    /// # Arguments
    ///
    /// * `ecosystem_path` - Path to the ecosystem directory.
    /// * `ecosystem_name` - Name of the ecosystem (used for S3 archive key).
    /// * `s3_config` - S3 configuration for the sync backend.
    /// * `logger` - Custom logger implementation.
    ///
    /// # Errors
    ///
    /// Returns error if S3 client initialization fails.
    #[cfg(feature = "s3")]
    pub async fn with_s3_sync(
        ecosystem_path: &Path,
        ecosystem_name: &str,
        s3_config: S3Config,
        logger: Arc<dyn Logger>,
    ) -> Result<Self> {
        logger.debug(&format!(
            "Creating StateManager with S3 sync backend at {}",
            ecosystem_path.display()
        ));

        let backend = create_s3_sync_backend(
            ecosystem_path,
            ecosystem_name,
            s3_config,
            Arc::clone(&logger),
        )
        .await?;

        Ok(Self {
            backend: Arc::from(backend),
            base_path: ecosystem_path.to_path_buf(),
            logger,
        })
    }

    /// Get a reference to the logger.
    #[must_use]
    pub fn logger(&self) -> &Arc<dyn Logger> {
        &self.logger
    }

    /// Get ecosystem-level state operations.
    #[must_use]
    pub fn ecosystem(&self) -> EcosystemStateOps {
        EcosystemStateOps::new(Arc::clone(&self.backend), Arc::clone(&self.logger))
    }

    /// Get chain-level state operations.
    ///
    /// # Arguments
    ///
    /// * `chain_name` - Name of the chain.
    #[must_use]
    pub fn chain(&self, chain_name: &str) -> ChainStateOps {
        self.logger.debug(&format!(
            "Getting chain state operations for '{}'",
            chain_name
        ));
        ChainStateOps::new(
            Arc::clone(&self.backend),
            chain_name.to_string(),
            Arc::clone(&self.logger),
        )
    }

    /// Get the base path of this state manager.
    #[must_use]
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// List all chains in the ecosystem.
    ///
    /// Returns chain names for directories that contain a ZkStack.yaml file.
    ///
    /// # Errors
    ///
    /// Returns error if reading the chains directory fails.
    pub async fn list_chains(&self) -> Result<Vec<String>> {
        self.logger.debug("Listing chains in ecosystem");
        let entries = self.backend.list(paths::CHAINS_DIR).await?;

        let mut chains = Vec::new();
        for name in entries {
            let metadata_key = paths::chain_metadata_path(&name);
            if self.backend.exists(&metadata_key).await? {
                chains.push(name);
            }
        }

        self.logger
            .debug(&format!("Found {} chains: {:?}", chains.len(), chains));
        Ok(chains)
    }

    /// Check if ecosystem state exists.
    ///
    /// Returns true if the ecosystem metadata file (ZkStack.yaml) exists.
    pub async fn exists(&self) -> Result<bool> {
        self.backend.exists(paths::ECOSYSTEM_METADATA).await
    }

    /// Delete all ecosystem state.
    ///
    /// Removes the entire ecosystem directory and all its contents.
    ///
    /// # Errors
    ///
    /// Returns error if the directory doesn't exist or deletion fails.
    pub async fn delete_all(&self) -> Result<()> {
        self.logger.info(&format!(
            "Deleting ecosystem state at {}",
            self.base_path.display()
        ));
        tokio::fs::remove_dir_all(&self.base_path)
            .await
            .map_err(|e| crate::error::StateError::DeleteFailed {
                path: self.base_path.clone(),
                source: e,
            })
    }

    /// List all files in the ecosystem state directory for display.
    ///
    /// Returns a list of relative file paths that exist in the state directory.
    /// Useful for showing the user what will be deleted.
    pub fn list_state_files(&self) -> Vec<String> {
        let mut files = Vec::new();
        if self.base_path.exists() {
            collect_files_recursive(&self.base_path, &self.base_path, &mut files);
        }
        files.sort();
        files
    }
}

/// Recursively collect file paths relative to base.
fn collect_files_recursive(base: &Path, current: &Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(base, &path, files);
            } else if let Ok(relative) = path.strip_prefix(base) {
                files.push(relative.display().to_string());
            }
        }
    }
}
