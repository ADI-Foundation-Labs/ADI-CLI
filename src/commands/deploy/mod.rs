//! Deployment commands for ecosystem and chain contracts.

mod ecosystem;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Subcommands for the `deploy` command.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeployCommand {
    /// Deploy ecosystem contracts to settlement layer
    Ecosystem(ecosystem::EcosystemDeployArgs),
}

impl DeployCommand {
    /// Execute the deploy subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            DeployCommand::Ecosystem(args) => ecosystem::run(args, context).await,
        }
    }
}
