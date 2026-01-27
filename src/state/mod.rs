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
}
