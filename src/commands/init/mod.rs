//! Initialization commands for ecosystem and chain setup.

mod ecosystem;

use adi_ecosystem::{L1Network, ProverMode, ETH_ADDRESS};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Subcommands for the `init` command.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InitCommand {
    /// Initialize a new ecosystem configuration
    Ecosystem(EcosystemArgs),
}

/// Arguments for `init ecosystem` command.
///
/// All arguments except `protocol_version` are optional and will fall back
/// to values from the config file (.adi.yml) if not provided.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct EcosystemArgs {
    /// Protocol version for the toolkit image (e.g., v29.0.11)
    #[arg(long, short = 'p')]
    pub protocol_version: String,

    /// Ecosystem name
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// L1 network (localhost, sepolia, mainnet)
    #[arg(long, value_enum)]
    pub l1_network: Option<L1Network>,

    /// Initial chain name
    #[arg(long)]
    pub chain_name: Option<String>,

    /// Initial chain ID
    #[arg(long)]
    pub chain_id: Option<u64>,

    /// Prover mode (no-proofs, gpu)
    #[arg(long, value_enum)]
    pub prover_mode: Option<ProverMode>,

    /// Base token address (default: ETH)
    #[arg(long, default_value = ETH_ADDRESS)]
    pub base_token_address: Option<String>,

    /// Base token price nominator
    #[arg(long)]
    pub base_token_price_nominator: Option<u64>,

    /// Base token price denominator
    #[arg(long)]
    pub base_token_price_denominator: Option<u64>,

    /// Enable EVM emulator
    #[arg(long)]
    pub evm_emulator: Option<bool>,
}

impl InitCommand {
    /// Execute the init subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            InitCommand::Ecosystem(args) => ecosystem::run(&args, context).await,
        }
    }
}
