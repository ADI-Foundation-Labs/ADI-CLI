//! Toolkit command execution via Docker containers.

use crate::cleanup::cleanup_tmp_dir;
use crate::config::ToolkitConfig;
use crate::error::Result;
use adi_docker::{transform_url_for_container, ContainerConfig, ContainerManager, DockerClient};
use adi_types::{LogCrateLogger, Logger};
use console::style;
use semver::Version;
use std::path::Path;
use std::sync::Arc;

/// Get current user UID:GID for container user mapping (Unix only).
#[cfg(unix)]
fn get_current_user() -> Option<String> {
    use std::process::Command;

    let uid = Command::new("id").arg("-u").output().ok()?;
    let gid = Command::new("id").arg("-g").output().ok()?;

    let uid = String::from_utf8_lossy(&uid.stdout).trim().to_string();
    let gid = String::from_utf8_lossy(&gid.stdout).trim().to_string();

    if uid.is_empty() || gid.is_empty() {
        return None;
    }

    Some(format!("{}:{}", uid, gid))
}

/// Get current user UID:GID for container user mapping (non-Unix stub).
#[cfg(not(unix))]
fn get_current_user() -> Option<String> {
    None
}

/// Parameters for running a command in a toolkit container.
struct RunCommandParams<'a> {
    /// Command and arguments to execute.
    command: &'a [&'a str],
    /// Working directory mounted into the container.
    state_dir: &'a Path,
    /// Directory for saving log files.
    log_dir: &'a Path,
    /// Protocol version for toolkit image selection.
    protocol_version: &'a Version,
    /// Environment variables to pass to the container.
    env_vars: &'a [(&'a str, &'a str)],
    /// Command name for log file naming.
    log_command: &'a str,
    /// Label for progress display.
    log_label: &'a str,
    /// Whether to suppress output.
    quiet: bool,
}

/// Parameters for `forge verify-contract`.
pub struct ForgeVerifyParams<'a> {
    /// Contract address to verify.
    pub address: &'a str,
    /// Path to contract in format "path/to/Contract.sol:ContractName".
    pub contract_path: &'a str,
    /// Chain ID for the network.
    pub chain_id: u64,
    /// Block explorer API URL.
    pub verifier_url: &'a str,
    /// Verifier type ("blockscout", "etherscan", "sourcify", etc.).
    pub verifier: &'a str,
    /// Block explorer API key (optional for public explorers like Blockscout).
    pub api_key: Option<&'a str>,
    /// Optional constructor arguments (hex-encoded).
    pub constructor_args: Option<&'a str>,
    /// Protocol version for toolkit image selection.
    pub protocol_version: &'a Version,
    /// Directory for saving log files.
    pub log_dir: &'a Path,
    /// Root path for contract sources (e.g., /deps/era-contracts/l1-contracts).
    pub root_path: &'a str,
}

/// Parameters for `zkstack ecosystem init`.
pub struct EcosystemInitParams<'a> {
    /// Path to the ecosystem directory.
    pub ecosystem_dir: &'a Path,
    /// Settlement layer RPC URL.
    pub l1_rpc_url: &'a str,
    /// Optional gas price in wei (uses default if None).
    pub gas_price_wei: Option<u128>,
    /// Protocol version for toolkit image selection.
    pub protocol_version: &'a Version,
    /// Whether to deploy ecosystem contracts.
    pub deploy_ecosystem: bool,
    /// Name of the chain to initialize/deploy.
    pub chain_name: &'a str,
}

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
        self.run_command_internal(RunCommandParams {
            command,
            state_dir,
            log_dir: state_dir,
            protocol_version,
            env_vars,
            log_command,
            log_label,
            quiet: false,
        })
        .await
    }

    /// Internal command runner with quiet mode support.
    async fn run_command_internal(&self, params: RunCommandParams<'_>) -> Result<i64> {
        let image_ref = self
            .config
            .image_reference(params.protocol_version, self.logger.as_ref());
        let image_uri = image_ref.full_uri();

        if !params.quiet {
            self.logger.info(&format!(
                "Using toolkit image: {}",
                style(&image_uri).green()
            ));
        }
        self.logger.debug(&format!(
            "Running command: {:?} (state_dir: {}, log_dir: {})",
            params.command,
            params.state_dir.display(),
            params.log_dir.display()
        ));

        self.client.pull_image(&image_uri).await?;

        // Always include CI=true to suppress interactive prompts (e.g., telemetry)
        let mut all_env_vars: Vec<(String, String)> = vec![("CI".to_string(), "true".to_string())];
        all_env_vars.extend(
            params
                .env_vars
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
        );

        let container_config = ContainerConfig {
            state_dir: params.state_dir.to_path_buf(),
            command: params.command.iter().map(|s| (*s).to_string()).collect(),
            env_vars: all_env_vars,
            timeout_seconds: self.config.timeout_seconds,
            log_dir: params.log_dir.to_path_buf(),
            log_command: params.log_command.to_string(),
            log_label: params.log_label.to_string(),
            quiet: params.quiet,
            user: get_current_user(),
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
        let tmp_dir = params.state_dir.join(".tmp");
        if tmp_dir.exists() {
            // Check for crash reports before cleanup (only on failure)
            if matches!(&result, Ok(code) if *code != 0) {
                self.log_crash_reports(&tmp_dir);
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

        let mut command = vec!["zkstack"];
        command.extend(args);
        let label = format!("Running zkstack {}...", args.first().unwrap_or(&""));

        self.run_command_internal(RunCommandParams {
            command: &command,
            state_dir,
            log_dir,
            protocol_version,
            env_vars: &[],
            log_command: "zkstack",
            log_label: &label,
            quiet: false,
        })
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
    pub async fn run_forge_verify(&self, params: &ForgeVerifyParams<'_>) -> Result<i64> {
        self.logger.debug(&format!(
            "Running forge verify-contract for {} (contract: {}, root: {})",
            params.address, params.contract_path, params.root_path
        ));

        // Build the forge verify-contract command
        // Forge verify doesn't use src setting, so we prepend contracts/ to the path
        // Exception: lib/ paths (e.g., OpenZeppelin contracts) are at project root level
        let full_contract_path = if params.contract_path.starts_with("lib/") {
            params.contract_path.to_string()
        } else {
            format!("contracts/{}", params.contract_path)
        };
        let chain_id_str = params.chain_id.to_string();
        let mut args: Vec<&str> = vec![
            "forge",
            "verify-contract",
            params.address,
            &full_contract_path,
            "--chain-id",
            &chain_id_str,
            "--verifier",
            params.verifier,
            "--verifier-url",
            params.verifier_url,
            "--root",
            params.root_path,
            "--compiler-version",
            "0.8.28",
            "--evm-version",
            "cancun",
            "--num-of-optimizations",
            "28000",
            "--watch", // Wait for verification to complete (not just submission accepted)
        ];

        if let Some(key) = params.api_key {
            args.push("--etherscan-api-key");
            args.push(key);
        }

        if let Some(ctor_args) = params.constructor_args {
            args.push("--constructor-args");
            args.push(ctor_args);
        }

        let temp_dir = std::env::temp_dir();

        // Run in quiet mode - output is suppressed during batch verification
        // (progress bar shows status, logs are saved to file)
        self.run_command_internal(RunCommandParams {
            command: &args,
            state_dir: &temp_dir,
            log_dir: params.log_dir,
            protocol_version: params.protocol_version,
            env_vars: &[],
            log_command: "forge-verify",
            log_label: &format!("Verifying {}...", params.address),
            quiet: true,
        })
        .await
    }

    /// Execute `zkstack ecosystem init` with foundry.toml permission fix.
    pub async fn run_zkstack_ecosystem_init(
        &self,
        params: &EcosystemInitParams<'_>,
    ) -> Result<i64> {
        self.logger.debug(&format!(
            "Running zkstack ecosystem init (ecosystem_dir: {}, rpc: {}, deploy_ecosystem: {})",
            params.ecosystem_dir.display(),
            params.l1_rpc_url,
            params.deploy_ecosystem
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
            params.deploy_ecosystem, params.chain_name
        );

        // When not deploying ecosystem, point to existing contracts config
        if !params.deploy_ecosystem {
            // In container, ecosystem_dir is mounted as /workspace
            // zkstack expects path to ecosystem root where configs/contracts.yaml exists
            zkstack_args.push_str(" --ecosystem-contracts-path /workspace/configs/contracts.yaml");
        }

        if let Some(gas_price) = params.gas_price_wei {
            zkstack_args.push_str(&format!(" -a --with-gas-price -a {}", gas_price));
        }

        let container_rpc_url =
            transform_url_for_container(params.l1_rpc_url, self.logger.as_ref());
        zkstack_args.push_str(&format!(" --l1-rpc-url {}", container_rpc_url));

        let shell_cmd = format!(
            r#"{foundry_fix} && \
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
            foundry_fix = foundry_fix,
            zkstack = zkstack_args
        );

        let deploy_msg = if params.deploy_ecosystem {
            "deploying ecosystem + chain contracts"
        } else {
            "deploying chain contracts only"
        };
        self.logger.debug(&format!(
            "Fixing foundry.toml permissions and {}",
            deploy_msg
        ));

        let shell_command = vec!["sh", "-c", &shell_cmd];

        let label = if params.deploy_ecosystem {
            "Deploying ecosystem contracts..."
        } else {
            "Deploying chain contracts..."
        };

        self.run_command(
            &shell_command,
            params.ecosystem_dir,
            params.protocol_version,
            &[],
            "deploy",
            label,
        )
        .await
    }

    /// Log any crash reports found in the given directory.
    fn log_crash_reports(&self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            if filename_str.starts_with("report-") && filename_str.ends_with(".toml") {
                self.logger.error(&format!(
                    "Crash report available at: {}",
                    entry.path().display()
                ));
            }
        }
    }
}
