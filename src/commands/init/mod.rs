//! Init subcommands for initializing ecosystem and chain configurations.

mod chain;
mod ecosystem;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

pub use chain::InitChain;
pub use ecosystem::InitEcosystem;

/// Init subcommands for ecosystem and chain initialization.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InitCommand {
    /// Initialize a new ZkSync ecosystem configuration
    Ecosystem(InitEcosystem),
    /// Initialize a new chain configuration within an ecosystem
    Chain(InitChain),
}

impl InitCommand {
    /// Execute the init subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            InitCommand::Ecosystem(cmd) => cmd.run(context).await,
            InitCommand::Chain(cmd) => cmd.run(context).await,
        }
    }
}
