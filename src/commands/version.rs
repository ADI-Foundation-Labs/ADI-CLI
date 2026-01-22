use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

// Include the generated build info
include!(concat!(env!("OUT_DIR"), "/built.rs"));

#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VersionCommand {
    /// Show version information
    Show,
}

impl VersionCommand {
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            VersionCommand::Show => show_version(context).await,
        }
    }
}

async fn show_version(context: &Context) -> Result<()> {
    let package_name = PKG_NAME;
    let package_version = PKG_VERSION;

    // Get git commit hash
    let git_commit = GIT_COMMIT_HASH.unwrap_or("unknown");
    let git_commit_short = if git_commit.len() >= 8 {
        &git_commit[..8]
    } else {
        git_commit
    };

    // Display version information
    context.info(&format!("{} {}", package_name, package_version));
    context.info(&format!("commit: {}", git_commit_short));

    Ok(())
}
