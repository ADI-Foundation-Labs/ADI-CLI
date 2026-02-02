//! High-level state management.

mod chain;
mod ecosystem;

pub use chain::ChainStateOps;
pub use ecosystem::EcosystemStateOps;

use crate::backend::{create_backend, BackendType, StateBackend};
use crate::error::Result;
use crate::paths;
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
}

impl StateManager {
    /// Create a new state manager with filesystem backend.
    ///
    /// # Arguments
    ///
    /// * `ecosystem_path` - Path to the ecosystem directory.
    #[must_use]
    pub fn new(ecosystem_path: &Path) -> Self {
        let backend = create_backend(BackendType::Filesystem, ecosystem_path);
        Self {
            backend: Arc::from(backend),
            base_path: ecosystem_path.to_path_buf(),
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
        }
    }

    /// Get ecosystem-level state operations.
    #[must_use]
    pub fn ecosystem(&self) -> EcosystemStateOps {
        EcosystemStateOps::new(Arc::clone(&self.backend))
    }

    /// Get chain-level state operations.
    ///
    /// # Arguments
    ///
    /// * `chain_name` - Name of the chain.
    #[must_use]
    pub fn chain(&self, chain_name: &str) -> ChainStateOps {
        ChainStateOps::new(Arc::clone(&self.backend), chain_name.to_string())
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
        let entries = self.backend.list(paths::CHAINS_DIR).await?;

        let mut chains = Vec::new();
        for name in entries {
            let metadata_key = paths::chain_metadata_path(&name);
            if self.backend.exists(&metadata_key).await? {
                chains.push(name);
            }
        }

        Ok(chains)
    }
}

/// Helper to deserialize YAML content with proper error context.
pub(crate) fn deserialize_yaml<T: serde::de::DeserializeOwned>(
    content: &str,
    path: &Path,
) -> Result<T> {
    serde_yaml::from_str(content).map_err(|e| crate::error::StateError::YamlParseFailed {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Helper to serialize value to YAML with proper error context.
pub(crate) fn serialize_yaml<T: serde::Serialize>(value: &T, path: &Path) -> Result<String> {
    serde_yaml::to_string(value).map_err(|e| crate::error::StateError::YamlSerializeFailed {
        path: path.to_path_buf(),
        source: e,
    })
}
