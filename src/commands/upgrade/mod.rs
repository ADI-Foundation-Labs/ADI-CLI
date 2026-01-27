//! Upgrade subcommands for upgrading ecosystem and chain contracts.

use clap::Subcommand;
use eyre::bail;
use serde::{Deserialize, Serialize};

use crate::{context::Context, error::Result};

/// Upgrade subcommands for ecosystem and chain contract upgrades.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpgradeCommand {
    /// Upgrade ecosystem contracts to a new protocol version
    Ecosystem,
    /// Upgrade chain contracts to match ecosystem version
    Chain,
}

impl UpgradeCommand {
    /// Execute the upgrade subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            UpgradeCommand::Ecosystem => upgrade_ecosystem(context).await,
            UpgradeCommand::Chain => upgrade_chain(context).await,
        }
    }
}

/// Upgrade ecosystem contracts to a new protocol version.
///
/// This is a placeholder that will be implemented in US4 tasks (T074-T084).
async fn upgrade_ecosystem(_context: &Context) -> Result<()> {
    bail!("upgrade ecosystem command not yet implemented")
}

/// Upgrade chain contracts to match ecosystem version.
///
/// This is a placeholder that will be implemented in US5 tasks (T085-T089).
async fn upgrade_chain(_context: &Context) -> Result<()> {
    bail!("upgrade chain command not yet implemented")
}
