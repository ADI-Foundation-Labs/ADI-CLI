//! Upgrade subcommands for upgrading ecosystem and chain contracts.

pub mod ecosystem;

use clap::Subcommand;
use eyre::bail;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

pub use ecosystem::UpgradeEcosystem;

/// Upgrade subcommands for ecosystem and chain contract upgrades.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpgradeCommand {
    /// Upgrade ecosystem contracts to a new protocol version
    Ecosystem(UpgradeEcosystem),
    /// Upgrade chain contracts to match ecosystem version
    Chain,
}

impl UpgradeCommand {
    /// Execute the upgrade subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            UpgradeCommand::Ecosystem(cmd) => cmd.run(context).await,
            UpgradeCommand::Chain => upgrade_chain(context).await,
        }
    }
}

/// Upgrade chain contracts to match ecosystem version.
///
/// This is a placeholder that will be implemented in US5 tasks (T085-T089).
async fn upgrade_chain(_context: &Context) -> Result<()> {
    bail!("upgrade chain command not yet implemented")
}
