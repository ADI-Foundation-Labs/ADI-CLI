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

use super::{StateBackend, ValidationResult};

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

    /// Validate state integrity.
    ///
    /// Checks for orphaned temporary files and unreadable state files.
    async fn validate(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::ok();

        // Check if base directory exists
        if !self.base_path.exists() {
            // No state directory yet is valid (fresh install)
            return Ok(result);
        }

        // Collect orphaned temp files and validate readable files
        self.validate_directory(&self.base_path, &mut result)
            .await?;

        // State is valid if there are no unreadable files
        // Orphaned temp files are warnings, not critical errors
        result.is_valid = result.unreadable_files.is_empty();

        Ok(result)
    }

    /// Clean up orphaned temporary files.
    ///
    /// Removes any `.*.tmp` files left over from interrupted atomic writes.
    async fn cleanup_temp_files(&self) -> Result<usize> {
        if !self.base_path.exists() {
            return Ok(0);
        }

        let mut count = 0;
        self.cleanup_temp_files_recursive(&self.base_path, &mut count)
            .await?;
        Ok(count)
    }

    /// Validate and ensure the state directory is ready for use.
    ///
    /// Creates the directory if it doesn't exist and verifies write access.
    async fn ensure_ready(&self) -> Result<()> {
        // Create directory if it doesn't exist
        if !self.base_path.exists() {
            fs::create_dir_all(&self.base_path)
                .await
                .wrap_err_with(|| {
                    format!(
                        "Failed to create state directory '{}'. \
                     Please ensure the parent directory exists and is writable.",
                        self.base_path.display()
                    )
                })?;
            log::debug!("Created state directory: {}", self.base_path.display());
        }

        // Verify it's actually a directory
        if !self.base_path.is_dir() {
            return Err(eyre::eyre!(
                "State path '{}' exists but is not a directory. \
                 Please remove the file or choose a different state directory.",
                self.base_path.display()
            ));
        }

        // Verify write access by creating and removing a test file
        let test_file = self.base_path.join(".write_test");
        match fs::write(&test_file, b"test").await {
            Ok(()) => {
                // Clean up test file
                let _ = fs::remove_file(&test_file).await;
            }
            Err(e) => {
                let kind = e.kind();
                return Err(e).wrap_err_with(|| {
                    format!(
                        "State directory '{}' is not writable. \
                         Please check permissions (required: read/write). \
                         Error: {kind}",
                        self.base_path.display()
                    )
                });
            }
        }

        log::debug!("State directory ready: {}", self.base_path.display());
        Ok(())
    }

    /// Get the base path of this state backend.
    fn base_path(&self) -> &std::path::Path {
        &self.base_path
    }

    /// Create a backup of a key before a destructive operation.
    async fn backup(&self, key: &str) -> Result<Option<String>> {
        // Check if key exists
        let data = match self.get(key).await? {
            Some(data) => data,
            None => return Ok(None),
        };

        // Generate backup key with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let backup_key = format!(".backups/{key}/{timestamp}");

        // Store backup
        self.set(&backup_key, &data).await?;

        log::debug!("Created backup for '{}' at '{}'", key, backup_key);
        Ok(Some(backup_key))
    }

    /// Restore a key from a backup.
    async fn restore(&self, key: &str, backup_key: Option<&str>) -> Result<()> {
        let restore_from = match backup_key {
            Some(bk) => bk.to_string(),
            None => {
                // Get most recent backup
                let backups = self.list_backups(key).await?;
                backups
                    .into_iter()
                    .next()
                    .ok_or_else(|| eyre::eyre!("No backups found for key '{}'", key))?
            }
        };

        let backup_data = self
            .get(&restore_from)
            .await?
            .ok_or_else(|| eyre::eyre!("Backup '{}' not found", restore_from))?;

        self.set(key, &backup_data).await?;

        log::debug!("Restored '{}' from backup '{}'", key, restore_from);
        Ok(())
    }

    /// List available backups for a key (newest first).
    async fn list_backups(&self, key: &str) -> Result<Vec<String>> {
        let backup_prefix = format!(".backups/{key}");
        let mut backups = self.list_keys(&backup_prefix).await?;

        // Sort by timestamp descending (newest first)
        // Backup keys are formatted as .backups/{key}/{timestamp}
        // where timestamp is YYYYMMDD_HHMMSS_mmm
        backups.sort_by(|a, b| b.cmp(a));

        Ok(backups)
    }

    /// Delete a key with automatic backup.
    async fn delete_with_backup(&self, key: &str) -> Result<Option<String>> {
        let backup_key = self.backup(key).await?;
        self.delete(key).await?;
        Ok(backup_key)
    }

    /// Set a key with automatic backup of existing value.
    async fn set_with_backup(&self, key: &str, value: &[u8]) -> Result<Option<String>> {
        let backup_key = self.backup(key).await?;
        self.set(key, value).await?;
        Ok(backup_key)
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

    /// Recursively validates a directory and its contents.
    ///
    /// Checks for:
    /// - Orphaned `.*.tmp` files from interrupted atomic writes
    /// - Files that cannot be read (potential corruption)
    async fn validate_directory(&self, dir: &Path, result: &mut ValidationResult) -> Result<()> {
        let mut entries = match fs::read_dir(dir).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                return Err(e)
                    .wrap_err_with(|| format!("Failed to read directory: {}", dir.display()))
            }
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .wrap_err("Failed to read directory entry")?
        {
            let path = entry.path();

            if path.is_dir() {
                Box::pin(self.validate_directory(&path, result)).await?;
            } else if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();

                // Check for orphaned temp files
                if name_str.starts_with('.') && name_str.ends_with(".tmp") {
                    result.orphaned_temp_files.push(path.display().to_string());
                    continue;
                }

                // Skip other hidden files
                if name_str.starts_with('.') {
                    continue;
                }

                // Try to read the file to verify it's not corrupted
                if let Err(e) = fs::read(&path).await {
                    result
                        .unreadable_files
                        .push((path.display().to_string(), e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Recursively cleans up orphaned temporary files.
    async fn cleanup_temp_files_recursive(&self, dir: &Path, count: &mut usize) -> Result<()> {
        let mut entries = match fs::read_dir(dir).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                return Err(e)
                    .wrap_err_with(|| format!("Failed to read directory: {}", dir.display()))
            }
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .wrap_err("Failed to read directory entry")?
        {
            let path = entry.path();

            if path.is_dir() {
                Box::pin(self.cleanup_temp_files_recursive(&path, count)).await?;
            } else if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();

                // Remove orphaned temp files
                if name_str.starts_with('.') && name_str.ends_with(".tmp") {
                    if let Err(e) = fs::remove_file(&path).await {
                        // Log but don't fail on cleanup errors
                        log::warn!(
                            "Failed to remove orphaned temp file {}: {}",
                            path.display(),
                            e
                        );
                    } else {
                        *count += 1;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
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

    #[tokio::test]
    async fn test_validate_empty_directory() {
        let (backend, _temp_dir) = create_test_backend().await;

        let result = backend.validate().await.expect("Failed to validate");

        assert!(result.is_valid);
        assert!(result.orphaned_temp_files.is_empty());
        assert!(result.unreadable_files.is_empty());
    }

    #[tokio::test]
    async fn test_validate_with_valid_files() {
        let (backend, _temp_dir) = create_test_backend().await;

        // Create some valid files
        backend
            .set("ecosystems/test/metadata", b"metadata")
            .await
            .expect("Failed to set");
        backend
            .set("ecosystems/test/contracts", b"contracts")
            .await
            .expect("Failed to set");

        let result = backend.validate().await.expect("Failed to validate");

        assert!(result.is_valid);
        assert!(result.orphaned_temp_files.is_empty());
        assert!(result.unreadable_files.is_empty());
    }

    #[tokio::test]
    async fn test_validate_detects_orphaned_temp_files() {
        let (backend, temp_dir) = create_test_backend().await;

        // Create a valid file first (to create directory structure)
        backend
            .set("ecosystems/test/metadata", b"metadata")
            .await
            .expect("Failed to set");

        // Manually create an orphaned temp file
        let orphan_path = temp_dir.path().join("ecosystems/test/.metadata.tmp");
        std::fs::write(&orphan_path, b"orphan data").expect("Failed to create orphan");

        let result = backend.validate().await.expect("Failed to validate");

        // Orphaned temp files are warnings, not errors
        assert!(result.is_valid);
        assert_eq!(result.orphaned_temp_files.len(), 1);
        assert!(result
            .orphaned_temp_files
            .first()
            .unwrap()
            .contains(".metadata.tmp"));
    }

    #[tokio::test]
    async fn test_cleanup_temp_files() {
        let (backend, temp_dir) = create_test_backend().await;

        // Create a valid file first
        backend
            .set("ecosystems/test/metadata", b"metadata")
            .await
            .expect("Failed to set");

        // Manually create orphaned temp files
        let orphan1 = temp_dir.path().join("ecosystems/test/.metadata.tmp");
        let orphan2 = temp_dir.path().join("ecosystems/test/.contracts.tmp");
        std::fs::write(&orphan1, b"orphan1").expect("Failed to create orphan1");
        std::fs::write(&orphan2, b"orphan2").expect("Failed to create orphan2");

        // Verify orphans exist
        assert!(orphan1.exists());
        assert!(orphan2.exists());

        // Clean up
        let count = backend
            .cleanup_temp_files()
            .await
            .expect("Failed to cleanup");

        assert_eq!(count, 2);
        assert!(!orphan1.exists());
        assert!(!orphan2.exists());

        // Original file should still exist
        assert!(backend
            .exists("ecosystems/test/metadata")
            .await
            .expect("Failed to check"));
    }

    #[tokio::test]
    async fn test_validate_nonexistent_base_path() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let nonexistent_path = temp_dir.path().join("does_not_exist");
        let backend = FilesystemBackend::new(nonexistent_path).expect("Failed to create backend");

        let result = backend.validate().await.expect("Failed to validate");

        // Nonexistent base path is valid (fresh install)
        assert!(result.is_valid);
        assert!(result.orphaned_temp_files.is_empty());
        assert!(result.unreadable_files.is_empty());
    }

    #[tokio::test]
    async fn test_ensure_ready_creates_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let new_path = temp_dir.path().join("new_state_dir");
        let backend = FilesystemBackend::new(new_path.clone()).expect("Failed to create backend");

        assert!(!new_path.exists());

        backend
            .ensure_ready()
            .await
            .expect("Failed to ensure ready");

        assert!(new_path.exists());
        assert!(new_path.is_dir());
    }

    #[tokio::test]
    async fn test_ensure_ready_existing_directory() {
        let (backend, _temp_dir) = create_test_backend().await;

        // Should succeed for existing directory
        backend
            .ensure_ready()
            .await
            .expect("Failed to ensure ready");
    }

    #[tokio::test]
    async fn test_ensure_ready_nested_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let nested_path = temp_dir.path().join("a/b/c/state");
        let backend =
            FilesystemBackend::new(nested_path.clone()).expect("Failed to create backend");

        backend
            .ensure_ready()
            .await
            .expect("Failed to ensure ready");

        assert!(nested_path.exists());
        assert!(nested_path.is_dir());
    }

    #[tokio::test]
    async fn test_ensure_ready_fails_if_file_exists() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("not_a_dir");

        // Create a file at the path
        std::fs::write(&file_path, b"I'm a file").expect("Failed to create file");

        let backend = FilesystemBackend::new(file_path).expect("Failed to create backend");

        let result = backend.ensure_ready().await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not a directory"));
    }

    #[tokio::test]
    async fn test_base_path() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let expected_path = temp_dir.path().to_path_buf();
        let backend =
            FilesystemBackend::new(expected_path.clone()).expect("Failed to create backend");

        assert_eq!(backend.base_path(), expected_path);
    }

    #[tokio::test]
    async fn test_backup_creates_backup() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";
        let value = b"original data";

        // Create original
        backend.set(key, value).await.expect("Failed to set");

        // Create backup
        let backup_key = backend.backup(key).await.expect("Failed to backup");
        assert!(backup_key.is_some());

        let backup_key = backup_key.unwrap();
        assert!(backup_key.starts_with(".backups/ecosystems/test/metadata/"));

        // Verify backup content
        let backup_data = backend
            .get(&backup_key)
            .await
            .expect("Failed to get backup");
        assert_eq!(backup_data, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_backup_nonexistent_returns_none() {
        let (backend, _temp_dir) = create_test_backend().await;

        let backup_key = backend
            .backup("nonexistent")
            .await
            .expect("Failed to backup");
        assert!(backup_key.is_none());
    }

    #[tokio::test]
    async fn test_restore_from_backup() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";
        let original_value = b"original data";
        let new_value = b"new data";

        // Create original and backup
        backend
            .set(key, original_value)
            .await
            .expect("Failed to set");
        let backup_key = backend
            .backup(key)
            .await
            .expect("Failed to backup")
            .unwrap();

        // Overwrite with new value
        backend.set(key, new_value).await.expect("Failed to set");
        let current = backend.get(key).await.expect("Failed to get");
        assert_eq!(current, Some(new_value.to_vec()));

        // Restore from backup
        backend
            .restore(key, Some(&backup_key))
            .await
            .expect("Failed to restore");

        // Verify restored value
        let restored = backend.get(key).await.expect("Failed to get");
        assert_eq!(restored, Some(original_value.to_vec()));
    }

    #[tokio::test]
    async fn test_restore_most_recent_backup() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";

        // Create multiple backups
        backend.set(key, b"version 1").await.expect("Failed to set");
        backend.backup(key).await.expect("Failed to backup");

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        backend.set(key, b"version 2").await.expect("Failed to set");
        backend.backup(key).await.expect("Failed to backup");

        // Set to different value
        backend.set(key, b"version 3").await.expect("Failed to set");

        // Restore most recent (version 2)
        backend.restore(key, None).await.expect("Failed to restore");

        let restored = backend.get(key).await.expect("Failed to get");
        assert_eq!(restored, Some(b"version 2".to_vec()));
    }

    #[tokio::test]
    async fn test_list_backups() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";

        // Create multiple backups
        backend.set(key, b"v1").await.expect("Failed to set");
        backend.backup(key).await.expect("Failed to backup");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        backend.set(key, b"v2").await.expect("Failed to set");
        backend.backup(key).await.expect("Failed to backup");

        let backups = backend
            .list_backups(key)
            .await
            .expect("Failed to list backups");

        assert_eq!(backups.len(), 2);
        // Should be newest first
        assert!(backups.first().unwrap() > backups.get(1).unwrap());
    }

    #[tokio::test]
    async fn test_delete_with_backup() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";
        let value = b"important data";

        backend.set(key, value).await.expect("Failed to set");

        // Delete with backup
        let backup_key = backend
            .delete_with_backup(key)
            .await
            .expect("Failed to delete");
        assert!(backup_key.is_some());

        // Original should be gone
        assert!(!backend.exists(key).await.expect("Failed to check exists"));

        // Backup should exist with original data
        let backup_data = backend
            .get(&backup_key.unwrap())
            .await
            .expect("Failed to get");
        assert_eq!(backup_data, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_set_with_backup() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/metadata";
        let original = b"original";
        let updated = b"updated";

        backend.set(key, original).await.expect("Failed to set");

        // Update with backup
        let backup_key = backend
            .set_with_backup(key, updated)
            .await
            .expect("Failed to set");
        assert!(backup_key.is_some());

        // Current value should be updated
        let current = backend.get(key).await.expect("Failed to get");
        assert_eq!(current, Some(updated.to_vec()));

        // Backup should have original
        let backup_data = backend
            .get(&backup_key.unwrap())
            .await
            .expect("Failed to get");
        assert_eq!(backup_data, Some(original.to_vec()));
    }

    #[tokio::test]
    async fn test_set_with_backup_no_existing() {
        let (backend, _temp_dir) = create_test_backend().await;

        let key = "ecosystems/test/new_key";
        let value = b"new data";

        // Set with backup when key doesn't exist
        let backup_key = backend
            .set_with_backup(key, value)
            .await
            .expect("Failed to set");
        assert!(backup_key.is_none()); // No backup created since nothing existed

        // Value should be set
        let current = backend.get(key).await.expect("Failed to get");
        assert_eq!(current, Some(value.to_vec()));
    }
}
