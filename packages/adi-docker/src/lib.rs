//! Docker orchestration SDK for ADI toolkit containers.
//!
//! This crate provides a high-level interface for managing Docker containers
//! that run the ADI toolkit (zkstack, foundry-zksync, era-contracts).
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
//! # Container Lifecycle
//!
//! All containers are ephemeral (created, run, removed per operation):
//!
//! 1. **Check daemon** - Verify Docker is running
//! 2. **Pull image** - Download if not available locally
//! 3. **Create container** - With volume mount and host network
//! 4. **Start + stream** - Run with real-time output streaming
//! 5. **Wait** - Wait for completion with configurable timeout
//! 6. **Remove** - Clean up container
//!
//! # Example
//!
//! ```rust,no_run
//! use adi_docker::{DockerClient, DockerConfig, ToolkitRunner};
//! use semver::Version;
//! use std::path::Path;
//!
//! # async fn example() -> adi_docker::Result<()> {
//! // Create client and verify Docker is running
//! let client = DockerClient::new().await?;
//!
//! // Configure with defaults (harbor.io/adi, 30min timeout)
//! let config = DockerConfig::default();
//!
//! // Create runner
//! let runner = ToolkitRunner::new(client, config);
//!
//! // Run zkstack command
//! let version = Version::new(29, 0, 11);
//! let state_dir = Path::new("/home/user/.adi_cli/state");
//!
//! let exit_code = runner.run_zkstack(
//!     &["ecosystem", "init"],
//!     state_dir,
//!     &version
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
//! The [`DockerConfig`] struct controls:
//! - Registry URL (default: `harbor.io/adi`)
//! - Image name (default: `adi-toolkit`)
//! - Operation timeout (default: 30 minutes)
//!
//! Image tags are derived from the protocol version: `v{major}.{minor}.{patch}`
//!
//! # Error Handling
//!
//! All operations return [`Result<T>`] with [`DockerError`] variants for
//! specific failure modes:
//!
//! - [`DockerError::DaemonNotRunning`] - Docker daemon not accessible
//! - [`DockerError::PullFailed`] - Image pull failed (check `docker login`)
//! - [`DockerError::Timeout`] - Operation exceeded timeout
//! - [`DockerError::ContainerFailed`] - Container exited with error

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod client;
mod config;
mod container;
mod error;
mod image;
mod runner;
mod stream;

// Public re-exports
pub use client::DockerClient;
pub use config::{ContainerConfig, DockerConfig, ImageReference};
pub use config::{DEFAULT_IMAGE_NAME, DEFAULT_REGISTRY, DEFAULT_TIMEOUT_SECONDS};
pub use error::{DockerError, Result};
pub use runner::ToolkitRunner;
