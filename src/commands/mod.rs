//! CLI command definitions and dispatch.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

mod accept;
mod add;
pub mod chain_ops;
pub mod chain_prompts;
mod completions;
mod config;
mod deploy;
mod ecosystem;
pub mod helpers;
mod init;
mod owners;
mod server_params;
mod state;
pub mod state_paths;
mod transfer;
mod upgrade;
mod verify;
mod version;

/// Available CLI commands.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Commands {
    /// Display CLI version and build information
    Version,
    /// Display current configuration
    Config,
    /// Display ecosystem and chain information with deployed contracts
    Ecosystem(ecosystem::EcosystemArgs),
    /// Initialize ecosystem configuration (run before deploy)
    Init(init::InitArgs),
    /// Add a new chain to an existing ecosystem
    Add(add::AddArgs),
    /// Deploy smart contracts to the settlement layer (L1)
    Deploy(deploy::DeployArgs),
    /// Accept pending ownership transfers for deployed contracts
    Accept(accept::AcceptArgs),
    /// Accept and transfer ownership of ecosystem contracts to a new owner
    Transfer(transfer::TransferArgs),
    /// Display owners of deployed L1 contracts
    Owners(owners::OwnersArgs),
    /// Check and submit contract verification to block explorers
    Verify(verify::VerifyArgs),
    /// Display server parameters for Docker Compose configuration
    ServerParams(server_params::ServerParamsArgs),
    /// Manage state synchronization with S3
    State(state::StateArgs),
    /// Generate shell completion scripts
    Completions(completions::CompletionsArgs),
    /// Upgrade ecosystem and chain contracts to a new protocol version
    Upgrade(upgrade::UpgradeArgs),
}

impl Commands {
    /// Execute the selected command.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            Commands::Version => version::run().await,
            Commands::Config => config::run(context).await,
            Commands::Ecosystem(args) => ecosystem::run(&args, context).await,
            Commands::Init(args) => init::run(&args, context).await,
            Commands::Add(args) => add::run(&args, context).await,
            Commands::Deploy(args) => deploy::run(args, context).await,
            Commands::Accept(args) => accept::run(args, context).await,
            Commands::Transfer(args) => transfer::run(args, context).await,
            Commands::Owners(args) => owners::run(&args, context).await,
            Commands::Verify(args) => verify::run(args, context).await,
            Commands::ServerParams(args) => server_params::run(&args, context).await,
            Commands::State(args) => state::run(&args, context).await,
            Commands::Completions(args) => completions::run(&args).await,
            Commands::Upgrade(args) => upgrade::run(args, context).await,
        }
    }
}
