//! State backend abstraction for ecosystem persistence.
//!
//! This module defines the `StateBackend` trait which provides an abstract
//! interface for persisting ecosystem and chain state. The default implementation
//! uses the filesystem, but the trait allows for future database backends.

use crate::error::Result;
use async_trait::async_trait;

/// Abstract interface for state persistence.
#[allow(dead_code)] // Will be used in Phase 2 when FilesystemBackend is implemented
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
