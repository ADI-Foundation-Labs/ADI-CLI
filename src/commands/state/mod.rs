//! State management commands.
//!
//! Provides commands for syncing ecosystem state to S3 and restoring from S3.

mod restore;
mod sync;

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Arguments for state management commands.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct StateArgs {
    /// State management subcommand.
    #[command(subcommand)]
    pub command: StateCommands,
}

/// State management subcommands.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
pub enum StateCommands {
    /// Manually sync local state to S3
    Sync(sync::SyncArgs),
    /// Restore state from S3 to local filesystem
    Restore(restore::RestoreArgs),
}

/// Execute the state command.
pub async fn run(args: &StateArgs, context: &Context) -> Result<()> {
    match &args.command {
        StateCommands::Sync(sync_args) => sync::run(sync_args, context).await,
        StateCommands::Restore(restore_args) => restore::run(restore_args, context).await,
    }
}
