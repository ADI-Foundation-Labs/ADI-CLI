//! CLI command definitions and dispatch.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

mod accept;
mod config;
mod deploy;
mod init;
mod version;

/// Available CLI commands.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Commands {
    /// Display CLI version and build information
    Version,
    /// Display current configuration
    Config,
    /// Initialize ecosystem configuration (run before deploy)
    Init(init::InitArgs),
    /// Deploy smart contracts to the settlement layer (L1)
    Deploy(deploy::DeployArgs),
    /// Accept pending ownership transfers for deployed contracts
    Accept(accept::AcceptArgs),
}

impl Commands {
    /// Execute the selected command.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            Commands::Version => version::run().await,
            Commands::Config => config::run(context).await,
            Commands::Init(args) => init::run(&args, context).await,
            Commands::Deploy(args) => deploy::run(args, context).await,
            Commands::Accept(args) => accept::run(args, context).await,
        }
    }
}
