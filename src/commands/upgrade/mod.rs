//! Upgrade subcommands for upgrading ecosystem and chain contracts.

pub mod chain;
pub mod ecosystem;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

pub use chain::UpgradeChain;
pub use ecosystem::UpgradeEcosystem;

/// Upgrade subcommands for ecosystem and chain contract upgrades.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpgradeCommand {
    /// Upgrade ecosystem contracts to a new protocol version
    Ecosystem(UpgradeEcosystem),
    /// Upgrade chain contracts to match ecosystem version
    Chain(UpgradeChain),
}

impl UpgradeCommand {
    /// Execute the upgrade subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            UpgradeCommand::Ecosystem(cmd) => cmd.run(context).await,
            UpgradeCommand::Chain(cmd) => cmd.run(context).await,
        }
    }
}
