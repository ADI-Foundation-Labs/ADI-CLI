use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

// Include the generated build info
include!(concat!(env!("OUT_DIR"), "/built.rs"));

#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShowCommand {
    /// Show version information
    Version,
    /// Show current configuration
    Config,
}

impl ShowCommand {
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            ShowCommand::Version => show_version().await,
            ShowCommand::Config => show_config(context).await,
        }
    }
}

async fn show_version() -> Result<()> {
    let package_name = PKG_NAME;
    let package_version = PKG_VERSION;

    let git_commit = GIT_COMMIT_HASH.unwrap_or("unknown");
    let git_commit_short = git_commit.get(..8).unwrap_or(git_commit);

    log::info!("{} {}", package_name, package_version);
    log::info!("commit: {}", git_commit_short);

    Ok(())
}

async fn show_config(context: &Context) -> Result<()> {
    let config = context.config();
    log::info!("{:#?}", config);
    Ok(())
}
