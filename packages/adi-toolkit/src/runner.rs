//! Toolkit command execution via Docker containers.

use crate::cleanup::cleanup_tmp_dir;
use crate::config::ToolkitConfig;
use crate::error::Result;
use adi_docker::{transform_url_for_container, ContainerConfig, ContainerManager, DockerClient};
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
/// let version = Version::new(30, 0, 2);
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

        // Check for crash reports on failure
        if exit_code != 0 {
            let tmp_dir = state_dir.join(".tmp");
            if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
                for entry in entries.flatten() {
                    let filename = entry.file_name();
                    let filename_str = filename.to_string_lossy();
                    if filename_str.starts_with("report-") && filename_str.ends_with(".toml") {
                        log::error!("Crash report available at: {}", entry.path().display());
                    }
                }
            }
        }

        // Clean up tmp directory (keep only *.md files)
        let tmp_dir = state_dir.join(".tmp");
        if tmp_dir.exists() {
            cleanup_tmp_dir(&tmp_dir);
        }

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
    /// let version = Version::new(30, 0, 2);
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

    /// Execute `zkstack ecosystem init` with foundry.toml permission fix.
    ///
    /// This method:
    /// 1. Copies genesis.json to the required container location
    /// 2. Fixes foundry.toml to allow read-write access to script-out directory
    /// 3. Runs `zkstack ecosystem init` with deployment flags
    ///
    /// # Arguments
    ///
    /// * `ecosystem_dir` - Host directory containing ecosystem state (e.g., ~/.adi_cli/state/adi_ecosystem).
    /// * `l1_rpc_url` - L1 RPC endpoint URL.
    /// * `gas_price_wei` - Optional gas price in wei (for non-local networks).
    /// * `protocol_version` - Protocol version for toolkit image selection.
    ///
    /// # Returns
    ///
    /// Container exit code (0 = success).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use adi_toolkit::ToolkitRunner;
    /// # use semver::Version;
    /// # use std::path::Path;
    /// # async fn example() -> adi_toolkit::Result<()> {
    /// # let runner = ToolkitRunner::new().await?;
    /// let version = Version::new(30, 0, 2);
    /// let ecosystem_dir = Path::new("/home/user/.adi_cli/state/adi_ecosystem");
    ///
    /// let exit_code = runner.run_zkstack_ecosystem_init(
    ///     ecosystem_dir,
    ///     "http://localhost:8545",
    ///     None,
    ///     &version,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_zkstack_ecosystem_init(
        &self,
        ecosystem_dir: &Path,
        l1_rpc_url: &str,
        gas_price_wei: Option<u128>,
        protocol_version: &Version,
    ) -> Result<i64> {
        log::debug!(
            "Running zkstack ecosystem init (ecosystem_dir: {}, rpc: {})",
            ecosystem_dir.display(),
            l1_rpc_url
        );

        // Build foundry.toml fix command (change read to read-write for script-out)
        let foundry_fix = r#"sed -i.bak 's/{ access = "read", path = "\.\.\/l1-contracts\/script-out\/" }/{ access = "read-write", path = "..\/l1-contracts\/script-out\/" }/' /deps/zksync-era/contracts/l1-contracts/foundry.toml"#;

        // Build zkstack command arguments
        // --dev bypasses interactive prompts (explicit flags override dev defaults)
        // --zksync-os selects VM option to avoid VM selection prompt
        let mut zkstack_args = String::from(
            "zkstack ecosystem init \
             --verbose \
             --zksync-os \
             --ignore-prerequisites \
             --observability false \
             --deploy-ecosystem true \
             --deploy-erc20 false \
             --deploy-paymaster false",
        );

        // Add gas price if provided
        if let Some(gas_price) = gas_price_wei {
            zkstack_args.push_str(&format!(" -a --with-gas-price -a {}", gas_price));
        }

        // Transform localhost URLs to host.docker.internal for macOS Docker containers
        let container_rpc_url = transform_url_for_container(l1_rpc_url);

        // Add L1 RPC URL
        zkstack_args.push_str(&format!(" --l1-rpc-url {}", container_rpc_url));

        // Build complete shell command: copy genesis + fix foundry + run zkstack
        // Use expect to auto-confirm interactive prompts (cliclack reads from terminal, not stdin)
        // Pattern matches only prompts like (Y/n), letting regular output flow through
        let shell_cmd = format!(
            r#"cp /workspace/{genesis} {genesis_path} && {foundry_fix} && \
expect -c 'set timeout 3600
log_user 1
spawn {zkstack}
while 1 {{
    expect {{
        eof {{ break }}
        timeout {{ break }}
        -re "\\(.*\\)\\s*$" {{ send "\r" }}
    }}
}}
catch wait result
exit [lindex $result 3]'"#,
            genesis = GENESIS_FILENAME,
            genesis_path = GENESIS_CONTAINER_PATH,
            foundry_fix = foundry_fix,
            zkstack = zkstack_args
        );

        log::info!("Fixing foundry.toml permissions and deploying ecosystem contracts");

        let command = vec!["sh", "-c", &shell_cmd];

        // CI=true skips forge telemetry prompt (detected as non-interactive)
        self.run_command(&command, ecosystem_dir, protocol_version, &[("CI", "true")])
            .await
    }
}
