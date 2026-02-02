//! State backend abstraction.

mod filesystem;

pub use filesystem::FilesystemBackend;

use crate::error::Result;
use async_trait::async_trait;
use std::path::Path;

/// Backend type for state storage.
#[derive(Clone, Debug, Default)]
pub enum BackendType {
    /// Filesystem-based storage (default).
    #[default]
    Filesystem,
}

/// Abstract state storage backend.
///
/// Provides low-level key-value operations for state persistence.
/// Keys are relative paths within the state directory.
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
/// // Read raw YAML content
/// let content = backend.read("configs/wallets.yaml").await?;
///
/// // Check if file exists
/// let exists = backend.exists("ZkStack.yaml").await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait StateBackend: Send + Sync {
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
    async fn read(&self, key: &str) -> Result<String>;

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
    async fn write(&self, key: &str, content: &str) -> Result<()>;

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
}

/// Create a backend instance based on backend type.
///
/// # Arguments
///
/// * `backend_type` - The type of backend to create.
/// * `base_path` - Base path for the backend (interpretation depends on type).
#[must_use]
pub fn create_backend(backend_type: BackendType, base_path: &Path) -> Box<dyn StateBackend> {
    match backend_type {
        BackendType::Filesystem => Box::new(FilesystemBackend::new(base_path)),
    }
}
