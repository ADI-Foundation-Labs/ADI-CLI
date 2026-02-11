//! Initialization command for ecosystem and chain setup.

mod ecosystem;

use adi_ecosystem::{L1Network, ProverMode};
use alloy_primitives::Address;
use clap::Args;
use serde::{Deserialize, Serialize};

pub use ecosystem::run;

/// Arguments for `init` command.
///
/// Creates ecosystem configuration with an initial chain.
/// Generates ZkStack.yaml, wallet keys, and chain config in the state directory.
/// Requires genesis.json in state dir (download from protocol version URL).
///
/// All arguments except `protocol_version` are optional and will fall back
/// to values from the config file (.adi.yml) if not provided.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct InitArgs {
    /// Protocol version for the toolkit Docker image (e.g., v29.0.11).
    /// Determines which era-contracts and zkstack version to use.
    #[arg(long, short = 'p')]
    pub protocol_version: String,

    /// Ecosystem name (used for directory name and identification)
    #[arg(
        long,
        help = "Ecosystem name (used for directory name and identification)"
    )]
    pub ecosystem_name: Option<String>,

    /// Settlement layer network: localhost (Anvil), sepolia (testnet), or mainnet
    #[arg(
        long,
        value_enum,
        help = "Settlement layer network: localhost (Anvil), sepolia (testnet), or mainnet"
    )]
    pub l1_network: Option<L1Network>,

    /// Name for the initial chain within this ecosystem
    #[arg(long, help = "Name for the initial chain within this ecosystem")]
    pub chain_name: Option<String>,

    /// Unique numeric chain ID (must not conflict with existing chains)
    #[arg(
        long,
        help = "Unique numeric chain ID (must not conflict with existing chains)"
    )]
    pub chain_id: Option<u64>,

    /// Prover mode: no-proofs (testing) or gpu (production with ZK proofs)
    #[arg(
        long,
        value_enum,
        help = "Prover mode: no-proofs (testing) or gpu (production with ZK proofs)"
    )]
    pub prover_mode: Option<ProverMode>,

    /// Custom base token contract address. Use ETH address (0x0...01) for native ETH
    #[arg(
        long,
        help = "Custom base token contract address. Use ETH address (0x0...01) for native ETH"
    )]
    pub base_token_address: Option<Address>,

    /// Price ratio numerator (with denominator, sets ETH/token rate, e.g., 1:100 = 1 ETH per 100 tokens)
    #[arg(
        long,
        help = "Price ratio numerator (with denominator, sets ETH/token rate, e.g., 1:100 = 1 ETH per 100 tokens)"
    )]
    pub base_token_price_nominator: Option<u64>,

    /// Price ratio denominator (with nominator, sets ETH/token rate, e.g., 1:100 = 1 ETH per 100 tokens)
    #[arg(
        long,
        help = "Price ratio denominator (with nominator, sets ETH/token rate, e.g., 1:100 = 1 ETH per 100 tokens)"
    )]
    pub base_token_price_denominator: Option<u64>,

    /// Enable EVM bytecode emulator for running unmodified Ethereum contracts
    #[arg(
        long,
        help = "Enable EVM bytecode emulator for running unmodified Ethereum contracts"
    )]
    pub evm_emulator: Option<bool>,

    /// Force reinitialization without confirmation prompt
    #[arg(
        long,
        short = 'f',
        help = "Force reinitialization without confirmation"
    )]
    pub force: bool,
}
