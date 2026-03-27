//! Default configuration types for CLI.
//!
//! These types define the structure of the CLI configuration file (`~/.adi.yml`).
//! They are separate from the domain types (`EcosystemConfig`, `ChainConfig`) which
//! are used for building zkstack commands.

mod chain;
mod ecosystem;

pub use chain::{ChainDefaults, ChainFundingDefaults, ChainOwnershipDefaults, OperatorsDefaults};
pub use ecosystem::{EcosystemDefaults, EcosystemOwnershipDefaults};
