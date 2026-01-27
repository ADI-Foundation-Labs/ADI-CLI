//! CLI command implementations.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

mod deploy;
mod init;
mod upgrade;
mod version;

/// Available CLI commands.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Commands {
    /// Initialize ecosystem or chain configurations
    Init {
        #[command(subcommand)]
        command: init::InitCommand,
    },
    /// Deploy ecosystem or chain contracts
    Deploy {
        #[command(subcommand)]
        command: deploy::DeployCommand,
    },
    /// Upgrade ecosystem or chain contracts
    Upgrade {
        #[command(subcommand)]
        command: upgrade::UpgradeCommand,
    },
    /// Show version information
    Version {
        #[command(subcommand)]
        command: version::VersionCommand,
    },
}

impl Commands {
    /// Execute the selected command.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            Commands::Init { command } => command.run(context).await,
            Commands::Deploy { command } => command.run(context).await,
            Commands::Upgrade { command } => command.run(context).await,
            Commands::Version { command } => command.run(context).await,
        }
    }
}
