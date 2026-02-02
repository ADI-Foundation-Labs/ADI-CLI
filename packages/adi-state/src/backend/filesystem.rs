//! Filesystem-based state backend implementation.

use crate::backend::StateBackend;
use crate::error::{Result, StateError};
use async_trait::async_trait;
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
}

#[async_trait]
impl StateBackend for FilesystemBackend {
    async fn read(&self, key: &str) -> Result<String> {
        let path = self.full_path(key);
        log::debug!("Reading state file: {}", path.display());

        if !path.exists() {
            return Err(StateError::NotFound(path));
        }

        fs::read_to_string(&path)
            .await
            .map_err(|e| StateError::ReadFailed { path, source: e })
    }

    async fn write(&self, key: &str, content: &str) -> Result<()> {
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

        let content = backend.read("ZkStack.yaml").await.unwrap();
        assert!(content.contains("test_ecosystem"));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_read_nonexistent_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let result = backend.read("nonexistent.yaml").await;
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_write_existing_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        backend
            .write("ZkStack.yaml", "name: updated_ecosystem\n")
            .await
            .unwrap();

        let content = backend.read("ZkStack.yaml").await.unwrap();
        assert!(content.contains("updated_ecosystem"));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_write_nonexistent_file_fails() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path());

        let result = backend.write("nonexistent.yaml", "content").await;
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
}
