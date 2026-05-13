//! Parameter types for toolkit command execution.

use semver::Version;
use std::path::Path;

/// Parameters for running a command in a toolkit container.
pub(crate) struct RunCommandParams<'a> {
    /// Command and arguments to execute.
    pub command: &'a [&'a str],
    /// Working directory mounted into the container.
    pub state_dir: &'a Path,
    /// Directory for saving log files.
    pub log_dir: &'a Path,
    /// Protocol version for toolkit image selection.
    pub protocol_version: &'a Version,
    /// Environment variables to pass to the container.
    pub env_vars: &'a [(&'a str, &'a str)],
    /// Command name for log file naming.
    pub log_command: &'a str,
    /// Label for progress display.
    pub log_label: &'a str,
    /// Whether to suppress output.
    pub quiet: bool,
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

/// Parameters for `forge script` execution inside the toolkit container.
pub struct ForgeScriptParams<'a> {
    /// Working directory inside the container (e.g. `/deps/fee-adjuster-contracts`).
    pub working_dir: &'a str,
    /// Script path relative to `working_dir` (e.g. `script/Deploy.s.sol`).
    pub script_path: &'a str,
    /// Function signature (e.g. `"run(address)"`).
    pub signature: &'a str,
    /// Positional arguments for the function call.
    pub sig_args: &'a [&'a str],
    /// JSON-RPC URL to broadcast against (transformed for container reachability).
    pub rpc_url: &'a str,
    /// Optional gas price in wei (`-g` flag), applied when not on localhost.
    pub gas_price_wei: Option<u128>,
    /// State directory mounted at /workspace (used for log output, not script CWD).
    pub state_dir: &'a Path,
    /// Protocol version for toolkit image selection.
    pub protocol_version: &'a Version,
    /// Label for progress display.
    pub log_label: &'a str,
    /// Command name for log file naming.
    pub log_command: &'a str,
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
