//! Deployment command for ecosystem and chain contracts.
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

pub use ecosystem::{run, DeployArgs};
