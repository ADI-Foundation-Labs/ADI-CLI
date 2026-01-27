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
