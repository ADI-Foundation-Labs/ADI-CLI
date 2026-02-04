use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

mod deploy;
mod init;
mod show;

#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Commands {
    /// Show various information (version, config)
    Show {
        #[command(subcommand)]
        command: show::ShowCommand,
    },
    /// Initialize ecosystem or chain
    Init {
        #[command(subcommand)]
        command: init::InitCommand,
    },
    /// Deploy ecosystem or chain contracts
    Deploy {
        #[command(subcommand)]
        command: deploy::DeployCommand,
    },
}

impl Commands {
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            Commands::Show { command } => command.run(context).await,
            Commands::Init { command } => command.run(context).await,
            Commands::Deploy { command } => command.run(context).await,
        }
    }
}
