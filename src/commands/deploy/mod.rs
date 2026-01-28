//! Deploy subcommands for deploying ecosystem and chain contracts.

pub mod chain;
pub mod ecosystem;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

pub use chain::DeployChain;
pub use ecosystem::DeployEcosystem;

use crate::{context::Context, error::Result};

/// Deploy subcommands for ecosystem and chain contract deployment.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeployCommand {
    /// Deploy ecosystem contracts to the settlement layer
    Ecosystem(DeployEcosystem),
    /// Deploy chain contracts to the settlement layer and register with Bridgehub
    Chain(DeployChain),
}

impl DeployCommand {
    /// Execute the deploy subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            DeployCommand::Ecosystem(cmd) => cmd.run(context).await,
            DeployCommand::Chain(cmd) => cmd.run(context).await,
        }
    }
}
