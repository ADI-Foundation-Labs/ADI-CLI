//! SDK for running ADI toolkit containers (zkstack, forge, cast).
//!
//! This crate provides a high-level interface for executing commands in
//! Docker toolkit containers. It knows about:
//! - Protocol versions and image tagging
//! - Registry configuration
//! - zkstack, forge, and cast command execution
//!
//! # Overview
//!
//! The ADI CLI runs on the host machine and orchestrates pre-built Docker
//! toolkit images. These images contain all the tools needed for ZkSync
//! ecosystem management:
//!
//! - `zkstack` - ZkSync stack CLI for ecosystem/chain operations
//! - `forge` - Foundry Solidity compiler and tester
//! - `cast` - Foundry EVM interaction tool
//! - `era-contracts` - Smart contract upgrade scripts
//!
//! # Example
//!
//! ```rust,no_run
//! use adi_toolkit::{ToolkitRunner, ProtocolVersion};
//! use std::path::Path;
//!
//! # async fn example() -> adi_toolkit::Result<()> {
//! // Create runner (connects to Docker)
//! let runner = ToolkitRunner::new().await?;
//!
//! // Parse version
//! let version = ProtocolVersion::parse("v30.0.2").expect("valid version");
//! let state_dir = Path::new("/home/user/.adi_cli/state");
//!
//! // Run zkstack command
//! let exit_code = runner.run_zkstack(
//!     &["ecosystem", "init"],
//!     state_dir,
//!     state_dir, // log_dir
//!     &version.to_semver(),
//! ).await?;
//!
//! if exit_code == 0 {
//!     println!("Ecosystem initialized successfully!");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Configuration
//!
//! The [`ToolkitConfig`] struct controls:
//! - Registry URL (default: `adi-chain/cli`)
//! - Image name (default: `adi-toolkit`)
//! - Operation timeout (default: 1 hour)
//!
//! Image tags are derived from the protocol version: `v{major}.{minor}.{patch}`

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod cleanup;
mod config;
mod error;
mod runner;
mod version;

// Public re-exports
pub use adi_docker::DEFAULT_TIMEOUT_SECONDS;
pub use cleanup::cleanup_tmp_dir;
pub use config::{ImageReference, ToolkitConfig};
pub use config::{DEFAULT_IMAGE_NAME, DEFAULT_REGISTRY};
pub use error::{Result, ToolkitError};
pub use runner::ToolkitRunner;
pub use version::{ParseError, ProtocolVersion};
