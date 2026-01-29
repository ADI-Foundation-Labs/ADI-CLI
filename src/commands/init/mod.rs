//! Initialization commands for ecosystem and chain setup.

mod ecosystem;

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Subcommands for the `init` command.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InitCommand {
    /// Initialize a new ecosystem configuration
    Ecosystem(EcosystemArgs),
}

/// Arguments for `init ecosystem` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct EcosystemArgs {
    /// Protocol version for the toolkit image (e.g., v29.0.11, v30.0.2)
    #[arg(long, short = 'p')]
    pub protocol_version: String,
}

impl InitCommand {
    /// Execute the init subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            InitCommand::Ecosystem(args) => ecosystem::run(&args, context).await,
        }
    }
}
