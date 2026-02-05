//! Pure Docker management SDK for container orchestration.
//!
//! This crate provides low-level Docker operations:
//! - Daemon connection and health checking
//! - Image management (pull, check existence)
//! - Container lifecycle (create, start, wait, remove)
//! - Real-time output streaming
//!
//! # Overview
//!
//! The `adi-docker` crate abstracts Docker operations for running containers.
//! It is used by `adi-toolkit` which adds knowledge about toolkit images and versions.
//!
//! # Example
//!
//! ```rust,no_run
//! use adi_docker::{DockerClient, ContainerConfig, ContainerManager};
//!
//! # async fn example() -> adi_docker::Result<()> {
//! // Create client and verify Docker is running
//! let client = DockerClient::new().await?;
//!
//! // Pull image if needed
//! client.pull_image("alpine:latest").await?;
//!
//! // Run container
//! let config = ContainerConfig {
//!     command: vec!["echo".to_string(), "hello".to_string()],
//!     ..Default::default()
//! };
//!
//! let manager = ContainerManager::new(client.inner().clone());
//! let exit_code = manager.run("alpine:latest", &config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Error Handling
//!
//! All operations return [`Result<T>`] with [`DockerError`] variants for
//! specific failure modes:
//!
//! - [`DockerError::DaemonNotRunning`] - Docker daemon not accessible
//! - [`DockerError::PullFailed`] - Image pull failed
//! - [`DockerError::Timeout`] - Operation exceeded timeout
//! - [`DockerError::ContainerFailed`] - Container exited with error

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod client;
mod config;
mod container;
mod error;
mod image;
mod stream;
mod url;

// Public re-exports
pub use client::DockerClient;
pub use config::{ContainerConfig, DEFAULT_TIMEOUT_SECONDS};
pub use container::ContainerManager;
pub use error::{DockerError, Result};
pub use url::transform_url_for_container;
