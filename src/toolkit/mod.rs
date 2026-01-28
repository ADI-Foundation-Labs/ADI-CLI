//! Toolkit abstraction for executing commands in Docker containers.
//!
//! This module provides a high-level interface for running zkstack, forge,
//! and cast commands inside pre-built Docker toolkit containers.
//!
//! # Overview
//!
//! The ADI CLI orchestrates pre-built Docker toolkit images that contain:
//! - **zkstack CLI**: ZkSync ecosystem and chain management
//! - **foundry-zksync**: forge and cast for contract interactions
//! - **era-contracts**: Smart contract sources and upgrade scripts
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  ToolkitRunner                                          │
//! │  ├── Manages toolkit image versions                     │
//! │  ├── Creates ephemeral containers                       │
//! │  ├── Mounts state directory as /workspace               │
//! │  └── Streams output in real-time                        │
//! └────────────────────┬────────────────────────────────────┘
//!                      │
//! ┌────────────────────▼────────────────────────────────────┐
//! │  adi-toolkit:v{VERSION}                                 │
//! │  ├── /workspace (mounted from host state_dir)           │
//! │  ├── /era-contracts                                     │
//! │  ├── zkstack (CLI binary)                               │
//! │  ├── forge (Foundry)                                    │
//! │  └── cast (Foundry)                                     │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::toolkit::ToolkitRunner;
//! use semver::Version;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     // Connect to Docker and create runner
//!     let runner = ToolkitRunner::connect()?;
//!
//!     // Check Docker is available
//!     if !runner.is_docker_available().await? {
//!         return Err(eyre::eyre!("Docker daemon is not running"));
//!     }
//!
//!     let version = Version::new(29, 0, 11);
//!     let state_dir = PathBuf::from("/home/user/.adi_cli/state");
//!
//!     // Ensure toolkit image is available
//!     runner.ensure_image(&version, |msg| {
//!         log::info!("{}", msg);
//!     }).await?;
//!
//!     // Run zkstack command
//!     let result = runner.run_zkstack(
//!         &["--version"],
//!         &state_dir,
//!         &version,
//!         |line| print!("{}", line.content()),
//!     ).await?;
//!
//!     if !result.success() {
//!         return Err(eyre::eyre!("Command failed with exit code: {}", result.exit_code));
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Protocol Version Support
//!
//! Each toolkit image is tagged with a protocol version (e.g., `v29.0.11`).
//! The runner ensures the correct image is used for each operation.
//!
//! ```rust,ignore
//! // Use v29 toolkit
//! let v29 = Version::new(29, 0, 11);
//! runner.run_zkstack(&["ecosystem", "create"], &state_dir, &v29, |_| {}).await?;
//!
//! // Use v30 toolkit for upgrades
//! let v30 = Version::new(30, 0, 0);
//! runner.run_forge(&["script", "Upgrade.s.sol"], &state_dir, &v30, |_| {}).await?;
//! ```

mod config;
mod runner;

// Re-export main types
// Note: These are public API - used by commands and future external consumers
#[allow(unused_imports)]
pub use config::{ContainerPaths, ToolkitConfig};
#[allow(unused_imports)]
pub use runner::{run_simple, ToolkitResult, ToolkitRunner};
