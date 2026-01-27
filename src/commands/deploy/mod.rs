//! Deploy subcommands for deploying ecosystem and chain contracts.

use clap::Subcommand;
use eyre::bail;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

/// Deploy subcommands for ecosystem and chain contract deployment.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeployCommand {
    /// Deploy ecosystem contracts to the settlement layer
    Ecosystem,
    /// Deploy chain contracts to the settlement layer and register with Bridgehub
    Chain,
}

impl DeployCommand {
    /// Execute the deploy subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            DeployCommand::Ecosystem => deploy_ecosystem(context).await,
            DeployCommand::Chain => deploy_chain(context).await,
        }
    }
}

/// Deploy ecosystem contracts to the settlement layer.
///
/// This is a placeholder that will be implemented in US2 tasks (T048-T059).
async fn deploy_ecosystem(_context: &Context) -> Result<()> {
    bail!("deploy ecosystem command not yet implemented")
}

/// Deploy chain contracts to the settlement layer and register with Bridgehub.
///
/// This is a placeholder that will be implemented in US3b tasks (T066-T073).
async fn deploy_chain(_context: &Context) -> Result<()> {
    bail!("deploy chain command not yet implemented")
}
