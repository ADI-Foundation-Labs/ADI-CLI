//! Add chain command for adding chains to existing ecosystems.

mod chain;

use adi_ecosystem::ProverMode;
use alloy_primitives::Address;
use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

pub use chain::run;

/// Arguments for `add` command.
///
/// Adds a new chain to an existing ecosystem.
/// Creates chain configuration with ZkStack.yaml and wallets.
/// Requires an initialized ecosystem (run `adi init` first).
///
/// All arguments are optional and will fall back to values from the config file
/// (.adi.yml) if not provided.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct AddArgs {
    /// Protocol version for the toolkit Docker image (e.g., v30.0.2).
    /// Determines which zkstack version to use for chain creation.
    /// Falls back to config file if not provided.
    #[arg(long, short = 'p')]
    pub protocol_version: Option<String>,

    /// Ecosystem name (falls back to config file if not provided).
    #[arg(
        long,
        help = "Ecosystem name (falls back to config file if not provided)"
    )]
    pub ecosystem_name: Option<String>,

    /// Settlement layer JSON-RPC URL for chain ID validation
    #[arg(
        long,
        env = "ADI_RPC_URL",
        help = "Settlement layer JSON-RPC URL (e.g., http://localhost:8545)"
    )]
    pub rpc_url: Option<Url>,

    /// Name for the new chain (must be unique within ecosystem).
    #[arg(
        long,
        help = "Name for the new chain (falls back to config file if not provided)"
    )]
    pub chain_name: Option<String>,

    /// Unique numeric chain ID (must not conflict with existing chains).
    #[arg(
        long,
        help = "Unique numeric chain ID (falls back to config file if not provided)"
    )]
    pub chain_id: Option<u64>,

    /// Prover mode: no-proofs (testing) or gpu (production with ZK proofs).
    #[arg(
        long,
        value_enum,
        help = "Prover mode: no-proofs or gpu (falls back to config file if not provided)"
    )]
    pub prover_mode: Option<ProverMode>,

    /// Custom base token contract address. Use ETH address (0x0...01) for native ETH.
    #[arg(
        long,
        help = "Custom base token contract address (falls back to config file if not provided)"
    )]
    pub base_token_address: Option<Address>,

    /// Price ratio numerator (with denominator, sets ETH/token rate).
    #[arg(
        long,
        help = "Price ratio numerator (falls back to config file if not provided)"
    )]
    pub base_token_price_nominator: Option<u64>,

    /// Price ratio denominator (with nominator, sets ETH/token rate).
    #[arg(
        long,
        help = "Price ratio denominator (falls back to config file if not provided)"
    )]
    pub base_token_price_denominator: Option<u64>,

    /// Enable EVM bytecode emulator for running unmodified Ethereum contracts.
    #[arg(
        long,
        help = "Enable EVM bytecode emulator (falls back to config file if not provided)"
    )]
    pub evm_emulator: Option<bool>,

    /// Skip confirmation prompt.
    #[arg(long, short = 'y', help = "Skip confirmation prompt")]
    pub yes: bool,

    /// Force overwrite if chain already exists.
    #[arg(long, short = 'f', help = "Overwrite if chain already exists")]
    pub force: bool,
}
