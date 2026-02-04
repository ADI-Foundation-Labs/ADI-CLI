//! Deployment commands for ecosystem and chain contracts.
//!
//! # Wallet Roles
//!
//! Deployment uses specialized wallets, each with a specific purpose:
//!
//! - **Deployer**: Deploys smart contracts to the settlement layer (L1)
//! - **Governor**: Manages governance operations and contract upgrades
//! - **Operator**: Commits batches to L1
//! - **Prove Operator**: Submits validity proofs to L1
//! - **Execute Operator**: Executes verified batches on L1
//! - **Funder**: User-provided wallet that funds all other wallets

mod ecosystem;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Subcommands for the `deploy` command.
#[derive(Clone, Subcommand, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeployCommand {
    /// Fund wallets and deploy ecosystem contracts (bridgehub, governance, etc.)
    Ecosystem(ecosystem::EcosystemDeployArgs),
}

impl DeployCommand {
    /// Execute the deploy subcommand.
    pub async fn run(self, context: &Context) -> Result<()> {
        match self {
            DeployCommand::Ecosystem(args) => ecosystem::run(args, context).await,
        }
    }
}
