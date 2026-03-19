//! Version handlers for v0.30.x protocol versions.

use super::{PostUpgradeHook, VersionHandler};

/// Handler for v0.30.1 upgrades.
pub struct V0_30_1Handler;

impl VersionHandler for V0_30_1Handler {
    fn upgrade_script(&self) -> &str {
        "l1-contracts/deploy-scripts/upgrade/EcosystemUpgrade.s.sol"
    }

    fn post_upgrade_hooks(&self) -> Vec<PostUpgradeHook> {
        // v0.30.1 has no post-upgrade hooks
        vec![]
    }
}
