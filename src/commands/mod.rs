use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

mod accept;
mod deploy;
mod init;
mod show;

#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Commands {
    /// Display CLI version, current configuration, or system state
    Show {
        #[command(subcommand)]
        command: show::ShowCommand,
    },
    /// Initialize ecosystem or chain configuration (run before deploy)
    Init {
        #[command(subcommand)]
        command: init::InitCommand,
    },
    /// Deploy smart contracts to the settlement layer (L1)
    Deploy {
        #[command(subcommand)]
        command: deploy::DeployCommand,
    },
    /// Accept pending ownership transfers for deployed contracts
    Accept {
        #[command(subcommand)]
        command: accept::AcceptCommand,
    },
}

impl Commands {
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            Commands::Show { command } => command.run(context).await,
            Commands::Init { command } => command.run(context).await,
            Commands::Deploy { command } => command.run(context).await,
            Commands::Accept { command } => command.run(context).await,
        }
    }
}
