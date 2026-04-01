//! Contract address types for ecosystem and chain deployments.

mod bridge;
mod chain;
mod ecosystem;

// Re-export bridge types
pub use bridge::{BridgeContracts, BridgesConfig};

// Re-export chain types
pub use chain::{ChainContracts, ChainEcosystemContracts, ChainL1Contracts, ChainL2Contracts};

// Re-export ecosystem types
pub use ecosystem::{CoreEcosystemContracts, EcosystemContracts, L1Contracts, ZkSyncOsCtm};
