//! Version metadata for v0.31.0 upgrades.

use alloy_primitives::{address, Address};

/// Handler for v0.31.0 upgrades.
pub struct V0_31_0Handler;

impl V0_31_0Handler {
    /// Protocol version being upgraded from (v30.1) as a raw uint.
    #[must_use]
    pub fn old_protocol_version(&self) -> u64 {
        0x1e00000001
    }

    /// Protocol version reached by this upgrade (v31) as a raw uint.
    #[must_use]
    pub fn new_protocol_version(&self) -> u64 {
        0x1f00000000
    }

    /// Deterministic CREATE2 factory used for the upgrade deploys.
    #[must_use]
    pub fn create2_factory(&self) -> Address {
        address!("4e59b44847b379578588920cA78FbF26c0B4956C")
    }

    /// Default governance upgrade-timer delay (0 outside mainnet).
    #[must_use]
    pub fn governance_upgrade_timer_initial_delay(&self) -> u64 {
        0
    }
}
