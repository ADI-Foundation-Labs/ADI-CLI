//! Filesystem-based state backend implementation.

use crate::backend::StateBackend;
use crate::error::{Result, StateError};
use adi_types::Logger;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Filesystem-based state backend.
///
/// Stores state as YAML files in the filesystem.
/// The base path is the ecosystem directory root.
#[derive(Clone)]
pub struct FilesystemBackend {
    /// Base path for all state operations.
    base_path: PathBuf,
    /// Logger for debug messages.
    logger: Arc<dyn Logger>,
}

impl FilesystemBackend {
    /// Create a new filesystem backend.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Root directory for state storage (ecosystem directory).
    /// * `logger` - Logger for debug messages.
    #[must_use]
    pub fn new(base_path: &Path, logger: Arc<dyn Logger>) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
            logger,
        }
    }

    /// Get the full path for a key.
    fn full_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

#[async_trait]
impl StateBackend for FilesystemBackend {
    async fn read_raw(&self, key: &str) -> Result<String> {
        let path = self.full_path(key);
        self.logger
            .debug(&format!("Reading state file: {}", path.display()));

        match fs::read_to_string(&path).await {
            Ok(content) => Ok(content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(StateError::NotFound(path)),
            Err(e) => Err(StateError::ReadFailed { path, source: e }),
        }
    }

    async fn write_raw(&self, key: &str, content: &str) -> Result<()> {
        let path = self.full_path(key);
        self.logger
            .debug(&format!("Writing state file: {}", path.display()));

        // Open without create — fails with NotFound if file doesn't exist
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    StateError::NotFound(path.clone())
                } else {
                    StateError::WriteFailed {
                        path: path.clone(),
                        source: e,
                    }
                }
            })?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| StateError::WriteFailed { path, source: e })
    }

    async fn create_raw(&self, key: &str, content: &str) -> Result<()> {
        let path = self.full_path(key);
        self.logger
            .debug(&format!("Creating state file: {}", path.display()));

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StateError::WriteFailed {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
        }

        // create_new — fails with AlreadyExists if file exists
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    StateError::AlreadyExists(path.clone())
                } else {
                    StateError::WriteFailed {
                        path: path.clone(),
                        source: e,
                    }
                }
            })?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| StateError::WriteFailed { path, source: e })
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.full_path(key);
        self.logger.debug(&format!(
            "Checking if state file exists: {}",
            path.display()
        ));
        fs::try_exists(&path)
            .await
            .map_err(|e| StateError::ReadFailed { path, source: e })
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let dir_path = self.full_path(prefix);
        self.logger
            .debug(&format!("Listing state directory: {}", dir_path.display()));

        // Check existence and type asynchronously
        match fs::metadata(&dir_path).await {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Ok(Vec::new()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(StateError::ReadFailed {
                    path: dir_path,
                    source: e,
                })
            }
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
        self.logger
            .debug(&format!("Deleting state file: {}", path.display()));

        fs::remove_file(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StateError::NotFound(path)
            } else {
                StateError::DeleteFailed { path, source: e }
            }
        })
    }

    async fn delete_dir(&self, key: &str) -> Result<()> {
        let path = self.full_path(key);
        self.logger
            .debug(&format!("Deleting state directory: {}", path.display()));

        fs::remove_dir_all(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StateError::NotFound(path)
            } else {
                StateError::DeleteFailed { path, source: e }
            }
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use adi_types::NoopLogger;
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
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        let content = backend.read_raw("ZkStack.yaml").await.unwrap();
        assert!(content.contains("test_ecosystem"));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_read_nonexistent_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        let result = backend.read_raw("nonexistent.yaml").await;
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_write_existing_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

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
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        let result = backend.write_raw("nonexistent.yaml", "content").await;
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_exists() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        assert!(backend.exists("ZkStack.yaml").await.unwrap());
        assert!(!backend.exists("nonexistent.yaml").await.unwrap());
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_list_chains() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        let chains = backend.list("chains").await.unwrap();
        assert_eq!(chains, vec!["test_chain"]);
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_list_nonexistent_dir() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        let entries = backend.list("nonexistent").await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_create_new_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

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
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

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
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        let result = backend.create_raw("ZkStack.yaml", "new content").await;
        assert!(matches!(result, Err(StateError::AlreadyExists(_))));
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_delete_existing_file() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        assert!(backend.exists("ZkStack.yaml").await.unwrap());
        backend.delete("ZkStack.yaml").await.unwrap();
        assert!(!backend.exists("ZkStack.yaml").await.unwrap());
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_delete_nonexistent_file_fails() {
        let dir = setup_test_dir();
        let backend = FilesystemBackend::new(dir.path(), Arc::new(NoopLogger));

        let result = backend.delete("nonexistent.yaml").await;
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }
}
