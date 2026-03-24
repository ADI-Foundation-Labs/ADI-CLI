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

    /// Directory for upgrade environment config (e.g., "upgrade-envs/v0.30.1-airbender-fix").
    fn upgrade_env_dir(&self) -> &str;

    /// Output TOML filename from forge script.
    fn upgrade_output_toml(&self) -> &str;

    /// Output YAML filename (the ecosystem upgrade output).
    fn upgrade_output_yaml(&self) -> &str;

    /// Upgrade name for zkstack (e.g., "v30-zk-sync-os-blobs").
    fn upgrade_name(&self) -> &str;

    /// Old protocol version hex for chain.toml (e.g., "0x1e00000000").
    fn old_protocol_version_hex(&self) -> &str;

    /// Previous version's upgrade YAML filename to load state_transition values from.
    fn previous_upgrade_yaml(&self) -> &str;

    /// Post-upgrade hooks to run after governance execution.
    fn post_upgrade_hooks(&self) -> Vec<PostUpgradeHook>;

    /// Return version-specific chain.toml defaults as a typed config.
    fn chain_toml_defaults(&self) -> crate::chain_toml::ChainTomlConfig;
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
