//! Toolkit command execution via Docker containers.

use crate::cleanup::cleanup_tmp_dir;
use crate::config::ToolkitConfig;
use crate::error::Result;
use adi_docker::{transform_url_for_container, ContainerConfig, ContainerManager, DockerClient};
use adi_types::{LogCrateLogger, Logger};
use colored::Colorize;
use semver::Version;
use std::path::Path;
use std::sync::Arc;

/// Genesis file name expected in state directory.
pub const GENESIS_FILENAME: &str = "genesis.json";

/// Path where genesis.json should be copied in the container.
pub const GENESIS_CONTAINER_PATH: &str = "/deps/zksync-era/etc/env/file_based/genesis.json";

/// Executes commands inside Docker toolkit containers.
///
/// Container lifecycle: create -> start -> stream output -> wait -> remove
pub struct ToolkitRunner {
    client: DockerClient,
    config: ToolkitConfig,
    logger: Arc<dyn Logger>,
}

impl ToolkitRunner {
    /// Create a new ToolkitRunner by connecting to Docker.
    pub async fn new() -> Result<Self> {
        Self::with_logger(Arc::new(LogCrateLogger)).await
    }

    /// Create a new ToolkitRunner with custom logger.
    pub async fn with_logger(logger: Arc<dyn Logger>) -> Result<Self> {
        let client = DockerClient::with_logger(Arc::clone(&logger)).await?;
        Ok(Self {
            client,
            config: ToolkitConfig::default(),
            logger,
        })
    }

    /// Create a new ToolkitRunner with custom configuration.
    pub async fn with_config(config: ToolkitConfig) -> Result<Self> {
        Self::with_config_and_logger(config, Arc::new(LogCrateLogger)).await
    }

    /// Create a new ToolkitRunner with custom configuration and logger.
    pub async fn with_config_and_logger(
        config: ToolkitConfig,
        logger: Arc<dyn Logger>,
    ) -> Result<Self> {
        let client = DockerClient::with_logger(Arc::clone(&logger)).await?;
        Ok(Self {
            client,
            config,
            logger,
        })
    }

    /// Execute a generic command in the toolkit container.
    /// Logs are saved to state_dir/logs/.
    pub async fn run_command(
        &self,
        command: &[&str],
        state_dir: &Path,
        protocol_version: &Version,
        env_vars: &[(&str, &str)],
        log_command: &str,
        log_label: &str,
    ) -> Result<i64> {
        self.run_command_internal(
            command,
            state_dir,
            state_dir,
            protocol_version,
            env_vars,
            log_command,
            log_label,
            false,
        )
        .await
    }

    /// Execute a command with separate working directory and log directory.
    /// Use this when state_dir is a temp directory but logs should go elsewhere.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_command_with_log_dir(
        &self,
        command: &[&str],
        state_dir: &Path,
        log_dir: &Path,
        protocol_version: &Version,
        env_vars: &[(&str, &str)],
        log_command: &str,
        log_label: &str,
    ) -> Result<i64> {
        self.run_command_internal(
            command,
            state_dir,
            log_dir,
            protocol_version,
            env_vars,
            log_command,
            log_label,
            false,
        )
        .await
    }

    /// Internal command runner with quiet mode support.
    #[allow(clippy::too_many_arguments)]
    async fn run_command_internal(
        &self,
        command: &[&str],
        state_dir: &Path,
        log_dir: &Path,
        protocol_version: &Version,
        env_vars: &[(&str, &str)],
        log_command: &str,
        log_label: &str,
        quiet: bool,
    ) -> Result<i64> {
        let image_ref = self
            .config
            .image_reference(protocol_version, self.logger.as_ref());
        let image_uri = image_ref.full_uri();

        if !quiet {
            self.logger
                .info(&format!("Using toolkit image: {}", image_uri.green()));
        }
        self.logger.debug(&format!(
            "Running command: {:?} (state_dir: {}, log_dir: {})",
            command,
            state_dir.display(),
            log_dir.display()
        ));

        self.client.pull_image(&image_uri).await?;

        let container_config = ContainerConfig {
            state_dir: state_dir.to_path_buf(),
            command: command.iter().map(|s| (*s).to_string()).collect(),
            env_vars: env_vars
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            timeout_seconds: self.config.timeout_seconds,
            log_dir: log_dir.to_path_buf(),
            log_command: log_command.to_string(),
            log_label: log_label.to_string(),
            quiet,
            ..Default::default()
        };

        self.logger.debug(&format!(
            "Container config: working_dir={}, timeout={}s",
            container_config.working_dir, container_config.timeout_seconds
        ));

        let manager =
            ContainerManager::with_logger(self.client.inner().clone(), Arc::clone(&self.logger));
        let result = manager.run(&image_uri, &container_config).await;

        // Always clean up tmp directory (keep only *.md files), even on error/interrupt
        let tmp_dir = state_dir.join(".tmp");
        if tmp_dir.exists() {
            // Check for crash reports before cleanup (only on failure)
            if let Ok(ref exit_code) = result {
                if *exit_code != 0 {
                    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
                        for entry in entries.flatten() {
                            let filename = entry.file_name();
                            let filename_str = filename.to_string_lossy();
                            if filename_str.starts_with("report-")
                                && filename_str.ends_with(".toml")
                            {
                                self.logger.error(&format!(
                                    "Crash report available at: {}",
                                    entry.path().display()
                                ));
                            }
                        }
                    }
                }
            }
            cleanup_tmp_dir(&tmp_dir, self.logger.as_ref());
        }

        let exit_code = result?;
        self.logger
            .debug(&format!("Command completed with exit code: {}", exit_code));

        Ok(exit_code)
    }

    /// Execute zkstack CLI command in toolkit container.
    ///
    /// # Arguments
    /// * `args` - Arguments to pass to zkstack
    /// * `state_dir` - Container working directory (mounted as /workspace)
    /// * `log_dir` - Directory for saving logs (use state_dir if same, or real state dir if using temp)
    /// * `protocol_version` - Protocol version for toolkit image selection
    pub async fn run_zkstack(
        &self,
        args: &[&str],
        state_dir: &Path,
        log_dir: &Path,
        protocol_version: &Version,
    ) -> Result<i64> {
        self.logger
            .debug(&format!("Running zkstack with args: {:?}", args));

        let zkstack_cmd = format!("zkstack {}", args.join(" "));
        let shell_cmd = format!(
            "cp /workspace/{} {} && {}",
            GENESIS_FILENAME, GENESIS_CONTAINER_PATH, zkstack_cmd
        );

        self.logger.debug(&format!(
            "Copying genesis.json to {}",
            GENESIS_CONTAINER_PATH
        ));

        let command = vec!["sh", "-c", &shell_cmd];
        let label = format!("Running zkstack {}...", args.first().unwrap_or(&""));

        self.run_command_with_log_dir(
            &command,
            state_dir,
            log_dir,
            protocol_version,
            &[],
            "zkstack",
            &label,
        )
        .await
    }

    /// Execute forge command in toolkit container.
    pub async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &Version,
    ) -> Result<i64> {
        self.logger
            .debug(&format!("Running forge with args: {:?}", args));
        let mut command = vec!["forge"];
        command.extend(args);

        self.run_command(
            &command,
            state_dir,
            protocol_version,
            &[],
            "forge",
            "Running forge...",
        )
        .await
    }

    /// Execute cast command in toolkit container.
    pub async fn run_cast(&self, args: &[&str], protocol_version: &Version) -> Result<i64> {
        self.logger
            .debug(&format!("Running cast with args: {:?}", args));
        let mut command = vec!["cast"];
        command.extend(args);

        let temp_dir = std::env::temp_dir();
        self.logger.debug(&format!(
            "Using temp directory for cast: {}",
            temp_dir.display()
        ));

        self.run_command(
            &command,
            &temp_dir,
            protocol_version,
            &[],
            "cast",
            "Running cast...",
        )
        .await
    }

    /// Execute `forge verify-contract` in toolkit container.
    ///
    /// # Arguments
    /// * `address` - Contract address to verify
    /// * `contract_path` - Path to contract in format "path/to/Contract.sol:ContractName"
    /// * `chain_id` - Chain ID for the network
    /// * `verifier_url` - Block explorer API URL
    /// * `verifier` - Verifier type ("blockscout", "etherscan", "sourcify", etc.)
    /// * `api_key` - Block explorer API key (optional for public explorers like Blockscout)
    /// * `constructor_args` - Optional constructor arguments (hex-encoded)
    /// * `protocol_version` - Protocol version for toolkit image selection
    /// * `log_dir` - Directory for saving log files
    /// * `root_path` - Root path for contract sources (e.g., /deps/era-contracts/l1-contracts)
    #[allow(clippy::too_many_arguments)]
    pub async fn run_forge_verify(
        &self,
        address: &str,
        contract_path: &str,
        chain_id: u64,
        verifier_url: &str,
        verifier: &str,
        api_key: Option<&str>,
        constructor_args: Option<&str>,
        protocol_version: &Version,
        log_dir: &Path,
        root_path: &str,
    ) -> Result<i64> {
        self.logger.debug(&format!(
            "Running forge verify-contract for {} (contract: {}, root: {})",
            address, contract_path, root_path
        ));

        // Build the forge verify-contract command
        // Forge verify doesn't use src setting, so we prepend contracts/ to the path
        // Exception: lib/ paths (e.g., OpenZeppelin contracts) are at project root level
        let full_contract_path = if contract_path.starts_with("lib/") {
            contract_path.to_string()
        } else {
            format!("contracts/{}", contract_path)
        };
        let chain_id_str = chain_id.to_string();
        let mut args: Vec<&str> = vec![
            "forge",
            "verify-contract",
            address,
            &full_contract_path,
            "--chain-id",
            &chain_id_str,
            "--verifier",
            verifier,
            "--verifier-url",
            verifier_url,
            "--root",
            root_path,
            "--compiler-version",
            "0.8.28",
            "--evm-version",
            "cancun",
            "--num-of-optimizations",
            "28000",
            "--watch", // Wait for verification to complete (not just submission accepted)
        ];

        if let Some(key) = api_key {
            args.push("--etherscan-api-key");
            args.push(key);
        }

        if let Some(ctor_args) = constructor_args {
            args.push("--constructor-args");
            args.push(ctor_args);
        }

        let temp_dir = std::env::temp_dir();

        // Run in quiet mode - output is suppressed during batch verification
        // (progress bar shows status, logs are saved to file)
        self.run_command_internal(
            &args,
            &temp_dir,
            log_dir,
            protocol_version,
            &[("CI", "true")], // Suppress interactive prompts like telemetry
            "forge-verify",
            &format!("Verifying {}...", address),
            true, // quiet mode
        )
        .await
    }

    /// Execute `zkstack ecosystem init` with foundry.toml permission fix.
    ///
    /// # Arguments
    ///
    /// * `ecosystem_dir` - Path to the ecosystem directory
    /// * `l1_rpc_url` - Settlement layer RPC URL
    /// * `gas_price_wei` - Optional gas price in wei (uses default if None)
    /// * `protocol_version` - Protocol version for toolkit image selection
    /// * `deploy_ecosystem` - Whether to deploy ecosystem contracts:
    ///   - `true` for first-time deployment (creates Bridgehub, Governance, etc.)
    ///   - `false` when adding a chain to an existing ecosystem
    /// * `chain_name` - Name of the chain to initialize/deploy
    /// * `verification` - Contract verification options
    #[allow(clippy::too_many_arguments)]
    pub async fn run_zkstack_ecosystem_init(
        &self,
        ecosystem_dir: &Path,
        l1_rpc_url: &str,
        gas_price_wei: Option<u128>,
        protocol_version: &Version,
        deploy_ecosystem: bool,
        chain_name: &str,
    ) -> Result<i64> {
        self.logger.debug(&format!(
            "Running zkstack ecosystem init (ecosystem_dir: {}, rpc: {}, deploy_ecosystem: {})",
            ecosystem_dir.display(),
            l1_rpc_url,
            deploy_ecosystem
        ));

        let foundry_fix = r#"sed -i.bak 's/{ access = "read", path = "\.\.\/l1-contracts\/script-out\/" }/{ access = "read-write", path = "..\/l1-contracts\/script-out\/" }/' /deps/zksync-era/contracts/l1-contracts/foundry.toml"#;

        let mut zkstack_args = format!(
            "zkstack ecosystem init \
             --verbose \
             --zksync-os \
             --ignore-prerequisites \
             --observability false \
             --deploy-ecosystem {} \
             --deploy-erc20 false \
             --deploy-paymaster false \
             --chain {}",
            deploy_ecosystem, chain_name
        );

        // When not deploying ecosystem, point to existing contracts config
        if !deploy_ecosystem {
            // In container, ecosystem_dir is mounted as /workspace
            // zkstack expects path to ecosystem root where configs/contracts.yaml exists
            zkstack_args.push_str(" --ecosystem-contracts-path /workspace/configs/contracts.yaml");
        }

        if let Some(gas_price) = gas_price_wei {
            zkstack_args.push_str(&format!(" -a --with-gas-price -a {}", gas_price));
        }

        let container_rpc_url = transform_url_for_container(l1_rpc_url, self.logger.as_ref());
        zkstack_args.push_str(&format!(" --l1-rpc-url {}", container_rpc_url));

        let shell_cmd = format!(
            r#"cp /workspace/{genesis} {genesis_path} && {foundry_fix} && \
stdbuf -oL expect -c 'set timeout 3600
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

        let deploy_msg = if deploy_ecosystem {
            "deploying ecosystem + chain contracts"
        } else {
            "deploying chain contracts only"
        };
        self.logger.debug(&format!(
            "Fixing foundry.toml permissions and {}",
            deploy_msg
        ));

        let shell_command = vec!["sh", "-c", &shell_cmd];

        let label = if deploy_ecosystem {
            "Deploying ecosystem contracts..."
        } else {
            "Deploying chain contracts..."
        };

        self.run_command(
            &shell_command,
            ecosystem_dir,
            protocol_version,
            &[("CI", "true")],
            "deploy",
            label,
        )
        .await
    }
}
