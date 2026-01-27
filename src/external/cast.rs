//! Wrapper for the cast CLI (foundry-zksync).
//!
//! This module provides `CastCli`, which wraps the cast command-line tool
//! for Ethereum RPC interactions, transaction sending, and calldata encoding.
//! The wrapper handles:
//!
//! - Contract calls (read-only)
//! - Transaction sending (state-changing)
//! - Calldata encoding
//! - Balance checking
//! - Async execution via `tokio::process::Command`
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::external::CastCli;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let cast = CastCli::new();
//!
//!     // Check ETH balance
//!     let balance = cast.balance(
//!         "0x1234567890abcdef1234567890abcdef12345678",
//!         "http://localhost:8545",
//!     ).await?;
//!     println!("Balance: {}", balance);
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};
use std::ffi::OsStr;
use std::process::Stdio;
use tokio::process::Command;

/// Options for sending transactions with cast.
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct SendOptions<'a> {
    /// ETH value to send (in wei).
    pub value: Option<&'a str>,
    /// Gas price (in wei).
    pub gas_price: Option<&'a str>,
}

#[allow(dead_code)]
impl<'a> SendOptions<'a> {
    /// Creates a new empty SendOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the ETH value to send.
    pub fn with_value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets the gas price.
    pub fn with_gas_price(mut self, gas_price: &'a str) -> Self {
        self.gas_price = Some(gas_price);
        self
    }
}

/// Output from a cast command execution.
#[derive(Debug, Clone)]
pub struct CastOutput {
    /// Exit code from the command (0 = success).
    pub exit_code: i32,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
}

impl CastOutput {
    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Wrapper for the cast CLI tool (foundry-zksync).
///
/// Provides typed methods for common cast operations including
/// contract calls, transaction sending, and calldata encoding.
///
/// # Example
///
/// ```rust,ignore
/// use adi_cli::external::CastCli;
///
/// let cast = CastCli::new();
///
/// // Read contract state
/// let output = cast.call(
///     "0xcontract",
///     "balanceOf(address)(uint256)",
///     &["0xowner"],
///     "http://localhost:8545",
/// ).await?;
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub struct CastCli {
    /// Path to the cast binary. Defaults to "cast".
    binary_path: String,
}

#[allow(dead_code)]
impl CastCli {
    /// Creates a new CastCli instance with the default binary path.
    pub fn new() -> Self {
        Self {
            binary_path: "cast".to_string(),
        }
    }

    /// Creates a new CastCli instance with a custom binary path.
    ///
    /// # Arguments
    ///
    /// * `binary_path` - Path to the cast binary.
    pub fn with_binary_path(binary_path: impl Into<String>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }

    /// Returns the path to the cast binary.
    pub fn binary_path(&self) -> &str {
        &self.binary_path
    }

    /// Executes a cast command with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments to pass to cast.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the exit code, stdout, and stderr.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to spawn or execute.
    pub async fn execute<I, S>(&self, args: I) -> Result<CastOutput>
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
            .wrap_err_with(|| format!("Failed to execute cast command: {}", self.binary_path))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CastOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Performs a read-only contract call.
    ///
    /// # Arguments
    ///
    /// * `contract` - Contract address.
    /// * `signature` - Function signature (e.g., "balanceOf(address)(uint256)").
    /// * `args` - Function arguments.
    /// * `rpc_url` - RPC endpoint URL.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the call result.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let output = cast.call(
    ///     "0xTokenContract",
    ///     "balanceOf(address)(uint256)",
    ///     &["0xOwnerAddress"],
    ///     "http://localhost:8545",
    /// ).await?;
    /// println!("Balance: {}", output.stdout.trim());
    /// ```
    pub async fn call(
        &self,
        contract: &str,
        signature: &str,
        args: &[&str],
        rpc_url: &str,
    ) -> Result<CastOutput> {
        let mut cmd_args = vec!["call", contract, signature];
        cmd_args.extend(args.iter().copied());
        cmd_args.push("--rpc-url");
        cmd_args.push(rpc_url);

        self.execute(cmd_args).await
    }

    /// Sends a transaction to a contract.
    ///
    /// # Arguments
    ///
    /// * `contract` - Contract address.
    /// * `signature` - Function signature (e.g., "transfer(address,uint256)").
    /// * `args` - Function arguments.
    /// * `private_key` - Private key for signing the transaction.
    /// * `rpc_url` - RPC endpoint URL.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the transaction result.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let output = cast.send(
    ///     "0xTokenContract",
    ///     "transfer(address,uint256)",
    ///     &["0xRecipient", "1000000000000000000"],
    ///     "0xprivate_key",
    ///     "http://localhost:8545",
    /// ).await?;
    /// ```
    pub async fn send(
        &self,
        contract: &str,
        signature: &str,
        args: &[&str],
        private_key: &str,
        rpc_url: &str,
    ) -> Result<CastOutput> {
        let mut cmd_args = vec!["send", contract, signature];
        cmd_args.extend(args.iter().copied());
        cmd_args.push("--private-key");
        cmd_args.push(private_key);
        cmd_args.push("--rpc-url");
        cmd_args.push(rpc_url);

        self.execute(cmd_args).await
    }

    /// Sends a transaction with additional options.
    ///
    /// # Arguments
    ///
    /// * `contract` - Contract address.
    /// * `signature` - Function signature.
    /// * `args` - Function arguments.
    /// * `private_key` - Private key for signing.
    /// * `rpc_url` - RPC endpoint URL.
    /// * `options` - Additional transaction options (value, gas price).
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the transaction result.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let options = SendOptions::new()
    ///     .with_value("1000000000000000000")
    ///     .with_gas_price("10000000000");
    ///
    /// let output = cast.send_with_options(
    ///     "0xContract",
    ///     "deposit()",
    ///     &[],
    ///     "0xprivate_key",
    ///     "http://localhost:8545",
    ///     &options,
    /// ).await?;
    /// ```
    pub async fn send_with_options(
        &self,
        contract: &str,
        signature: &str,
        args: &[&str],
        private_key: &str,
        rpc_url: &str,
        options: &SendOptions<'_>,
    ) -> Result<CastOutput> {
        let mut cmd_args = vec!["send", contract, signature];
        cmd_args.extend(args.iter().copied());
        cmd_args.push("--private-key");
        cmd_args.push(private_key);
        cmd_args.push("--rpc-url");
        cmd_args.push(rpc_url);

        if let Some(v) = options.value {
            cmd_args.push("--value");
            cmd_args.push(v);
        }

        if let Some(gp) = options.gas_price {
            cmd_args.push("--gas-price");
            cmd_args.push(gp);
        }

        self.execute(cmd_args).await
    }

    /// Encodes calldata for a function call.
    ///
    /// # Arguments
    ///
    /// * `signature` - Function signature (e.g., "transfer(address,uint256)").
    /// * `args` - Function arguments.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the encoded calldata.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let output = cast.calldata(
    ///     "transfer(address,uint256)",
    ///     &["0xRecipient", "1000000000000000000"],
    /// ).await?;
    /// println!("Calldata: {}", output.stdout.trim());
    /// ```
    pub async fn calldata(&self, signature: &str, args: &[&str]) -> Result<CastOutput> {
        let mut cmd_args = vec!["calldata", signature];
        cmd_args.extend(args.iter().copied());

        self.execute(cmd_args).await
    }

    /// Gets the ETH balance of an address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address to check.
    /// * `rpc_url` - RPC endpoint URL.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the balance in wei.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let output = cast.balance(
    ///     "0x1234567890abcdef1234567890abcdef12345678",
    ///     "http://localhost:8545",
    /// ).await?;
    /// println!("Balance: {} wei", output.stdout.trim());
    /// ```
    pub async fn balance(&self, address: &str, rpc_url: &str) -> Result<CastOutput> {
        self.execute(["balance", address, "--rpc-url", rpc_url])
            .await
    }

    /// Gets the ERC-20 token balance of an address.
    ///
    /// # Arguments
    ///
    /// * `token_contract` - The ERC-20 token contract address.
    /// * `address` - The address to check.
    /// * `rpc_url` - RPC endpoint URL.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the token balance.
    pub async fn token_balance(
        &self,
        token_contract: &str,
        address: &str,
        rpc_url: &str,
    ) -> Result<CastOutput> {
        self.call(
            token_contract,
            "balanceOf(address)(uint256)",
            &[address],
            rpc_url,
        )
        .await
    }

    /// Sends ETH to an address.
    ///
    /// # Arguments
    ///
    /// * `to` - Recipient address.
    /// * `value` - Amount to send in wei.
    /// * `private_key` - Private key for signing.
    /// * `rpc_url` - RPC endpoint URL.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the transaction result.
    pub async fn send_eth(
        &self,
        to: &str,
        value: &str,
        private_key: &str,
        rpc_url: &str,
    ) -> Result<CastOutput> {
        self.execute([
            "send",
            to,
            "--value",
            value,
            "--private-key",
            private_key,
            "--rpc-url",
            rpc_url,
        ])
        .await
    }

    /// Sends ERC-20 tokens to an address.
    ///
    /// # Arguments
    ///
    /// * `token_contract` - The ERC-20 token contract address.
    /// * `to` - Recipient address.
    /// * `amount` - Amount to send (in token units).
    /// * `private_key` - Private key for signing.
    /// * `rpc_url` - RPC endpoint URL.
    ///
    /// # Returns
    ///
    /// A `CastOutput` containing the transaction result.
    pub async fn send_token(
        &self,
        token_contract: &str,
        to: &str,
        amount: &str,
        private_key: &str,
        rpc_url: &str,
    ) -> Result<CastOutput> {
        self.send(
            token_contract,
            "transfer(address,uint256)",
            &[to, amount],
            private_key,
            rpc_url,
        )
        .await
    }

    /// Gets the cast version.
    ///
    /// # Returns
    ///
    /// The version string from `cast --version`.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or version cannot be parsed.
    pub async fn version(&self) -> Result<String> {
        let output = self.execute(["--version"]).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "cast --version failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        // Parse version from output (format: "cast X.Y.Z" or similar)
        Ok(output.stdout.trim().to_string())
    }

    /// Checks if cast is available and returns version info.
    ///
    /// # Returns
    ///
    /// `Ok(version)` if cast is available, or an error if not found.
    pub async fn check_available(&self) -> Result<String> {
        self.version().await.wrap_err(
            "cast CLI not found. Install foundry-zksync: \
             curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash",
        )
    }
}

impl Default for CastCli {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_output_success() {
        let output = CastOutput {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };
        assert!(output.success());
    }

    #[test]
    fn test_cast_output_failure() {
        let output = CastOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!output.success());
    }

    #[test]
    fn test_cast_cli_default_path() {
        let cast = CastCli::new();
        assert_eq!(cast.binary_path(), "cast");
    }

    #[test]
    fn test_cast_cli_custom_path() {
        let cast = CastCli::with_binary_path("/custom/path/cast");
        assert_eq!(cast.binary_path(), "/custom/path/cast");
    }
}
