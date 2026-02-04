mod config;
mod version;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShowCommand {
    /// Display CLI version and build information
    Version,
    /// Display current configuration (merged from file, env vars, and defaults)
    Config,
}

impl ShowCommand {
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            ShowCommand::Version => version::run().await,
            ShowCommand::Config => config::run(context).await,
        }
    }
}
