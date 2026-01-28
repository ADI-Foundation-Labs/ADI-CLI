//! State backend abstraction for ecosystem persistence.
//!
//! This module defines the `StateBackend` trait which provides an abstract
//! interface for persisting ecosystem and chain state. The default implementation
//! uses the filesystem, but the trait allows for future database backends.
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::state::{FilesystemBackend, StateBackend};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     // Create a filesystem backend
//!     let backend = FilesystemBackend::new(PathBuf::from("~/.adi_cli/state"))?;
//!
//!     // Store ecosystem metadata
//!     backend.set("ecosystems/my_ecosystem/metadata", b"data").await?;
//!
//!     // Retrieve it later
//!     if let Some(data) = backend.get("ecosystems/my_ecosystem/metadata").await? {
//!         println!("Got {} bytes", data.len());
//!     }
//!
//!     Ok(())
//! }
//! ```

mod filesystem;

// Re-export FilesystemBackend for external use
// Note: Currently unused as commands are implemented in later phases
#[allow(unused_imports)]
pub use filesystem::FilesystemBackend;

use crate::error::Result;
use async_trait::async_trait;

/// Result of state integrity validation.
///
/// Contains information about any issues found during validation,
/// including orphaned temp files and any corrupted or unreadable files.
// Note: Currently unused as commands are implemented in later phases
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Orphaned temporary files found (from interrupted writes).
    pub orphaned_temp_files: Vec<String>,
    /// Files that could not be read (potential corruption).
    pub unreadable_files: Vec<(String, String)>,
    /// Whether the validation passed (no critical issues).
    pub is_valid: bool,
}

#[allow(dead_code)]
impl ValidationResult {
    /// Creates a new validation result indicating success.
    pub fn ok() -> Self {
        Self {
            orphaned_temp_files: Vec::new(),
            unreadable_files: Vec::new(),
            is_valid: true,
        }
    }

    /// Returns true if there are any issues (warnings or errors).
    pub fn has_issues(&self) -> bool {
        !self.orphaned_temp_files.is_empty() || !self.unreadable_files.is_empty()
    }
}

/// Abstract interface for state persistence.
///
/// Implementations provide key-value storage for ecosystem and chain state.
/// Keys follow a hierarchical structure:
///
/// ```text
/// ecosystems/{name}/metadata
/// ecosystems/{name}/contracts
/// ecosystems/{name}/wallets
/// ecosystems/{name}/chains/{chain_name}/metadata
/// ecosystems/{name}/chains/{chain_name}/contracts
/// ecosystems/{name}/chains/{chain_name}/wallets
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use adi_cli::state::StateBackend;
///
/// async fn save_ecosystem(backend: &dyn StateBackend) -> Result<()> {
///     let data = b"ecosystem data";
///     backend.set("ecosystems/my_ecosystem/metadata", data).await?;
///     Ok(())
/// }
/// ```
// Note: Trait currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[async_trait]
pub trait StateBackend: Send + Sync {
    /// Retrieve value by key.
    ///
    /// Returns `Ok(Some(data))` if the key exists, `Ok(None)` if it doesn't,
    /// or an error if the operation fails.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Store value by key.
    ///
    /// Creates the key if it doesn't exist, or updates it if it does.
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Delete value by key.
    ///
    /// Returns success even if the key doesn't exist.
    async fn delete(&self, key: &str) -> Result<()>;

    /// List keys with prefix.
    ///
    /// Returns all keys that start with the given prefix.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;

    /// Check if key exists.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Validate state integrity.
    ///
    /// Checks for:
    /// - Orphaned temporary files from interrupted writes
    /// - Corrupted or unreadable state files
    /// - Directory structure validity
    ///
    /// Returns a `ValidationResult` containing any issues found.
    async fn validate(&self) -> Result<ValidationResult>;

    /// Clean up orphaned temporary files.
    ///
    /// Removes any `.*.tmp` files left over from interrupted atomic writes.
    /// Returns the number of files cleaned up.
    async fn cleanup_temp_files(&self) -> Result<usize>;

    /// Validate and ensure the state directory is ready for use.
    ///
    /// Performs the following checks:
    /// - Creates the directory if it doesn't exist
    /// - Verifies the directory is writable
    /// - Reports clear errors if directory cannot be used
    ///
    /// This should be called on startup before any state operations.
    async fn ensure_ready(&self) -> Result<()>;

    /// Get the base path of this state backend.
    ///
    /// Returns the root directory where state files are stored.
    fn base_path(&self) -> &std::path::Path;

    /// Create a backup of a key before a destructive operation.
    ///
    /// Creates a timestamped backup in the `.backups/` directory.
    /// Returns the backup key if backup was created, or None if key doesn't exist.
    ///
    /// Backup naming convention: `.backups/{key}/{timestamp}`
    async fn backup(&self, key: &str) -> Result<Option<String>>;

    /// Restore a key from a backup.
    ///
    /// If backup_key is None, restores from the most recent backup.
    async fn restore(&self, key: &str, backup_key: Option<&str>) -> Result<()>;

    /// List available backups for a key.
    ///
    /// Returns backup keys sorted by timestamp (newest first).
    async fn list_backups(&self, key: &str) -> Result<Vec<String>>;

    /// Delete a key with automatic backup.
    ///
    /// Creates a backup before deletion for safety.
    /// Returns the backup key if backup was created.
    async fn delete_with_backup(&self, key: &str) -> Result<Option<String>>;

    /// Set a key with automatic backup of existing value.
    ///
    /// Creates a backup of the existing value (if any) before overwriting.
    /// Returns the backup key if backup was created.
    async fn set_with_backup(&self, key: &str, value: &[u8]) -> Result<Option<String>>;
}
