//! Wrapper for the forge CLI (foundry-zksync).
//!
//! This module provides `ForgeCli`, which wraps the forge command-line tool
//! for Solidity smart contract compilation, deployment, and script execution.
//! The wrapper handles:
//!
//! - Script execution with proper arguments
//! - Build and compilation operations
//! - Async execution via `tokio::process::Command`
//! - Output capture and parsing
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::external::ForgeCli;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let forge = ForgeCli::new();
//!
//!     // Check version
//!     let version = forge.version().await?;
//!     println!("forge version: {}", version);
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};
use alloy_primitives::{Address, Bytes};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Output from a forge command execution.
#[derive(Debug, Clone)]
pub struct ForgeOutput {
    /// Exit code from the command (0 = success).
    pub exit_code: i32,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
}

impl ForgeOutput {
    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Wrapper for the forge CLI tool (foundry-zksync).
///
/// Provides typed methods for common forge operations including
/// script execution, contract building, and deployment.
///
/// # Example
///
/// ```rust,ignore
/// use adi_cli::external::ForgeCli;
/// use std::path::Path;
///
/// let forge = ForgeCli::new();
///
/// // Run a deployment script
/// let output = forge.script(
///     "script/Deploy.s.sol:DeployScript",
///     "http://localhost:8545",
///     Some("0xprivate_key"),
///     true, // broadcast
///     &["--ffi"],
/// ).await?;
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub struct ForgeCli {
    /// Path to the forge binary. Defaults to "forge".
    binary_path: String,
}

#[allow(dead_code)]
impl ForgeCli {
    /// Creates a new ForgeCli instance with the default binary path.
    pub fn new() -> Self {
        Self {
            binary_path: "forge".to_string(),
        }
    }

    /// Creates a new ForgeCli instance with a custom binary path.
    ///
    /// # Arguments
    ///
    /// * `binary_path` - Path to the forge binary.
    pub fn with_binary_path(binary_path: impl Into<String>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }

    /// Returns the path to the forge binary.
    pub fn binary_path(&self) -> &str {
        &self.binary_path
    }

    /// Executes a forge command with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments to pass to forge.
    ///
    /// # Returns
    ///
    /// A `ForgeOutput` containing the exit code, stdout, and stderr.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to spawn or execute.
    pub async fn execute<I, S>(&self, args: I) -> Result<ForgeOutput>
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
            .wrap_err_with(|| format!("Failed to execute forge command: {}", self.binary_path))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ForgeOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Executes a forge command in a specific working directory.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments to pass to forge.
    /// * `working_dir` - The directory to run the command in.
    ///
    /// # Returns
    ///
    /// A `ForgeOutput` containing the exit code, stdout, and stderr.
    pub async fn execute_in_dir<I, S>(&self, args: I, working_dir: &Path) -> Result<ForgeOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.binary_path)
            .args(args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to execute forge command in {}: {}",
                    working_dir.display(),
                    self.binary_path
                )
            })?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ForgeOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Runs a forge script.
    ///
    /// # Arguments
    ///
    /// * `script_path` - Path to the script (e.g., "script/Deploy.s.sol:DeployScript").
    /// * `rpc_url` - RPC endpoint URL.
    /// * `private_key` - Optional private key for signing transactions.
    /// * `broadcast` - Whether to broadcast transactions to the network.
    /// * `extra_args` - Additional arguments to pass to the script command.
    ///
    /// # Returns
    ///
    /// A `ForgeOutput` containing the execution results.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let output = forge.script(
    ///     "script/Deploy.s.sol:DeployScript",
    ///     "http://localhost:8545",
    ///     Some("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
    ///     true,
    ///     &["--ffi"],
    /// ).await?;
    /// ```
    pub async fn script(
        &self,
        script_path: &str,
        rpc_url: &str,
        private_key: Option<&str>,
        broadcast: bool,
        extra_args: &[&str],
    ) -> Result<ForgeOutput> {
        let mut args = vec!["script", script_path, "--rpc-url", rpc_url];

        if let Some(pk) = private_key {
            args.push("--private-key");
            args.push(pk);
        }

        if broadcast {
            args.push("--broadcast");
        }

        args.extend(extra_args.iter().copied());

        self.execute(args).await
    }

    /// Runs a forge script in a specific working directory.
    ///
    /// This is useful when the script requires access to local files
    /// or has relative path dependencies.
    ///
    /// # Arguments
    ///
    /// * `script_path` - Path to the script.
    /// * `rpc_url` - RPC endpoint URL.
    /// * `private_key` - Optional private key for signing transactions.
    /// * `broadcast` - Whether to broadcast transactions.
    /// * `working_dir` - The directory to run the script in.
    /// * `extra_args` - Additional arguments.
    pub async fn script_in_dir(
        &self,
        script_path: &str,
        rpc_url: &str,
        private_key: Option<&str>,
        broadcast: bool,
        working_dir: &Path,
        extra_args: &[&str],
    ) -> Result<ForgeOutput> {
        let mut args = vec!["script", script_path, "--rpc-url", rpc_url];

        if let Some(pk) = private_key {
            args.push("--private-key");
            args.push(pk);
        }

        if broadcast {
            args.push("--broadcast");
        }

        args.extend(extra_args.iter().copied());

        self.execute_in_dir(args, working_dir).await
    }

    /// Builds contracts in the current directory.
    ///
    /// # Returns
    ///
    /// A `ForgeOutput` containing the build results.
    pub async fn build(&self) -> Result<ForgeOutput> {
        self.execute(["build"]).await
    }

    /// Builds contracts in a specific directory.
    ///
    /// # Arguments
    ///
    /// * `working_dir` - The directory containing the contracts.
    pub async fn build_in_dir(&self, working_dir: &Path) -> Result<ForgeOutput> {
        self.execute_in_dir(["build"], working_dir).await
    }

    /// Gets the forge version.
    ///
    /// # Returns
    ///
    /// The version string from `forge --version`.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or version cannot be parsed.
    pub async fn version(&self) -> Result<String> {
        let output = self.execute(["--version"]).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "forge --version failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        // Parse version from output (format: "forge X.Y.Z" or similar)
        Ok(output.stdout.trim().to_string())
    }

    /// Checks if forge is available and returns version info.
    ///
    /// # Returns
    ///
    /// `Ok(version)` if forge is available, or an error if not found.
    pub async fn check_available(&self) -> Result<String> {
        self.version().await.wrap_err(
            "forge CLI not found. Install foundry-zksync: \
             curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash",
        )
    }
}

impl Default for ForgeCli {
    fn default() -> Self {
        Self::new()
    }
}

/// Upgrade input configuration from era-contracts upgrade-envs.
///
/// Structure varies by protocol version - stored as raw TOML for flexibility.
///
/// # Example
///
/// ```toml
/// era_chain_id = 531050204
/// testnet_verifier = true
/// old_protocol_version = "0x1d00000001"
///
/// [contracts]
/// genesis_root = "0x6ef70107..."
/// bridgehub_proxy_address = "0xc4fd2580..."
///
/// [state_transition]
/// admin_facet_addr = "0x493EE7a0..."
/// ```
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeInputConfig {
    raw: toml::Value,
}

#[allow(dead_code)]
impl UpgradeInputConfig {
    /// Creates a new UpgradeInputConfig from a TOML value.
    pub fn new(raw: toml::Value) -> Self {
        Self { raw }
    }

    /// Load configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("Failed to read upgrade config file: {}", path.display()))?;
        Self::from_str(&content)
    }

    /// Load configuration from a TOML string.
    ///
    /// # Arguments
    ///
    /// * `content` - TOML content as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the content cannot be parsed.
    pub fn from_str(content: &str) -> Result<Self> {
        let raw: toml::Value =
            toml::from_str(content).wrap_err("Failed to parse upgrade config TOML")?;
        Ok(Self { raw })
    }

    /// Get a typed value by dotted path (e.g., "contracts.genesis_root").
    ///
    /// # Arguments
    ///
    /// * `path` - Dotted path to the value (e.g., "contracts.genesis_root").
    ///
    /// # Returns
    ///
    /// `Some(value)` if the path exists and can be deserialized, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let chain_id: Option<u64> = config.get("era_chain_id")?;
    /// let genesis: Option<String> = config.get("contracts.genesis_root")?;
    /// ```
    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.raw;

        for part in &parts {
            match current {
                toml::Value::Table(table) => {
                    if let Some(value) = table.get(*part) {
                        current = value;
                    } else {
                        return Ok(None);
                    }
                }
                _ => return Ok(None),
            }
        }

        let result: T = current
            .clone()
            .try_into()
            .wrap_err_with(|| format!("Failed to deserialize value at path: {}", path))?;
        Ok(Some(result))
    }

    /// Check if a dotted path exists in the config.
    ///
    /// # Arguments
    ///
    /// * `path` - Dotted path to check.
    pub fn has(&self, path: &str) -> bool {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.raw;

        for part in &parts {
            match current {
                toml::Value::Table(table) => {
                    if let Some(value) = table.get(*part) {
                        current = value;
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }

    /// Get the raw TOML value for direct access.
    pub fn raw(&self) -> &toml::Value {
        &self.raw
    }

    /// Serialize to TOML string for writing to file.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(&self.raw).wrap_err("Failed to serialize upgrade config to TOML")
    }

    /// Set a value at a dotted path.
    ///
    /// # Arguments
    ///
    /// * `path` - Dotted path where to set the value.
    /// * `value` - Value to set.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be traversed (non-table intermediate).
    pub fn set<T: Serialize>(&mut self, path: &str, value: T) -> Result<()> {
        let toml_value =
            toml::Value::try_from(value).wrap_err("Failed to convert value to TOML")?;

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &mut self.raw;

        // Navigate to parent, creating tables as needed
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - set the value
                if let toml::Value::Table(table) = current {
                    table.insert((*part).to_string(), toml_value);
                    return Ok(());
                } else {
                    return Err(eyre::eyre!(
                        "Cannot set value at path '{}': parent is not a table",
                        path
                    ));
                }
            } else {
                // Intermediate part - ensure it's a table
                if let toml::Value::Table(table) = current {
                    current = table
                        .entry((*part).to_string())
                        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
                } else {
                    return Err(eyre::eyre!(
                        "Cannot navigate path '{}': intermediate '{}' is not a table",
                        path,
                        part
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Output from a forge upgrade script execution.
///
/// Contains the parsed results including deployed addresses, governance calls,
/// and transaction data.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeScriptOutput {
    /// New contract addresses deployed during the upgrade.
    pub deployed_addresses: HashMap<String, Address>,

    /// Governance calls extracted from the script output.
    pub governance_calls: GovernanceCalls,

    /// Raw TOML output from the forge script.
    pub raw_output: toml::Value,
}

/// Governance calls for executing an upgrade.
///
/// Contains the stage-based calls that need to be executed through governance.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GovernanceCalls {
    /// Stage 0 calls (initial setup).
    #[serde(default)]
    pub stage0: Vec<GovernanceCall>,

    /// Stage 1 calls (main upgrade).
    #[serde(default)]
    pub stage1: Vec<GovernanceCall>,

    /// Stage 2 calls (finalization).
    #[serde(default)]
    pub stage2: Vec<GovernanceCall>,
}

/// A single governance call.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceCall {
    /// Target contract address.
    pub target: Address,

    /// Encoded calldata.
    pub calldata: Bytes,

    /// Value to send (in wei).
    pub value: String,
}

#[allow(dead_code)]
impl ForgeCli {
    /// Runs an upgrade simulation script.
    ///
    /// Executes a forge script for upgrade preparation without broadcasting.
    ///
    /// # Arguments
    ///
    /// * `script_path` - Path to the upgrade script.
    /// * `rpc_url` - RPC endpoint URL.
    /// * `input_config_path` - Path to the upgrade input TOML file.
    /// * `working_dir` - Directory to run the script in.
    ///
    /// # Returns
    ///
    /// A `ForgeOutput` containing the script execution results.
    pub async fn run_upgrade_script(
        &self,
        script_path: &str,
        rpc_url: &str,
        input_config_path: &Path,
        working_dir: &Path,
    ) -> Result<ForgeOutput> {
        let input_path_str = input_config_path.to_string_lossy();
        let args = vec!["script", script_path, "--rpc-url", rpc_url, "--ffi", "-vvv"];

        // Set the environment variable for the input config
        let output = Command::new(&self.binary_path)
            .args(&args)
            .current_dir(working_dir)
            .env("UPGRADE_INPUT_CONFIG", input_path_str.as_ref())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to execute upgrade script in {}: {}",
                    working_dir.display(),
                    self.binary_path
                )
            })?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ForgeOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Runs an upgrade script and broadcasts transactions.
    ///
    /// # Arguments
    ///
    /// * `script_path` - Path to the upgrade script.
    /// * `rpc_url` - RPC endpoint URL.
    /// * `private_key` - Private key for signing transactions.
    /// * `input_config_path` - Path to the upgrade input TOML file.
    /// * `working_dir` - Directory to run the script in.
    ///
    /// # Returns
    ///
    /// A `ForgeOutput` containing the execution results.
    pub async fn run_upgrade_script_with_broadcast(
        &self,
        script_path: &str,
        rpc_url: &str,
        private_key: &str,
        input_config_path: &Path,
        working_dir: &Path,
    ) -> Result<ForgeOutput> {
        let input_path_str = input_config_path.to_string_lossy();
        let args = vec![
            "script",
            script_path,
            "--rpc-url",
            rpc_url,
            "--private-key",
            private_key,
            "--broadcast",
            "--ffi",
            "-vvv",
        ];

        let output = Command::new(&self.binary_path)
            .args(&args)
            .current_dir(working_dir)
            .env("UPGRADE_INPUT_CONFIG", input_path_str.as_ref())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to execute upgrade script with broadcast in {}: {}",
                    working_dir.display(),
                    self.binary_path
                )
            })?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ForgeOutput {
            exit_code,
            stdout,
            stderr,
        })
    }
}

/// Parses the output from a forge upgrade script into structured data.
///
/// Extracts deployed addresses, governance calls, and other relevant data
/// from the TOML output file generated by the forge script.
///
/// # Arguments
///
/// * `output_path` - Path to the TOML output file from the forge script.
///
/// # Returns
///
/// An `UpgradeScriptOutput` containing the parsed data.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
pub fn parse_upgrade_script_output(output_path: &Path) -> Result<UpgradeScriptOutput> {
    let content = std::fs::read_to_string(output_path).wrap_err_with(|| {
        format!(
            "Failed to read upgrade script output: {}",
            output_path.display()
        )
    })?;

    let raw_output: toml::Value =
        toml::from_str(&content).wrap_err("Failed to parse upgrade script output as TOML")?;

    // Extract deployed addresses
    let deployed_addresses = extract_deployed_addresses(&raw_output)?;

    // Extract governance calls
    let governance_calls = extract_governance_calls(&raw_output)?;

    Ok(UpgradeScriptOutput {
        deployed_addresses,
        governance_calls,
        raw_output,
    })
}

/// Extracts deployed addresses from the upgrade script output.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
fn extract_deployed_addresses(output: &toml::Value) -> Result<HashMap<String, Address>> {
    let mut addresses = HashMap::new();

    // Look for deployed_addresses section
    if let Some(toml::Value::Table(table)) = output.get("deployed_addresses") {
        for (key, value) in table {
            if let toml::Value::String(addr_str) = value {
                let addr: Address = addr_str
                    .parse()
                    .wrap_err_with(|| format!("Invalid address for {}: {}", key, addr_str))?;
                addresses.insert(key.clone(), addr);
            }
        }
    }

    Ok(addresses)
}

/// Extracts governance calls from the upgrade script output.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
fn extract_governance_calls(output: &toml::Value) -> Result<GovernanceCalls> {
    let mut calls = GovernanceCalls::default();

    if let Some(toml::Value::Table(table)) = output.get("governance_calls") {
        // Extract stage0 calls
        if let Some(stage0) = table.get("stage0") {
            calls.stage0 = parse_governance_call_list(stage0)?;
        }

        // Extract stage1 calls
        if let Some(stage1) = table.get("stage1") {
            calls.stage1 = parse_governance_call_list(stage1)?;
        }

        // Extract stage2 calls
        if let Some(stage2) = table.get("stage2") {
            calls.stage2 = parse_governance_call_list(stage2)?;
        }
    }

    Ok(calls)
}

/// Parses a list of governance calls from TOML.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
fn parse_governance_call_list(value: &toml::Value) -> Result<Vec<GovernanceCall>> {
    let mut calls = Vec::new();

    if let toml::Value::Array(array) = value {
        for item in array {
            if let toml::Value::Table(table) = item {
                let target_str = table
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| eyre::eyre!("Missing 'target' in governance call"))?;

                let calldata_str = table
                    .get("calldata")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| eyre::eyre!("Missing 'calldata' in governance call"))?;

                let value_str = table.get("value").and_then(|v| v.as_str()).unwrap_or("0");

                let target: Address = target_str
                    .parse()
                    .wrap_err_with(|| format!("Invalid target address: {}", target_str))?;

                let calldata: Bytes = calldata_str
                    .parse()
                    .wrap_err_with(|| format!("Invalid calldata: {}", calldata_str))?;

                calls.push(GovernanceCall {
                    target,
                    calldata,
                    value: value_str.to_string(),
                });
            }
        }
    }

    Ok(calls)
}

/// Generates upgrade input TOML from ecosystem state.
///
/// Creates an `UpgradeInputConfig` populated with the current ecosystem
/// contract addresses and configuration needed for an upgrade script.
///
/// # Arguments
///
/// * `ecosystem_name` - Name of the ecosystem.
/// * `chain_id` - Chain ID of the settlement layer.
/// * `old_protocol_version` - Current protocol version (hex string).
/// * `new_protocol_version` - Target protocol version (hex string).
/// * `contracts` - Map of contract names to addresses.
/// * `testnet_verifier` - Whether to use testnet verifier.
///
/// # Returns
///
/// An `UpgradeInputConfig` ready to be written to a file.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
pub fn generate_upgrade_input_config(
    ecosystem_name: &str,
    chain_id: u64,
    old_protocol_version: &str,
    new_protocol_version: &str,
    contracts: &HashMap<String, Address>,
    testnet_verifier: bool,
) -> Result<UpgradeInputConfig> {
    let mut config = UpgradeInputConfig::new(toml::Value::Table(toml::map::Map::new()));

    // Set top-level fields
    config.set("ecosystem_name", ecosystem_name)?;
    config.set("era_chain_id", chain_id)?;
    config.set("testnet_verifier", testnet_verifier)?;
    config.set("old_protocol_version", old_protocol_version)?;
    config.set("latest_protocol_version", new_protocol_version)?;

    // Set contract addresses
    for (name, address) in contracts {
        let path = format!("contracts.{}", name);
        config.set(&path, format!("{:?}", address))?;
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_output_success() {
        let output = ForgeOutput {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };
        assert!(output.success());
    }

    #[test]
    fn test_forge_output_failure() {
        let output = ForgeOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!output.success());
    }

    #[test]
    fn test_forge_cli_default_path() {
        let forge = ForgeCli::new();
        assert_eq!(forge.binary_path(), "forge");
    }

    #[test]
    fn test_forge_cli_custom_path() {
        let forge = ForgeCli::with_binary_path("/custom/path/forge");
        assert_eq!(forge.binary_path(), "/custom/path/forge");
    }
}
