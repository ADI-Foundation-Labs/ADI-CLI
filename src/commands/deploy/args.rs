//! CLI arguments for the deploy command.

use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

/// Arguments for `deploy` command.
///
/// Funds ecosystem wallets and deploys core infrastructure contracts.
/// Requires initialized ecosystem (run `adi init` first).
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct DeployArgs {
    /// Ecosystem name (falls back to config file if not provided)
    #[arg(
        long,
        help = "Ecosystem name (falls back to config file if not provided)"
    )]
    pub ecosystem_name: Option<String>,

    /// Chain name for wallet funding (falls back to config file if not provided)
    #[arg(
        long,
        help = "Chain name for wallet funding (falls back to config file if not provided)"
    )]
    pub chain_name: Option<String>,

    /// Settlement layer JSON-RPC URL (e.g., http://localhost:8545 or https://sepolia.infura.io/v3/KEY)
    #[arg(
        long,
        env = "ADI_RPC_URL",
        help = "Settlement layer JSON-RPC URL (e.g., http://localhost:8545)"
    )]
    pub rpc_url: Option<Url>,

    /// Funder wallet private key (hex). Prefer config file or env var for security
    #[arg(
        long,
        env = "ADI_FUNDER_KEY",
        help = "Funder wallet private key (hex). Prefer config file or env var for security"
    )]
    pub funder_key: Option<String>,

    /// Gas price multiplier percentage (default: 120 = 20% buffer over estimated gas)
    #[arg(
        long,
        help = "Gas price multiplier percentage (default: 120 = 20% buffer over estimated gas)"
    )]
    pub gas_multiplier: Option<u64>,

    /// Deployer wallet ETH amount in ether (default: 1.0)
    #[arg(long, help = "Deployer wallet ETH amount in ether (default: 1.0)")]
    pub deployer_eth: Option<f64>,

    /// Governor wallet ETH amount in ether (default: 1.0)
    #[arg(long, help = "Governor wallet ETH amount in ether (default: 1.0)")]
    pub governor_eth: Option<f64>,

    /// Governor custom gas token (CGT) amount. Only for chains with custom base token (default: 5.0)
    #[arg(
        long,
        help = "Governor custom gas token (CGT) amount. Only for chains with custom base token (default: 5.0)"
    )]
    pub governor_cgt_units: Option<f64>,

    /// Operator wallet ETH amount in ether (default: 5.0)
    #[arg(long, help = "Operator wallet ETH amount in ether (default: 5.0)")]
    pub operator_eth: Option<f64>,

    /// Prove operator wallet ETH (submits validity proofs to L1, default: 5.0)
    #[arg(
        long,
        help = "Prove operator wallet ETH (submits validity proofs to L1, default: 5.0)"
    )]
    pub prove_operator_eth: Option<f64>,

    /// Execute operator wallet ETH (executes batches on L1, default: 5.0)
    #[arg(
        long,
        help = "Execute operator wallet ETH (executes batches on L1, default: 5.0)"
    )]
    pub execute_operator_eth: Option<f64>,

    /// Skip wallet funding step (use if wallets are already funded)
    #[arg(
        long,
        help = "Skip wallet funding step (use if wallets are already funded)"
    )]
    pub skip_funding: bool,

    /// Preview funding plan without executing transactions
    #[arg(long, help = "Preview funding plan without executing transactions")]
    pub dry_run: bool,

    /// Skip confirmation prompt (for automation/scripting)
    #[arg(
        long,
        short = 'y',
        help = "Skip confirmation prompt (for automation/scripting)"
    )]
    pub yes: bool,

    /// Skip contract deployment step (only fund wallets)
    #[arg(long, help = "Skip contract deployment step (only fund wallets)")]
    pub skip_deployment: bool,

    /// Protocol version for toolkit image (e.g., v30.0.2). Required for deployment
    #[arg(
        long,
        short = 'p',
        help = "Protocol version for toolkit image (e.g., v30.0.2)"
    )]
    pub protocol_version: Option<String>,

    /// Use blob-based pubdata (EIP-4844). Overrides chain config if specified.
    ///
    /// When `true`, uses blobs for pubdata (L2 chains settling on L1).
    /// When `false`, uses calldata for pubdata (L3 chains settling on L2).
    #[arg(
        long,
        help = "Use blob-based pubdata (true=blobs for L2, false=calldata for L3)"
    )]
    pub blobs: Option<bool>,

    /// Enable Validium mode (no DA). Overrides chain config if specified.
    #[arg(long, help = "Enable Validium mode (no DA)")]
    pub validium: Option<bool>,
}
