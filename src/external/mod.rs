//! External tool wrappers for zkstack, forge, and cast CLIs.
//!
//! This module provides typed Rust wrappers around external CLI tools
//! that run inside Docker toolkit containers. The wrappers handle:
//!
//! - Command construction with proper arguments
//! - Async execution via Docker containers
//! - Output parsing and error handling
//! - Version checking and compatibility validation
//!
//! # Supported Tools
//!
//! - **zkstack**: ZkSync ecosystem and chain management CLI
//! - **forge**: Solidity smart contract compilation and deployment
//! - **cast**: Ethereum RPC interactions and calldata encoding
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::external::{ZkstackCli, ForgeCli, CastCli, check_all_tools};
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     // Check all tools are available
//!     let status = check_all_tools().await?;
//!     println!("All tools available: {}", status.all_available());
//!
//!     // Use individual tools
//!     let zkstack = ZkstackCli::new();
//!     let forge = ForgeCli::new();
//!     let cast = CastCli::new();
//!
//!     Ok(())
//! }
//! ```

mod cast;
mod forge;
mod zkstack;

// Re-export CLI wrappers
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(unused_imports)]
pub use cast::{CastCli, CastOutput, SendOptions};
#[allow(unused_imports)]
pub use forge::{ForgeCli, ForgeOutput};
#[allow(unused_imports)]
pub use zkstack::{
    ChainInitConfig, CommandOutput, EcosystemCreateConfig, EcosystemInitConfig, ZkstackCli,
};

use crate::error::Result;

/// Status of an external tool check.
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ToolStatus {
    /// Name of the tool.
    pub name: String,
    /// Whether the tool is available.
    pub available: bool,
    /// Version string if available.
    pub version: Option<String>,
    /// Error message if not available.
    pub error: Option<String>,
    /// Path to the tool binary.
    pub path: String,
}

impl ToolStatus {
    /// Creates a new ToolStatus for an available tool.
    fn available(
        name: impl Into<String>,
        version: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            available: true,
            version: Some(version.into()),
            error: None,
            path: path.into(),
        }
    }

    /// Creates a new ToolStatus for an unavailable tool.
    fn unavailable(
        name: impl Into<String>,
        error: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            available: false,
            version: None,
            error: Some(error.into()),
            path: path.into(),
        }
    }
}

/// Combined status of all external tools.
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AllToolsStatus {
    /// Status of zkstack CLI.
    pub zkstack: ToolStatus,
    /// Status of forge CLI.
    pub forge: ToolStatus,
    /// Status of cast CLI.
    pub cast: ToolStatus,
}

#[allow(dead_code)]
impl AllToolsStatus {
    /// Returns true if all tools are available.
    pub fn all_available(&self) -> bool {
        self.zkstack.available && self.forge.available && self.cast.available
    }

    /// Returns a list of unavailable tools.
    pub fn unavailable_tools(&self) -> Vec<&ToolStatus> {
        let mut unavailable = Vec::new();
        if !self.zkstack.available {
            unavailable.push(&self.zkstack);
        }
        if !self.forge.available {
            unavailable.push(&self.forge);
        }
        if !self.cast.available {
            unavailable.push(&self.cast);
        }
        unavailable
    }
}

/// Checks if zkstack CLI is available.
///
/// # Returns
///
/// A `ToolStatus` with version info if available, or error if not.
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub async fn check_zkstack() -> ToolStatus {
    let zkstack = ZkstackCli::new();
    match zkstack.version().await {
        Ok(version) => ToolStatus::available("zkstack", version, zkstack.binary_path()),
        Err(e) => ToolStatus::unavailable("zkstack", e.to_string(), zkstack.binary_path()),
    }
}

/// Checks if forge CLI is available.
///
/// # Returns
///
/// A `ToolStatus` with version info if available, or error if not.
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub async fn check_forge() -> ToolStatus {
    let forge = ForgeCli::new();
    match forge.version().await {
        Ok(version) => ToolStatus::available("forge", version, forge.binary_path()),
        Err(e) => ToolStatus::unavailable("forge", e.to_string(), forge.binary_path()),
    }
}

/// Checks if cast CLI is available.
///
/// # Returns
///
/// A `ToolStatus` with version info if available, or error if not.
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub async fn check_cast() -> ToolStatus {
    let cast = CastCli::new();
    match cast.version().await {
        Ok(version) => ToolStatus::available("cast", version, cast.binary_path()),
        Err(e) => ToolStatus::unavailable("cast", e.to_string(), cast.binary_path()),
    }
}

/// Checks all external tools and returns their status.
///
/// This function checks zkstack, forge, and cast CLIs in parallel
/// and returns a combined status report.
///
/// # Example
///
/// ```rust,ignore
/// use adi_cli::external::check_all_tools;
///
/// #[tokio::main]
/// async fn main() -> eyre::Result<()> {
///     let status = check_all_tools().await?;
///
///     if status.all_available() {
///         println!("All tools ready!");
///     } else {
///         for tool in status.unavailable_tools() {
///             println!("{} not available: {:?}", tool.name, tool.error);
///         }
///     }
///
///     Ok(())
/// }
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub async fn check_all_tools() -> Result<AllToolsStatus> {
    // Run all checks in parallel using tokio::join!
    let (zkstack, forge, cast) = tokio::join!(check_zkstack(), check_forge(), check_cast());

    Ok(AllToolsStatus {
        zkstack,
        forge,
        cast,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_status_available() {
        let status = ToolStatus::available("test", "1.0.0", "/usr/bin/test");
        assert!(status.available);
        assert_eq!(status.version, Some("1.0.0".to_string()));
        assert!(status.error.is_none());
    }

    #[test]
    fn test_tool_status_unavailable() {
        let status = ToolStatus::unavailable("test", "not found", "test");
        assert!(!status.available);
        assert!(status.version.is_none());
        assert_eq!(status.error, Some("not found".to_string()));
    }

    #[test]
    fn test_all_tools_status_all_available() {
        let status = AllToolsStatus {
            zkstack: ToolStatus::available("zkstack", "1.0", "zkstack"),
            forge: ToolStatus::available("forge", "1.0", "forge"),
            cast: ToolStatus::available("cast", "1.0", "cast"),
        };
        assert!(status.all_available());
        assert!(status.unavailable_tools().is_empty());
    }

    #[test]
    fn test_all_tools_status_some_unavailable() {
        let status = AllToolsStatus {
            zkstack: ToolStatus::available("zkstack", "1.0", "zkstack"),
            forge: ToolStatus::unavailable("forge", "not found", "forge"),
            cast: ToolStatus::available("cast", "1.0", "cast"),
        };
        assert!(!status.all_available());
        assert_eq!(status.unavailable_tools().len(), 1);
        assert_eq!(
            status.unavailable_tools().first().map(|t| t.name.as_str()),
            Some("forge")
        );
    }
}
