//! Wrapper for the zkstack CLI.
//!
//! This module provides `ZkstackCli`, which wraps the zkstack command-line tool
//! for ZkSync ecosystem and chain management operations. The wrapper handles:
//!
//! - Command construction with proper arguments
//! - Async execution via `tokio::process::Command`
//! - Output capture and parsing
//! - Error handling with context
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::external::ZkstackCli;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let zkstack = ZkstackCli::new();
//!
//!     // Check version
//!     let version = zkstack.version().await?;
//!     println!("zkstack version: {}", version);
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};
use std::ffi::OsStr;
use std::process::Stdio;
use tokio::process::Command;

/// Output from a zkstack command execution.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Exit code from the command (0 = success).
    pub exit_code: i32,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
}

impl CommandOutput {
    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Wrapper for the zkstack CLI tool.
///
/// Provides typed methods for common zkstack operations including
/// ecosystem creation, initialization, and chain management.
///
/// # Example
///
/// ```rust,ignore
/// use adi_cli::external::ZkstackCli;
///
/// let zkstack = ZkstackCli::new();
///
/// // Execute a custom command
/// let output = zkstack.execute(&["ecosystem", "create", "--help"]).await?;
/// if output.success() {
///     println!("{}", output.stdout);
/// }
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub struct ZkstackCli {
    /// Path to the zkstack binary. Defaults to "zkstack".
    binary_path: String,
}

#[allow(dead_code)]
impl ZkstackCli {
    /// Creates a new ZkstackCli instance with the default binary path.
    pub fn new() -> Self {
        Self {
            binary_path: "zkstack".to_string(),
        }
    }

    /// Creates a new ZkstackCli instance with a custom binary path.
    ///
    /// # Arguments
    ///
    /// * `binary_path` - Path to the zkstack binary.
    pub fn with_binary_path(binary_path: impl Into<String>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }

    /// Returns the path to the zkstack binary.
    pub fn binary_path(&self) -> &str {
        &self.binary_path
    }

    /// Executes a zkstack command with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments to pass to zkstack.
    ///
    /// # Returns
    ///
    /// A `CommandOutput` containing the exit code, stdout, and stderr.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to spawn or execute.
    pub async fn execute<I, S>(&self, args: I) -> Result<CommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .wrap_err_with(|| format!("Failed to execute zkstack command: {}", self.binary_path))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Gets the zkstack version.
    ///
    /// # Returns
    ///
    /// The version string from `zkstack --version`.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or version cannot be parsed.
    pub async fn version(&self) -> Result<String> {
        let output = self.execute(["--version"]).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "zkstack --version failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        // Parse version from output (format: "zkstack X.Y.Z" or similar)
        Ok(output.stdout.trim().to_string())
    }

    /// Checks if zkstack is available and returns version info.
    ///
    /// # Returns
    ///
    /// `Ok(version)` if zkstack is available, or an error if not found.
    pub async fn check_available(&self) -> Result<String> {
        self.version()
            .await
            .wrap_err("zkstack CLI not found. Ensure it is installed and available in PATH.")
    }

    /// Creates a new ecosystem using `zkstack ecosystem create`.
    ///
    /// This command initializes a new ZkSync ecosystem with the specified
    /// configuration. It generates the ecosystem directory structure and
    /// configuration files.
    ///
    /// # Arguments
    ///
    /// * `config` - Ecosystem creation configuration
    ///
    /// # Returns
    ///
    /// A `CommandOutput` with the result of the ecosystem creation.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to execute.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zkstack = ZkstackCli::new();
    /// let config = EcosystemCreateConfig {
    ///     ecosystem_name: "my_ecosystem".to_string(),
    ///     chain_name: "my_chain".to_string(),
    ///     chain_id: 270,
    ///     prover_mode: "no-proofs".to_string(),
    ///     wallet_creation: "random".to_string(),
    ///     l1_network: "localhost".to_string(),
    ///     start_containers: false,
    /// };
    /// let output = zkstack.ecosystem_create(&config).await?;
    /// ```
    pub async fn ecosystem_create(&self, config: &EcosystemCreateConfig) -> Result<CommandOutput> {
        let mut args = vec![
            "ecosystem".to_string(),
            "create".to_string(),
            "--zksync-os".to_string(),
            "-v".to_string(),
            "--ecosystem-name".to_string(),
            config.ecosystem_name.clone(),
            "--chain-name".to_string(),
            config.chain_name.clone(),
            "--chain-id".to_string(),
            config.chain_id.to_string(),
            "--prover-mode".to_string(),
            config.prover_mode.clone(),
            "--wallet-creation".to_string(),
            config.wallet_creation.clone(),
            "--l1-network".to_string(),
            config.l1_network.clone(),
        ];

        if !config.start_containers {
            args.push("--start-containers".to_string());
            args.push("false".to_string());
        }

        // Add ignore prerequisites for automation
        args.push("--ignore-prerequisites".to_string());

        let output = self.execute(&args).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "zkstack ecosystem create failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        Ok(output)
    }
}

/// Configuration for ecosystem creation via zkstack CLI.
#[derive(Debug, Clone)]
pub struct EcosystemCreateConfig {
    /// Name of the ecosystem.
    pub ecosystem_name: String,
    /// Name of the initial chain.
    pub chain_name: String,
    /// Chain ID for the initial chain.
    pub chain_id: u64,
    /// Prover mode ("no-proofs" or "gpu").
    pub prover_mode: String,
    /// Wallet creation mode ("random" or "provided").
    pub wallet_creation: String,
    /// L1 network ("localhost", "sepolia", "mainnet").
    pub l1_network: String,
    /// Whether to start Docker containers after creation.
    pub start_containers: bool,
}

/// Configuration for ecosystem initialization (contract deployment) via zkstack CLI.
///
/// This is used with `zkstack ecosystem init` to deploy ecosystem contracts
/// to the settlement layer.
#[derive(Debug, Clone)]
pub struct EcosystemInitConfig {
    /// Path to the ecosystem directory.
    pub ecosystem_path: std::path::PathBuf,
    /// Settlement layer RPC URL.
    pub l1_rpc_url: String,
    /// Deployer private key.
    pub deployer_private_key: String,
    /// Governor private key.
    pub governor_private_key: String,
    /// Whether to skip verifying wallets have sufficient balance.
    pub skip_balance_check: bool,
    /// Whether to skip contract verification.
    pub no_verification: bool,
    /// Optional gas price in wei.
    pub gas_price: Option<u64>,
}

impl ZkstackCli {
    /// Initialize ecosystem contracts using `zkstack ecosystem init`.
    ///
    /// This command deploys ecosystem contracts to the settlement layer,
    /// including Bridgehub, Governance, Verifier, and other infrastructure contracts.
    ///
    /// # Arguments
    ///
    /// * `config` - Ecosystem initialization configuration
    ///
    /// # Returns
    ///
    /// A `CommandOutput` with the result of the ecosystem initialization.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to execute.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zkstack = ZkstackCli::new();
    /// let config = EcosystemInitConfig {
    ///     ecosystem_path: PathBuf::from("/path/to/ecosystem"),
    ///     l1_rpc_url: "http://localhost:8545".to_string(),
    ///     deployer_private_key: "0x...".to_string(),
    ///     governor_private_key: "0x...".to_string(),
    ///     skip_balance_check: false,
    ///     no_verification: true,
    ///     gas_price: None,
    /// };
    /// let output = zkstack.ecosystem_init(&config).await?;
    /// ```
    pub async fn ecosystem_init(&self, config: &EcosystemInitConfig) -> Result<CommandOutput> {
        let mut args = vec![
            "ecosystem".to_string(),
            "init".to_string(),
            "-v".to_string(),
            "--ecosystem-path".to_string(),
            config.ecosystem_path.to_string_lossy().to_string(),
            "--l1-rpc-url".to_string(),
            config.l1_rpc_url.clone(),
            "--deployer-private-key".to_string(),
            config.deployer_private_key.clone(),
            "--governor-private-key".to_string(),
            config.governor_private_key.clone(),
        ];

        if config.skip_balance_check {
            args.push("--skip-balance-check".to_string());
        }

        if config.no_verification {
            args.push("--no-verification".to_string());
        }

        if let Some(gas_price) = config.gas_price {
            args.push("--gas-price".to_string());
            args.push(gas_price.to_string());
        }

        // Add ignore prerequisites for automation
        args.push("--ignore-prerequisites".to_string());

        let output = self.execute(&args).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "zkstack ecosystem init failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        Ok(output)
    }
}

impl Default for ZkstackCli {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for chain initialization (contract deployment) via zkstack CLI.
///
/// This is used with `zkstack chain init` to deploy chain contracts
/// to the settlement layer and register with Bridgehub.
#[derive(Debug, Clone)]
pub struct ChainInitConfig {
    /// Path to the ecosystem directory.
    pub ecosystem_path: std::path::PathBuf,
    /// Name of the chain to initialize.
    pub chain_name: String,
    /// Settlement layer RPC URL.
    pub l1_rpc_url: String,
    /// Deployer private key.
    pub deployer_private_key: String,
    /// Governor private key.
    pub governor_private_key: String,
    /// Whether to skip verifying wallets have sufficient balance.
    pub skip_balance_check: bool,
    /// Whether to skip contract verification.
    pub no_verification: bool,
    /// Optional gas price in wei.
    pub gas_price: Option<u64>,
}

impl ZkstackCli {
    /// Initialize chain contracts using `zkstack chain init`.
    ///
    /// This command deploys chain contracts to the settlement layer and
    /// registers the chain with Bridgehub. The chain must already be created
    /// via `adi init chain` before calling this.
    ///
    /// # Arguments
    ///
    /// * `config` - Chain initialization configuration
    ///
    /// # Returns
    ///
    /// A `CommandOutput` with the result of the chain initialization.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to execute.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zkstack = ZkstackCli::new();
    /// let config = ChainInitConfig {
    ///     ecosystem_path: PathBuf::from("/path/to/ecosystem"),
    ///     chain_name: "my_chain".to_string(),
    ///     l1_rpc_url: "http://localhost:8545".to_string(),
    ///     deployer_private_key: "0x...".to_string(),
    ///     governor_private_key: "0x...".to_string(),
    ///     skip_balance_check: false,
    ///     no_verification: true,
    ///     gas_price: None,
    /// };
    /// let output = zkstack.chain_init(&config).await?;
    /// ```
    pub async fn chain_init(&self, config: &ChainInitConfig) -> Result<CommandOutput> {
        let mut args = vec![
            "chain".to_string(),
            "init".to_string(),
            "-v".to_string(),
            "--ecosystem-path".to_string(),
            config.ecosystem_path.to_string_lossy().to_string(),
            "--chain-name".to_string(),
            config.chain_name.clone(),
            "--l1-rpc-url".to_string(),
            config.l1_rpc_url.clone(),
            "--deployer-private-key".to_string(),
            config.deployer_private_key.clone(),
            "--governor-private-key".to_string(),
            config.governor_private_key.clone(),
        ];

        if config.skip_balance_check {
            args.push("--skip-balance-check".to_string());
        }

        if config.no_verification {
            args.push("--no-verification".to_string());
        }

        if let Some(gas_price) = config.gas_price {
            args.push("--gas-price".to_string());
            args.push(gas_price.to_string());
        }

        // Add ignore prerequisites for automation
        args.push("--ignore-prerequisites".to_string());

        let output = self.execute(&args).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "zkstack chain init failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        Ok(output)
    }
}

/// Configuration for generating chain upgrade calldata via zkstack CLI.
///
/// This is used with `zkstack dev generate-chain-upgrade` to prepare upgrade
/// calldata for a specific chain after the ecosystem has been upgraded.
// Note: Currently unused as upgrade commands use forge scripts; will be used when
// integrating with zkstack in Docker containers (Phase 9)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ChainUpgradeConfig {
    /// Path to the ecosystem directory.
    pub ecosystem_path: std::path::PathBuf,
    /// Name of the chain to upgrade.
    pub chain_name: String,
    /// Target upgrade version (e.g., "v30").
    pub upgrade_version: String,
    /// Settlement layer RPC URL.
    pub l1_rpc_url: String,
    /// Chain RPC URL (L2).
    pub l2_rpc_url: Option<String>,
}

#[allow(dead_code)]
impl ZkstackCli {
    /// Generate chain upgrade calldata using `zkstack dev generate-chain-upgrade`.
    ///
    /// This command generates upgrade calldata for a specific chain after the
    /// ecosystem has been upgraded to a new protocol version. The chain upgrade
    /// must match the ecosystem's target version.
    ///
    /// # Arguments
    ///
    /// * `config` - Chain upgrade configuration
    ///
    /// # Returns
    ///
    /// A `CommandOutput` with the result of the chain upgrade generation.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to execute.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zkstack = ZkstackCli::new();
    /// let config = ChainUpgradeConfig {
    ///     ecosystem_path: PathBuf::from("/path/to/ecosystem"),
    ///     chain_name: "my_chain".to_string(),
    ///     upgrade_version: "v30".to_string(),
    ///     l1_rpc_url: "http://localhost:8545".to_string(),
    ///     l2_rpc_url: Some("http://localhost:3050".to_string()),
    /// };
    /// let output = zkstack.generate_chain_upgrade(&config).await?;
    /// ```
    pub async fn generate_chain_upgrade(
        &self,
        config: &ChainUpgradeConfig,
    ) -> Result<CommandOutput> {
        let mut args = vec![
            "dev".to_string(),
            "generate-chain-upgrade".to_string(),
            "-v".to_string(),
            "--ecosystem-path".to_string(),
            config.ecosystem_path.to_string_lossy().to_string(),
            "--chain-name".to_string(),
            config.chain_name.clone(),
            "--upgrade-version".to_string(),
            config.upgrade_version.clone(),
            "--l1-rpc-url".to_string(),
            config.l1_rpc_url.clone(),
        ];

        if let Some(ref l2_rpc_url) = config.l2_rpc_url {
            args.push("--l2-rpc-url".to_string());
            args.push(l2_rpc_url.clone());
        }

        // Add ignore prerequisites for automation
        args.push("--ignore-prerequisites".to_string());

        let output = self.execute(&args).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "zkstack dev generate-chain-upgrade failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_output_success() {
        let output = CommandOutput {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };
        assert!(output.success());
    }

    #[test]
    fn test_command_output_failure() {
        let output = CommandOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!output.success());
    }

    #[test]
    fn test_zkstack_cli_default_path() {
        let zkstack = ZkstackCli::new();
        assert_eq!(zkstack.binary_path(), "zkstack");
    }

    #[test]
    fn test_zkstack_cli_custom_path() {
        let zkstack = ZkstackCli::with_binary_path("/custom/path/zkstack");
        assert_eq!(zkstack.binary_path(), "/custom/path/zkstack");
    }
}
