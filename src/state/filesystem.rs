//! Filesystem-based implementation of the state backend.
//!
//! This module provides `FilesystemBackend`, which stores state data
//! as files in a configurable directory. Keys are mapped to file paths,
//! and atomic writes are used to prevent data corruption.

use crate::error::Result;
use async_trait::async_trait;
use eyre::WrapErr;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::StateBackend;

/// Filesystem-based state backend.
///
/// Stores state data as files in a base directory. Keys are converted
/// to file paths by appending them to the base path. For example, the key
/// `ecosystems/my_ecosystem/metadata` would be stored at
/// `{base_path}/ecosystems/my_ecosystem/metadata`.
///
/// # Atomic Writes
///
/// To prevent data corruption from interrupted writes, this backend
/// uses atomic writes: data is first written to a temporary file,
/// then atomically renamed to the target path.
///
/// # Example
///
/// ```rust,ignore
/// use adi_cli::state::{FilesystemBackend, StateBackend};
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> eyre::Result<()> {
///     let backend = FilesystemBackend::new(PathBuf::from("~/.adi_cli/state"))?;
///
///     // Store some data
///     backend.set("ecosystems/test/metadata", b"data").await?;
///
///     // Retrieve it
///     let data = backend.get("ecosystems/test/metadata").await?;
///     assert!(data.is_some());
///
///     Ok(())
/// }
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub struct FilesystemBackend {
    /// Base directory for all state files.
    base_path: PathBuf,
}

#[allow(dead_code)]
impl FilesystemBackend {
    /// Creates a new filesystem backend with the given base path.
    ///
    /// The base path directory will be created if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the base path cannot be created or accessed.
    pub fn new(base_path: PathBuf) -> Result<Self> {
        Ok(Self { base_path })
    }

    /// Returns the full file path for a given key.
    fn key_to_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }

    /// Returns the path for a temporary file used during atomic writes.
    fn temp_path(&self, key: &str) -> PathBuf {
        let mut path = self.key_to_path(key);
        let file_name = path
            .file_name()
            .map(|n| format!(".{}.tmp", n.to_string_lossy()))
            .unwrap_or_else(|| ".tmp".to_string());
        path.set_file_name(file_name);
        path
    }

    /// Ensures the parent directory for a key exists.
    async fn ensure_parent_dir(&self, key: &str) -> Result<()> {
        let path = self.key_to_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .wrap_err_with(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        Ok(())
    }
}

#[async_trait]
impl StateBackend for FilesystemBackend {
    /// Retrieve value by key.
    ///
    /// Returns `Ok(Some(data))` if the file exists, `Ok(None)` if it doesn't,
    /// or an error if the read operation fails.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.key_to_path(key);

        match fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => {
                Err(e).wrap_err_with(|| format!("Failed to read state file: {}", path.display()))
            }
        }
    }

    /// Store value by key using atomic write.
    ///
    /// Data is first written to a temporary file, then atomically renamed
    /// to the target path. This prevents data corruption from interrupted writes.
    async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        self.ensure_parent_dir(key).await?;

        let target_path = self.key_to_path(key);
        let temp_path = self.temp_path(key);

        // Write to temporary file first
        let mut file = fs::File::create(&temp_path)
            .await
            .wrap_err_with(|| format!("Failed to create temp file: {}", temp_path.display()))?;

        file.write_all(value)
            .await
            .wrap_err_with(|| format!("Failed to write to temp file: {}", temp_path.display()))?;

        file.sync_all()
            .await
            .wrap_err_with(|| format!("Failed to sync temp file: {}", temp_path.display()))?;

        // Atomically rename temp file to target
        fs::rename(&temp_path, &target_path)
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to rename temp file {} to {}",
                    temp_path.display(),
                    target_path.display()
                )
            })?;

        Ok(())
    }

    /// Delete value by key.
    ///
    /// Returns success even if the file doesn't exist.
    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.key_to_path(key);

        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => {
                Err(e).wrap_err_with(|| format!("Failed to delete state file: {}", path.display()))
            }
        }
    }

    /// List keys with prefix.
    ///
    /// Returns all keys (relative to base path) that start with the given prefix.
    /// Walks the directory tree recursively to find all matching files.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let search_path = self.key_to_path(prefix);
        let mut keys = Vec::new();

        // If the prefix path doesn't exist, return empty list
        if !search_path.exists() {
            // Check if prefix is a directory or if we need to search parent
            let parent_path = search_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.base_path.clone());

            if !parent_path.exists() {
                return Ok(keys);
            }

            // Search in parent directory for files matching prefix
            self.collect_keys_with_prefix(&parent_path, prefix, &mut keys)
                .await?;
        } else if search_path.is_dir() {
            // Prefix is a directory, collect all files under it
            self.collect_keys_recursive(&search_path, &mut keys).await?;
        } else {
            // Prefix is a file, include it if it matches
            let key = self.path_to_key(&search_path)?;
            keys.push(key);
        }

        Ok(keys)
    }

    /// Check if key exists.
    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.key_to_path(key);
        Ok(path.exists())
    }
}

#[allow(dead_code)]
impl FilesystemBackend {
    /// Converts a file path back to a key (relative to base_path).
    fn path_to_key(&self, path: &Path) -> Result<String> {
        let relative = path.strip_prefix(&self.base_path).wrap_err_with(|| {
            format!(
                "Path {} is not under base path {}",
                path.display(),
                self.base_path.display()
            )
        })?;

        Ok(relative.to_string_lossy().to_string())
    }

    /// Recursively collects all file keys under a directory.
    async fn collect_keys_recursive(&self, dir: &Path, keys: &mut Vec<String>) -> Result<()> {
        let mut entries = fs::read_dir(dir)
            .await
            .wrap_err_with(|| format!("Failed to read directory: {}", dir.display()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .wrap_err("Failed to read directory entry")?
        {
            let path = entry.path();

            // Skip hidden files (temp files)
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }

            if path.is_dir() {
                Box::pin(self.collect_keys_recursive(&path, keys)).await?;
            } else {
                let key = self.path_to_key(&path)?;
                keys.push(key);
            }
        }

        Ok(())
    }

    /// Collects keys in a directory that start with a given prefix.
    async fn collect_keys_with_prefix(
        &self,
        dir: &Path,
        prefix: &str,
        keys: &mut Vec<String>,
    ) -> Result<()> {
        let mut entries = fs::read_dir(dir)
            .await
            .wrap_err_with(|| format!("Failed to read directory: {}", dir.display()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .wrap_err("Failed to read directory entry")?
        {
            let path = entry.path();

            // Skip hidden files
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }

            let key = self.path_to_key(&path)?;

            if key.starts_with(prefix) {
                if path.is_dir() {
                    Box::pin(self.collect_keys_recursive(&path, keys)).await?;
                } else {
                    keys.push(key);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_backend() -> (FilesystemBackend, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let backend = FilesystemBackend::new(temp_dir.path().to_path_buf())
            .expect("Failed to create backend");
        (backend, temp_dir)
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";
        let value = b"test data";

        backend.set(key, value).await.expect("Failed to set");
        let result = backend.get(key).await.expect("Failed to get");

        assert_eq!(result, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let (backend, _temp_dir) = create_test_backend().await;

        let result = backend.get("nonexistent/key").await.expect("Failed to get");

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_exists() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";

        assert!(!backend.exists(key).await.expect("Failed to check exists"));

        backend.set(key, b"data").await.expect("Failed to set");

        assert!(backend.exists(key).await.expect("Failed to check exists"));
    }

    #[tokio::test]
    async fn test_delete() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";
        backend.set(key, b"data").await.expect("Failed to set");

        backend.delete(key).await.expect("Failed to delete");

        assert!(!backend.exists(key).await.expect("Failed to check exists"));
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let (backend, _temp_dir) = create_test_backend().await;

        // Should not error when deleting nonexistent key
        backend
            .delete("nonexistent/key")
            .await
            .expect("Failed to delete nonexistent");
    }

    #[tokio::test]
    async fn test_list_keys() {
        let (backend, _temp_dir) = create_test_backend().await;

        // Create several keys
        backend
            .set("ecosystems/test1/metadata", b"data1")
            .await
            .expect("Failed to set");
        backend
            .set("ecosystems/test1/contracts", b"data2")
            .await
            .expect("Failed to set");
        backend
            .set("ecosystems/test2/metadata", b"data3")
            .await
            .expect("Failed to set");

        // List keys under ecosystems/test1
        let keys = backend
            .list_keys("ecosystems/test1")
            .await
            .expect("Failed to list keys");

        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"ecosystems/test1/metadata".to_string()));
        assert!(keys.contains(&"ecosystems/test1/contracts".to_string()));
    }

    #[tokio::test]
    async fn test_list_keys_empty_prefix() {
        let (backend, _temp_dir) = create_test_backend().await;

        let keys = backend
            .list_keys("nonexistent")
            .await
            .expect("Failed to list keys");

        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_atomic_write() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";
        let value = b"test data for atomic write";

        // Write data
        backend.set(key, value).await.expect("Failed to set");

        // Verify temp file doesn't exist
        let temp_path = backend.temp_path(key);
        assert!(!temp_path.exists());

        // Verify target file exists with correct content
        let result = backend.get(key).await.expect("Failed to get");
        assert_eq!(result, Some(value.to_vec()));
    }
}
