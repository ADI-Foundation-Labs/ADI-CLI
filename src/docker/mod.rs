//! Docker orchestration for the ADI CLI.
//!
//! This module provides Docker container orchestration using the Bollard crate.
//! It handles:
//!
//! - **Client**: Connection to Docker daemon
//! - **Images**: Pulling and managing toolkit images
//! - **Containers**: Creating and running ephemeral containers
//! - **Streaming**: Real-time output from containers
//!
//! # Architecture
//!
//! The CLI runs on the host machine and orchestrates pre-built toolkit
//! Docker images containing zkstack, foundry-zksync, and era-contracts.
//!
//! ```text
//! Host Machine
//! ┌─────────────────────────────────────────────────────────┐
//! │  adi-cli (Rust binary)                                  │
//! │  ├── DockerClient (Bollard)                             │
//! │  ├── ImageManager (pull/ensure images)                  │
//! │  ├── ContainerManager (create/run/remove)               │
//! │  └── OutputStreamer (real-time logs)                    │
//! └────────────────────┬────────────────────────────────────┘
//!                      │ Docker API
//! ┌────────────────────▼────────────────────────────────────┐
//! │  Docker Daemon                                          │
//! │  └── adi-toolkit:v{VERSION} (ephemeral container)       │
//! │      ├── zkstack CLI                                    │
//! │      ├── foundry-zksync (forge, cast)                   │
//! │      └── era-contracts                                  │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::docker::{DockerClient, ImageManager, ContainerManager, ContainerConfig, VolumeMount};
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     // Connect to Docker
//!     let client = DockerClient::connect()?;
//!
//!     // Check daemon is running
//!     if !client.is_available().await? {
//!         return Err(eyre::eyre!("Docker daemon is not running"));
//!     }
//!
//!     // Ensure toolkit image is available
//!     let images = ImageManager::new(client.clone());
//!     images.ensure("harbor.io/adi/adi-toolkit:v29.0.11", |p| {
//!         log::info!("{}", p);
//!     }).await?;
//!
//!     // Run a command in the toolkit
//!     let containers = ContainerManager::new(client);
//!     let config = ContainerConfig::new("harbor.io/adi/adi-toolkit:v29.0.11")
//!         .with_cmd(vec!["zkstack", "--version"])
//!         .with_mount(VolumeMount::new("/host/workspace", "/workspace"))
//!         .with_working_dir("/workspace")
//!         .with_host_network();
//!
//!     let result = containers.run(&config).await?;
//!     println!("Exit code: {}", result.exit_code);
//!
//!     Ok(())
//! }
//! ```

mod client;
mod container;
mod image;
mod stream;

// Re-export main types
// Note: These are public API - used by toolkit module and future external consumers
#[allow(unused_imports)]
pub use client::{DockerClient, DockerVersion};
#[allow(unused_imports)]
pub use container::{ContainerConfig, ContainerManager, ContainerResult, EnvVar, VolumeMount};
#[allow(unused_imports)]
pub use image::{ImageInfo, ImageManager, PullProgress};
#[allow(unused_imports)]
pub use stream::{CollectedOutput, OutputLine, OutputStreamer};
