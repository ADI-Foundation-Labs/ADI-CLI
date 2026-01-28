//! Toolkit command execution via Docker containers.
//!
// Note: This module is part of the Toolkit API (T099-T101).
// It will be used by command implementations that execute in Docker containers.
#![allow(dead_code)]
//!
//! Provides the `ToolkitRunner` struct for executing zkstack, forge, and cast
//! commands inside ephemeral Docker toolkit containers.
//!
//! # Container Lifecycle
//!
//! Each command execution follows this lifecycle:
//!
//! ```text
//! 1. Ensure toolkit image is available (pull if needed)
//! 2. Create ephemeral container with volume mounts
//! 3. Start container and stream output
//! 4. Wait for completion
//! 5. Remove container
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
//!     let runner = ToolkitRunner::connect()?;
//!     let version = Version::new(29, 0, 11);
//!     let state_dir = PathBuf::from("/path/to/state");
//!
//!     // Run zkstack command
//!     let result = runner.run_zkstack(
//!         &["ecosystem", "create", "--help"],
//!         &state_dir,
//!         &version,
//!         |line| println!("{}", line.content()),
//!     ).await?;
//!
//!     println!("Exit code: {}", result.exit_code);
//!     Ok(())
//! }
//! ```

use crate::config::DockerConfig;
use crate::docker::{
    ContainerConfig, ContainerManager, ContainerResult, DockerClient, ImageManager, OutputLine,
    OutputStreamer, VolumeMount,
};
use crate::error::{Result, WrapErr};
use semver::Version;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::config::{ContainerPaths, ToolkitConfig};

/// Executes commands inside Docker toolkit containers.
///
/// The `ToolkitRunner` manages the lifecycle of toolkit containers,
/// including image pulling, container creation, output streaming,
/// and cleanup.
#[derive(Debug, Clone)]
pub struct ToolkitRunner {
    client: DockerClient,
    images: ImageManager,
    containers: ContainerManager,
    streamer: OutputStreamer,
    toolkit_config: ToolkitConfig,
}

/// Result of a toolkit command execution.
#[derive(Debug, Clone)]
pub struct ToolkitResult {
    /// Container exit code (0 = success).
    pub exit_code: i64,
    /// Captured stdout output.
    pub stdout: String,
    /// Captured stderr output.
    pub stderr: String,
}

impl ToolkitResult {
    /// Check if the command was successful (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output (stdout + stderr).
    #[allow(dead_code)] // May be used for simple output handling
    pub fn combined_output(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

impl ToolkitRunner {
    /// Connect to Docker and create a toolkit runner.
    ///
    /// Uses default Docker connection and toolkit configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if Docker daemon is not accessible.
    pub fn connect() -> Result<Self> {
        let client = DockerClient::connect()?;
        Ok(Self::new(client, ToolkitConfig::default()))
    }

    /// Create a toolkit runner with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `client` - Docker client.
    /// * `toolkit_config` - Toolkit image configuration.
    pub fn new(client: DockerClient, toolkit_config: ToolkitConfig) -> Self {
        Self {
            images: ImageManager::new(client.clone()),
            containers: ContainerManager::new(client.clone()),
            streamer: OutputStreamer::new(client.clone()),
            client,
            toolkit_config,
        }
    }

    /// Create a toolkit runner from application config.
    ///
    /// # Arguments
    ///
    /// * `docker_config` - Docker configuration from the application.
    pub fn from_config(docker_config: &DockerConfig) -> Result<Self> {
        let client = DockerClient::connect()?;
        let toolkit_config = ToolkitConfig::from(docker_config);
        Ok(Self::new(client, toolkit_config))
    }

    /// Check if Docker daemon is available.
    pub async fn is_docker_available(&self) -> Result<bool> {
        self.client.is_available().await
    }

    /// Ensure the toolkit image for a version is available.
    ///
    /// Pulls the image if it doesn't exist locally.
    ///
    /// # Arguments
    ///
    /// * `version` - Protocol version.
    /// * `progress_callback` - Called with progress updates during pull.
    ///
    /// # Returns
    ///
    /// `true` if the image was pulled, `false` if it already existed.
    pub async fn ensure_image<F>(&self, version: &Version, progress_callback: F) -> Result<bool>
    where
        F: Fn(String),
    {
        let image = self.toolkit_config.image_for_version(version);
        self.images
            .ensure(&image, |p| progress_callback(p.to_string()))
            .await
    }

    /// Run a zkstack CLI command.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments (e.g., `["ecosystem", "create"]`).
    /// * `state_dir` - Host directory to mount as `/workspace`.
    /// * `version` - Protocol version for selecting toolkit image.
    /// * `output_callback` - Called with each line of output.
    ///
    /// # Returns
    ///
    /// Toolkit result with exit code and captured output.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = runner.run_zkstack(
    ///     &["ecosystem", "create", "--name", "test"],
    ///     &PathBuf::from("/workspace"),
    ///     &Version::new(29, 0, 11),
    ///     |line| println!("{}", line.content()),
    /// ).await?;
    /// ```
    pub async fn run_zkstack<F>(
        &self,
        args: &[&str],
        state_dir: &Path,
        version: &Version,
        output_callback: F,
    ) -> Result<ToolkitResult>
    where
        F: Fn(OutputLine) + Send + Sync + 'static,
    {
        let mut cmd = vec![ContainerPaths::ZKSTACK_BIN];
        cmd.extend(args);
        self.run_command(&cmd, state_dir, version, output_callback)
            .await
    }

    /// Run a forge CLI command.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments (e.g., `["script", "Deploy.s.sol"]`).
    /// * `state_dir` - Host directory to mount as `/workspace`.
    /// * `version` - Protocol version for selecting toolkit image.
    /// * `output_callback` - Called with each line of output.
    ///
    /// # Returns
    ///
    /// Toolkit result with exit code and captured output.
    pub async fn run_forge<F>(
        &self,
        args: &[&str],
        state_dir: &Path,
        version: &Version,
        output_callback: F,
    ) -> Result<ToolkitResult>
    where
        F: Fn(OutputLine) + Send + Sync + 'static,
    {
        let mut cmd = vec![ContainerPaths::FORGE_BIN];
        cmd.extend(args);
        self.run_command(&cmd, state_dir, version, output_callback)
            .await
    }

    /// Run a cast CLI command.
    ///
    /// Note: cast commands typically don't need a workspace mount,
    /// but we include it for consistency.
    ///
    /// # Arguments
    ///
    /// * `args` - Command arguments (e.g., `["call", "0x...", "balanceOf(address)"]`).
    /// * `version` - Protocol version for selecting toolkit image.
    /// * `output_callback` - Called with each line of output.
    ///
    /// # Returns
    ///
    /// Toolkit result with exit code and captured output.
    pub async fn run_cast<F>(
        &self,
        args: &[&str],
        version: &Version,
        output_callback: F,
    ) -> Result<ToolkitResult>
    where
        F: Fn(OutputLine) + Send + Sync + 'static,
    {
        let mut cmd = vec![ContainerPaths::CAST_BIN];
        cmd.extend(args);

        // Cast doesn't typically need a workspace, but we need to provide
        // a valid path for the container. Use a temp directory.
        let temp_dir = std::env::temp_dir();
        self.run_command(&cmd, &temp_dir, version, output_callback)
            .await
    }

    /// Run an arbitrary command in the toolkit container.
    ///
    /// This is the core execution method used by `run_zkstack`, `run_forge`,
    /// and `run_cast`.
    ///
    /// # Arguments
    ///
    /// * `cmd` - Full command to run (including binary name).
    /// * `state_dir` - Host directory to mount as `/workspace`.
    /// * `version` - Protocol version for selecting toolkit image.
    /// * `output_callback` - Called with each line of output.
    async fn run_command<F>(
        &self,
        cmd: &[&str],
        state_dir: &Path,
        version: &Version,
        output_callback: F,
    ) -> Result<ToolkitResult>
    where
        F: Fn(OutputLine) + Send + Sync + 'static,
    {
        let image = self.toolkit_config.image_for_version(version);

        // Ensure image is available
        self.images
            .ensure(&image, |p| {
                log::info!("Pulling image: {}", p);
            })
            .await
            .wrap_err_with(|| format!("Failed to ensure toolkit image: {image}"))?;

        // Build container config
        let config = ContainerConfig::new(&image)
            .with_cmd(cmd.iter().map(|s| s.to_string()).collect())
            .with_mount(VolumeMount::new(state_dir, ContainerPaths::WORKSPACE))
            .with_working_dir(ContainerPaths::WORKSPACE)
            .with_host_network()
            .with_label("adi-cli", "toolkit");

        // Create container
        let container_id = self.containers.create(&config).await?;

        // Collect output
        let stdout = Arc::new(Mutex::new(String::new()));
        let stderr = Arc::new(Mutex::new(String::new()));
        let stdout_clone = Arc::clone(&stdout);
        let stderr_clone = Arc::clone(&stderr);

        // Wrap callback in Arc for sharing across threads
        let callback = Arc::new(output_callback);

        // Start container
        self.containers.start(&container_id).await?;

        // Stream output
        let streamer = self.streamer.clone();
        let container_id_clone = container_id.clone();
        let stream_handle = tokio::spawn(async move {
            let _ = streamer
                .stream_logs(&container_id_clone, |line| {
                    // Capture output
                    match &line {
                        OutputLine::Stdout(s) => {
                            if let Ok(mut out) = stdout_clone.lock() {
                                out.push_str(s);
                            }
                        }
                        OutputLine::Stderr(s) => {
                            if let Ok(mut err) = stderr_clone.lock() {
                                err.push_str(s);
                            }
                        }
                    }
                    // Call user callback
                    callback(line);
                })
                .await;
        });

        // Wait for container
        let exit_code = self.containers.wait(&container_id).await?;

        // Wait for streaming to complete
        let _ = stream_handle.await;

        // Remove container
        self.containers.remove(&container_id).await?;

        // Extract captured output
        let stdout_str = stdout
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| String::new());
        let stderr_str = stderr
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| String::new());

        Ok(ToolkitResult {
            exit_code,
            stdout: stdout_str,
            stderr: stderr_str,
        })
    }

    /// Run a command without streaming output (collect only).
    ///
    /// This is useful when you need to parse the output but don't
    /// need real-time display.
    ///
    /// # Arguments
    ///
    /// * `cmd` - Full command to run.
    /// * `state_dir` - Host directory to mount.
    /// * `version` - Protocol version.
    #[allow(dead_code)] // May be used for quiet operations
    pub async fn run_quiet(
        &self,
        cmd: &[&str],
        state_dir: &Path,
        version: &Version,
    ) -> Result<ToolkitResult> {
        self.run_command(cmd, state_dir, version, |_| {}).await
    }

    /// Get the toolkit image reference for a version.
    pub fn image_for_version(&self, version: &Version) -> String {
        self.toolkit_config.image_for_version(version)
    }

    /// Get the toolkit configuration.
    pub fn config(&self) -> &ToolkitConfig {
        &self.toolkit_config
    }
}

/// Convenience function to run a simple container and get the exit code.
///
/// This is useful for quick one-off executions without setting up
/// a full `ToolkitRunner`.
#[allow(dead_code)] // May be used for simple operations
pub async fn run_simple(config: &ContainerConfig) -> Result<ContainerResult> {
    let client = DockerClient::connect()?;
    let containers = ContainerManager::new(client);
    containers.run(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolkit_result_success() {
        let result = ToolkitResult {
            exit_code: 0,
            stdout: "output".to_string(),
            stderr: String::new(),
        };
        assert!(result.success());
    }

    #[test]
    fn test_toolkit_result_failure() {
        let result = ToolkitResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!result.success());
    }

    #[test]
    fn test_toolkit_result_combined() {
        let result = ToolkitResult {
            exit_code: 0,
            stdout: "out\n".to_string(),
            stderr: "err\n".to_string(),
        };
        assert_eq!(result.combined_output(), "out\nerr\n");
    }
}
