//! Filesystem-based state backend implementation.

use crate::backend::StateBackend;
use crate::error::{Result, StateError};
use crate::paths;
use adi_types::{
    Apps, ChainContracts, ChainMetadata, EcosystemContracts, EcosystemMetadata, Erc20Deployments,
    InitialDeployments, Wallets,
};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Filesystem-based state backend.
///
/// Stores state as YAML files in the filesystem.
/// The base path is the ecosystem directory root.
#[derive(Clone, Debug)]
pub struct FilesystemBackend {
    /// Base path for all state operations.
    base_path: PathBuf,
}

impl FilesystemBackend {
    /// Create a new filesystem backend.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Root directory for state storage (ecosystem directory).
    #[must_use]
    pub fn new(base_path: &Path) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
        }
    }

    /// Get the full path for a key.
    fn full_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }

    /// Serialize value to YAML string.
    fn serialize<T: Serialize>(&self, value: &T, key: &str) -> Result<String> {
        serde_yaml::to_string(value).map_err(|e| StateError::YamlSerializeFailed {
            path: self.full_path(key),
            source: e,
        })
    }

    /// Deserialize YAML string to value.
    fn deserialize<T: DeserializeOwned>(&self, content: &str, key: &str) -> Result<T> {
        serde_yaml::from_str(content).map_err(|e| StateError::YamlParseFailed {
            path: self.full_path(key),
            source: e,
        })
    }
}

#[async_trait]
impl StateBackend for FilesystemBackend {
    // ========== RAW OPERATIONS ==========

    async fn read_raw(&self, key: &str) -> Result<String> {
        let path = self.full_path(key);
        log::debug!("Reading state file: {}", path.display());

        if !path.exists() {
            return Err(StateError::NotFound(path));
        }

        fs::read_to_string(&path)
            .await
            .map_err(|e| StateError::ReadFailed { path, source: e })
    }

    async fn write_raw(&self, key: &str, content: &str) -> Result<()> {
        let path = self.full_path(key);
        log::debug!("Writing state file: {}", path.display());

        // Per user requirement: file must exist for write operations
        if !path.exists() {
            return Err(StateError::NotFound(path));
        }

        fs::write(&path, content)
            .await
            .map_err(|e| StateError::WriteFailed { path, source: e })
    }

    async fn create_raw(&self, key: &str, content: &str) -> Result<()> {
        let path = self.full_path(key);
        log::debug!("Creating state file: {}", path.display());

        // Safety check: file must NOT exist for create operations
        if path.exists() {
            return Err(StateError::AlreadyExists(path));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| StateError::WriteFailed {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
            }
        }

        fs::write(&path, content)
            .await
            .map_err(|e| StateError::WriteFailed { path, source: e })
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.full_path(key);
        log::debug!("Checking if state file exists: {}", path.display());
        Ok(path.exists())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let dir_path = self.full_path(prefix);
        log::debug!("Listing state directory: {}", dir_path.display());

        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&dir_path)
            .await
            .map_err(|e| StateError::ReadFailed {
                path: dir_path.clone(),
                source: e,
            })?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| StateError::ReadFailed {
                path: dir_path.clone(),
                source: e,
            })?
        {
            if let Some(name) = entry.file_name().to_str() {
                // Only include directories for chain listing
                if entry.path().is_dir() {
                    entries.push(name.to_string());
                }
            }
        }

        entries.sort();
        Ok(entries)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.full_path(key);
        log::debug!("Deleting state file: {}", path.display());

        if !path.exists() {
            return Err(StateError::NotFound(path));
        }

        fs::remove_file(&path)
            .await
            .map_err(|e| StateError::DeleteFailed { path, source: e })
    }

    // ========== ECOSYSTEM METADATA ==========

    async fn read_ecosystem_metadata(&self) -> Result<EcosystemMetadata> {
        let key = paths::ECOSYSTEM_METADATA;
        log::debug!("Reading ecosystem metadata from {}", key);
        let content = self.read_raw(key).await?;
        self.deserialize(&content, key)
    }

    async fn write_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()> {
        let key = paths::ECOSYSTEM_METADATA;
        log::debug!("Writing ecosystem metadata to {}", key);
        let yaml = self.serialize(data, key)?;
        self.write_raw(key, &yaml).await
    }

    async fn create_ecosystem_metadata(&self, data: &EcosystemMetadata) -> Result<()> {
        let key = paths::ECOSYSTEM_METADATA;
        log::debug!("Creating ecosystem metadata at {}", key);
        let yaml = self.serialize(data, key)?;
        self.create_raw(key, &yaml).await
    }

    // ========== ECOSYSTEM WALLETS ==========

    async fn read_ecosystem_wallets(&self) -> Result<Wallets> {
        let key = paths::ecosystem_wallets_path();
        log::debug!("Reading ecosystem wallets from {}", key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_ecosystem_wallets(&self, data: &Wallets) -> Result<()> {
        let key = paths::ecosystem_wallets_path();
        log::debug!("Writing ecosystem wallets to {}", key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_ecosystem_wallets(&self, data: &Wallets) -> Result<()> {
        let key = paths::ecosystem_wallets_path();
        log::debug!("Creating ecosystem wallets at {}", key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== ECOSYSTEM CONTRACTS ==========

    async fn read_ecosystem_contracts(&self) -> Result<EcosystemContracts> {
        let key = paths::ecosystem_contracts_path();
        log::debug!("Reading ecosystem contracts from {}", key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()> {
        let key = paths::ecosystem_contracts_path();
        log::debug!("Writing ecosystem contracts to {}", key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_ecosystem_contracts(&self, data: &EcosystemContracts) -> Result<()> {
        let key = paths::ecosystem_contracts_path();
        log::debug!("Creating ecosystem contracts at {}", key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== INITIAL DEPLOYMENTS ==========

    async fn read_initial_deployments(&self) -> Result<InitialDeployments> {
        let key = paths::initial_deployments_path();
        log::debug!("Reading initial deployments from {}", key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_initial_deployments(&self, data: &InitialDeployments) -> Result<()> {
        let key = paths::initial_deployments_path();
        log::debug!("Writing initial deployments to {}", key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_initial_deployments(&self, data: &InitialDeployments) -> Result<()> {
        let key = paths::initial_deployments_path();
        log::debug!("Creating initial deployments at {}", key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== ERC20 DEPLOYMENTS ==========

    async fn read_erc20_deployments(&self) -> Result<Erc20Deployments> {
        let key = paths::erc20_deployments_path();
        log::debug!("Reading ERC20 deployments from {}", key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()> {
        let key = paths::erc20_deployments_path();
        log::debug!("Writing ERC20 deployments to {}", key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_erc20_deployments(&self, data: &Erc20Deployments) -> Result<()> {
        let key = paths::erc20_deployments_path();
        log::debug!("Creating ERC20 deployments at {}", key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== APPS ==========

    async fn read_apps(&self) -> Result<Apps> {
        let key = paths::apps_path();
        log::debug!("Reading apps config from {}", key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_apps(&self, data: &Apps) -> Result<()> {
        let key = paths::apps_path();
        log::debug!("Writing apps config to {}", key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_apps(&self, data: &Apps) -> Result<()> {
        let key = paths::apps_path();
        log::debug!("Creating apps config at {}", key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== CHAIN METADATA ==========

    async fn read_chain_metadata(&self, chain: &str) -> Result<ChainMetadata> {
        let key = paths::chain_metadata_path(chain);
        log::debug!("Reading chain '{}' metadata from {}", chain, key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()> {
        let key = paths::chain_metadata_path(chain);
        log::debug!("Writing chain '{}' metadata to {}", chain, key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_chain_metadata(&self, chain: &str, data: &ChainMetadata) -> Result<()> {
        let key = paths::chain_metadata_path(chain);
        log::debug!("Creating chain '{}' metadata at {}", chain, key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== CHAIN WALLETS ==========

    async fn read_chain_wallets(&self, chain: &str) -> Result<Wallets> {
        let key = paths::chain_wallets_path(chain);
        log::debug!("Reading chain '{}' wallets from {}", chain, key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()> {
        let key = paths::chain_wallets_path(chain);
        log::debug!("Writing chain '{}' wallets to {}", chain, key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_chain_wallets(&self, chain: &str, data: &Wallets) -> Result<()> {
        let key = paths::chain_wallets_path(chain);
        log::debug!("Creating chain '{}' wallets at {}", chain, key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }

    // ========== CHAIN CONTRACTS ==========

    async fn read_chain_contracts(&self, chain: &str) -> Result<ChainContracts> {
        let key = paths::chain_contracts_path(chain);
        log::debug!("Reading chain '{}' contracts from {}", chain, key);
        let content = self.read_raw(&key).await?;
        self.deserialize(&content, &key)
    }

    async fn write_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()> {
        let key = paths::chain_contracts_path(chain);
        log::debug!("Writing chain '{}' contracts to {}", chain, key);
        let yaml = self.serialize(data, &key)?;
        self.write_raw(&key, &yaml).await
    }

    async fn create_chain_contracts(&self, chain: &str, data: &ChainContracts) -> Result<()> {
        let key = paths::chain_contracts_path(chain);
        log::debug!("Creating chain '{}' contracts at {}", chain, key);
        let yaml = self.serialize(data, &key)?;
        self.create_raw(&key, &yaml).await
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().expect("Failed to create temp dir");

        // Create ecosystem structure
        std::fs::create_dir_all(dir.path().join("configs")).expect("Failed to create configs dir");
        std::fs::create_dir_all(dir.path().join("chains/test_chain/configs"))
            .expect("Failed to create chain dir");

        // Create ZkStack.yaml
        let mut file =
            File::create(dir.path().join("ZkStack.yaml")).expect("Failed to create file");
        writeln!(file, "name: test_ecosystem").expect("Failed to write");

        // Create wallets.yaml
        let mut file =
            File::create(dir.path().join("configs/wallets.yaml")).expect("Failed to create file");
        writeln!(file, "deployer: null").expect("Failed to write");

        // Create chain ZkStack.yaml
        let mut file = File::create(dir.path().join("chains/test_chain/ZkStack.yaml"))
            .expect("Failed to create file");
        writeln!(file, "name: test_chain").expect("Failed to write");

        dir
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_read_existing_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let content = backend.read_raw("ZkStack.yaml").await.unwrap();
        assert!(content.contains("test_ecosystem"));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_read_nonexistent_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let result = backend.read_raw("nonexistent.yaml").await;
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_write_existing_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        backend
            .write_raw("ZkStack.yaml", "name: updated_ecosystem\n")
            .await
            .unwrap();

        let content = backend.read_raw("ZkStack.yaml").await.unwrap();
        assert!(content.contains("updated_ecosystem"));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_write_nonexistent_file_fails() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let result = backend.write_raw("nonexistent.yaml", "content").await;
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_exists() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        assert!(backend.exists("ZkStack.yaml").await.unwrap());
        assert!(!backend.exists("nonexistent.yaml").await.unwrap());
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_list_chains() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let chains = backend.list("chains").await.unwrap();
        assert_eq!(chains, vec!["test_chain"]);
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_list_nonexistent_dir() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let entries = backend.list("nonexistent").await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_create_new_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        backend
            .create_raw("new_file.yaml", "content: test\n")
            .await
            .unwrap();

        let content = backend.read_raw("new_file.yaml").await.unwrap();
        assert!(content.contains("content: test"));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_create_with_parent_dirs() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        backend
            .create_raw("new_dir/nested/file.yaml", "content: nested\n")
            .await
            .unwrap();

        let content = backend.read_raw("new_dir/nested/file.yaml").await.unwrap();
        assert!(content.contains("content: nested"));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_create_existing_file_fails() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let result = backend.create_raw("ZkStack.yaml", "new content").await;
        assert!(matches!(result, Err(StateError::AlreadyExists(_))));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_delete_existing_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        assert!(backend.exists("ZkStack.yaml").await.unwrap());
        backend.delete("ZkStack.yaml").await.unwrap();
        assert!(!backend.exists("ZkStack.yaml").await.unwrap());
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_delete_nonexistent_file_fails() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let result = backend.delete("nonexistent.yaml").await;
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }
}
