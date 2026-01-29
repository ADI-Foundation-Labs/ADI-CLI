//! Toolkit command execution via Docker containers.

use crate::config::ToolkitConfig;
use crate::error::Result;
use adi_docker::{ContainerConfig, ContainerManager, DockerClient};
use semver::Version;
use std::path::Path;

/// Genesis file name expected in state directory.
pub const GENESIS_FILENAME: &str = "genesis.json";

/// Path where genesis.json should be copied in the container.
pub const GENESIS_CONTAINER_PATH: &str = "/deps/zksync-era/etc/env/file_based/genesis.json";

/// Executes commands inside Docker toolkit containers.
///
/// Container lifecycle: create -> start -> stream output -> wait -> remove
///
/// # Example
///
/// ```rust,no_run
/// use adi_toolkit::ToolkitRunner;
/// use semver::Version;
/// use std::path::Path;
///
/// # async fn example() -> adi_toolkit::Result<()> {
/// let runner = ToolkitRunner::new().await?;
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
    config: ToolkitConfig,
}

impl ToolkitRunner {
    /// Create a new ToolkitRunner by connecting to Docker.
    ///
    /// Uses default toolkit configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if connection to Docker daemon fails.
    pub async fn new() -> Result<Self> {
        let client = DockerClient::new().await?;
        Ok(Self {
            client,
            config: ToolkitConfig::default(),
        })
    }

    /// Create a new ToolkitRunner with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Custom toolkit configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if connection to Docker daemon fails.
    pub async fn with_config(config: ToolkitConfig) -> Result<Self> {
        let client = DockerClient::new().await?;
        Ok(Self { client, config })
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
        let image_uri = image_ref.full_uri();

        log::info!("Using toolkit image: {}", image_uri);
        log::debug!(
            "Running command: {:?} (state_dir: {})",
            command,
            state_dir.display()
        );

        // Ensure image is available
        self.client.pull_image(&image_uri).await?;

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
        let exit_code = manager.run(&image_uri, &container_config).await?;

        log::debug!("Command completed with exit code: {}", exit_code);
        Ok(exit_code)
    }

    /// Execute zkstack CLI command in toolkit container.
    ///
    /// Automatically copies genesis.json from /workspace to the required location
    /// before running the zkstack command.
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
    /// # use adi_toolkit::ToolkitRunner;
    /// # use semver::Version;
    /// # use std::path::Path;
    /// # async fn example() -> adi_toolkit::Result<()> {
    /// # let runner = ToolkitRunner::new().await?;
    /// let version = Version::new(29, 0, 11);
    /// let state_dir = Path::new("/home/user/.adi_cli/state");
    ///
    /// // Run: zkstack ecosystem init
    /// let exit_code = runner.run_zkstack(
    ///     &["ecosystem", "init"],
    ///     state_dir,
    ///     &version,
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

        // Build zkstack command string
        let zkstack_cmd = format!("zkstack {}", args.join(" "));

        // Build shell command that copies genesis.json first, then runs zkstack
        // The genesis.json is expected in /workspace (mounted state_dir)
        let shell_cmd = format!(
            "cp /workspace/{} {} && {}",
            GENESIS_FILENAME, GENESIS_CONTAINER_PATH, zkstack_cmd
        );

        log::info!("Copying genesis.json to {}", GENESIS_CONTAINER_PATH);

        let command = vec!["sh", "-c", &shell_cmd];

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
