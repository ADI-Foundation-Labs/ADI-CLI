use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

mod version;

#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Commands {
    /// Show version information
    Version {
        #[command(subcommand)]
        command: version::VersionCommand,
    },
}

impl Commands {
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            Commands::Version { command } => command.run(context).await,
        }
    }
}
