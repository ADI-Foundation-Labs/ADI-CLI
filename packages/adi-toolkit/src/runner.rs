//! Toolkit command execution via Docker containers.

use crate::client::DockerClient;
use crate::config::{ContainerConfig, DockerConfig};
use crate::container::ContainerManager;
use crate::error::Result;
use semver::Version;
use std::path::Path;

/// Executes commands inside Docker toolkit containers.
///
/// Container lifecycle: create -> start -> stream output -> wait -> remove
///
/// # Example
///
/// ```rust,no_run
/// use adi_toolkit::{DockerClient, DockerConfig, ToolkitRunner};
/// use semver::Version;
/// use std::path::Path;
///
/// # async fn example() -> adi_toolkit::Result<()> {
/// let client = DockerClient::new().await?;
/// let config = DockerConfig::default();
/// let runner = ToolkitRunner::new(client, config);
///
/// let version = Version::new(29, 0, 11);
/// let state_dir = Path::new("/home/user/.adi_cli/state");
///
/// // Run zkstack command
/// let exit_code = runner.run_zkstack(&["chain", "init"], state_dir, &version).await?;
/// # Ok(())
/// # }
/// ```
pub struct ToolkitRunner {
    client: DockerClient,
    config: DockerConfig,
}

impl ToolkitRunner {
    /// Create a new ToolkitRunner.
    ///
    /// # Arguments
    ///
    /// * `client` - Docker client for container operations.
    /// * `config` - Docker configuration with registry and timeout settings.
    #[must_use]
    pub fn new(client: DockerClient, config: DockerConfig) -> Self {
        Self { client, config }
    }

    /// Execute a generic command in the toolkit container.
    ///
    /// # Arguments
    ///
    /// * `command` - The command and arguments to execute.
    /// * `state_dir` - Host directory to mount as /workspace.
    /// * `protocol_version` - Protocol version to select toolkit image.
    /// * `env_vars` - Additional environment variables.
    ///
    /// # Returns
    ///
    /// Container exit code (0 = success).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Docker daemon is not running
    /// - Image cannot be pulled
    /// - Container fails to start
    /// - Operation times out
    pub async fn run_command(
        &self,
        command: &[&str],
        state_dir: &Path,
        protocol_version: &Version,
        env_vars: &[(&str, &str)],
    ) -> Result<i64> {
        let image_ref = self.config.image_reference(protocol_version);

        log::debug!(
            "Running command: {:?} (image: {}, state_dir: {})",
            command,
            image_ref.full_uri(),
            state_dir.display()
        );

        // Ensure image is available
        self.client.pull_image(&image_ref).await?;

        let container_config = ContainerConfig {
            state_dir: state_dir.to_path_buf(),
            command: command.iter().map(|s| (*s).to_string()).collect(),
            env_vars: env_vars
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            timeout_seconds: self.config.timeout_seconds,
            ..Default::default()
        };

        log::debug!(
            "Container config: working_dir={}, timeout={}s, env_vars={:?}",
            container_config.working_dir,
            container_config.timeout_seconds,
            container_config
                .env_vars
                .iter()
                .map(|(k, _)| k)
                .collect::<Vec<_>>()
        );

        let manager = ContainerManager::new(self.client.inner().clone());
        let exit_code = manager.run(&image_ref, &container_config).await?;

        log::debug!("Command completed with exit code: {}", exit_code);
        Ok(exit_code)
    }

    /// Execute zkstack CLI command in toolkit container.
    ///
    /// # Arguments
    ///
    /// * `args` - Arguments to pass to zkstack.
    /// * `state_dir` - Host directory containing ecosystem state.
    /// * `protocol_version` - Protocol version for toolkit image selection.
    ///
    /// # Returns
    ///
    /// Container exit code.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use adi_toolkit::{DockerClient, DockerConfig, ToolkitRunner};
    /// # use semver::Version;
    /// # use std::path::Path;
    /// # async fn example() -> adi_toolkit::Result<()> {
    /// # let client = DockerClient::new().await?;
    /// # let runner = ToolkitRunner::new(client, DockerConfig::default());
    /// let version = Version::new(29, 0, 11);
    /// let state_dir = Path::new("/home/user/.adi_cli/state");
    ///
    /// // Run: zkstack ecosystem init
    /// let exit_code = runner.run_zkstack(
    ///     &["ecosystem", "init"],
    ///     state_dir,
    ///     &version
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_zkstack(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &Version,
    ) -> Result<i64> {
        log::debug!("Running zkstack with args: {:?}", args);
        let mut command = vec!["zkstack"];
        command.extend(args);

        self.run_command(&command, state_dir, protocol_version, &[])
            .await
    }

    /// Execute forge command in toolkit container.
    ///
    /// # Arguments
    ///
    /// * `args` - Arguments to pass to forge.
    /// * `state_dir` - Host directory for forge operations.
    /// * `protocol_version` - Protocol version for toolkit image selection.
    ///
    /// # Returns
    ///
    /// Container exit code.
    pub async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &Version,
    ) -> Result<i64> {
        log::debug!("Running forge with args: {:?}", args);
        let mut command = vec!["forge"];
        command.extend(args);

        self.run_command(&command, state_dir, protocol_version, &[])
            .await
    }

    /// Execute cast command in toolkit container.
    ///
    /// Cast typically doesn't require state directory access.
    ///
    /// # Arguments
    ///
    /// * `args` - Arguments to pass to cast.
    /// * `protocol_version` - Protocol version for toolkit image selection.
    ///
    /// # Returns
    ///
    /// Container exit code.
    pub async fn run_cast(&self, args: &[&str], protocol_version: &Version) -> Result<i64> {
        log::debug!("Running cast with args: {:?}", args);
        let mut command = vec!["cast"];
        command.extend(args);

        // Cast typically doesn't need state directory
        let temp_dir = std::env::temp_dir();
        log::debug!("Using temp directory for cast: {}", temp_dir.display());
        self.run_command(&command, &temp_dir, protocol_version, &[])
            .await
    }
}
