//! Accept ownership commands for ecosystem and chain contracts.

mod ownership;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Subcommands for the `accept` command.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AcceptCommand {
    /// Accept pending ownership transfers for deployed contracts.
    ///
    /// After ecosystem deployment, some contracts may have pending ownership
    /// transfers that need to be accepted. This command handles:
    /// - Server Notifier (via multicall)
    /// - Validator Timelock (direct call)
    /// - Verifier (direct call)
    ///
    /// Failures are logged but do not stop the overall process.
    Ownership(ownership::OwnershipAcceptArgs),
}

impl AcceptCommand {
    /// Execute the accept subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            AcceptCommand::Ownership(args) => ownership::run(args, context).await,
        }
    }
}
