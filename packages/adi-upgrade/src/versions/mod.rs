//! Version-specific upgrade handlers.
//!
//! Each protocol version may have different upgrade scripts and post-hooks.

mod v0_30;

use adi_toolkit::ProtocolVersion;

/// Post-upgrade hook to run after governance execution.
#[derive(Debug, Clone)]
pub enum PostUpgradeHook {
    /// Setup DAValidator pair (v0.30.0 specific)
    DaValidatorSetup,
}

/// Handler for version-specific upgrade logic.
pub trait VersionHandler: Send + Sync {
    /// Forge script path for this version's upgrade.
    fn upgrade_script(&self) -> &str;

    /// Post-upgrade hooks to run after governance execution.
    fn post_upgrade_hooks(&self) -> Vec<PostUpgradeHook>;
}

/// Get the appropriate handler for a protocol version.
///
/// # Returns
///
/// `Some(handler)` if the version is supported, `None` otherwise.
#[must_use]
pub fn get_handler(version: &ProtocolVersion) -> Option<Box<dyn VersionHandler>> {
    match version {
        ProtocolVersion::V0_30_1 => Some(Box::new(v0_30::V0_30_1Handler)),
    }
}

/// Check if a protocol version is supported for upgrades.
#[must_use]
pub fn is_supported(version: &ProtocolVersion) -> bool {
    get_handler(version).is_some()
}
