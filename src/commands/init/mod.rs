//! Init subcommands for initializing ecosystem and chain configurations.

mod ecosystem;

use clap::Subcommand;
use eyre::bail;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

pub use ecosystem::InitEcosystem;

/// Init subcommands for ecosystem and chain initialization.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InitCommand {
    /// Initialize a new ZkSync ecosystem configuration
    Ecosystem(InitEcosystem),
    /// Initialize a new chain configuration within an ecosystem
    Chain,
}

impl InitCommand {
    /// Execute the init subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            InitCommand::Ecosystem(cmd) => cmd.run(context).await,
            InitCommand::Chain => init_chain(context).await,
        }
    }
}

/// Initialize a new chain configuration within an ecosystem.
///
/// This is a placeholder that will be implemented in US3 tasks (T062-T065).
async fn init_chain(_context: &Context) -> Result<()> {
    bail!("init chain command not yet implemented")
}
