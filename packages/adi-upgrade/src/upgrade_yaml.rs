//! Upgrade YAML file management.
//!
//! Handles loading previous upgrade outputs and saving new ones.

use std::path::{Path, PathBuf};

use alloy_primitives::Address;

use crate::chain_toml::PreviousUpgradeValues;
use crate::error::{Result, UpgradeError};

/// Directory name within state for storing upgrade outputs.
const UPGRADES_DIR: &str = "upgrades";

/// Load previous upgrade YAML and extract `[state_transition]` values.
///
/// Searches for the file in:
/// 1. Explicit path (if provided via `--previous-upgrade-yaml`)
/// 2. `state_dir/upgrades/<filename>`
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the explicit path does not exist
/// or if the file cannot be read.
pub fn load_previous_upgrade_values(
    explicit_path: Option<&Path>,
    state_dir: &Path,
    expected_filename: &str,
) -> Result<PreviousUpgradeValues> {
    let path = if let Some(p) = explicit_path {
        if !p.exists() {
            return Err(UpgradeError::Config(format!(
                "Previous upgrade YAML not found: {}",
                p.display()
            )));
        }
        p.to_path_buf()
    } else {
        let auto_path = state_dir.join(UPGRADES_DIR).join(expected_filename);
        if !auto_path.exists() {
            log::info!(
                "No previous upgrade YAML found at {}, using defaults",
                auto_path.display()
            );
            return Ok(PreviousUpgradeValues::default());
        }
        auto_path
    };

    log::info!("Loading previous upgrade values from {}", path.display());
    let content = std::fs::read_to_string(&path)
        .map_err(|e| UpgradeError::Config(format!("Failed to read {}: {e}", path.display())))?;

    Ok(parse_upgrade_yaml(&content))
}

/// Save upgrade output YAML to state directory for future upgrades.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if directory creation or file copy fails.
pub fn save_upgrade_yaml(source_path: &Path, state_dir: &Path, filename: &str) -> Result<PathBuf> {
    let upgrades_dir = state_dir.join(UPGRADES_DIR);
    std::fs::create_dir_all(&upgrades_dir)
        .map_err(|e| UpgradeError::Config(format!("Failed to create upgrades dir: {e}")))?;

    let dest = upgrades_dir.join(filename);
    std::fs::copy(source_path, &dest).map_err(|e| {
        UpgradeError::Config(format!("Failed to copy YAML to {}: {e}", dest.display()))
    })?;

    log::info!("Saved upgrade YAML to {}", dest.display());
    Ok(dest)
}

/// Parse YAML content to extract `state_transition` values.
fn parse_upgrade_yaml(content: &str) -> PreviousUpgradeValues {
    PreviousUpgradeValues {
        admin_facet_addr: extract_yaml_address(content, "admin_facet_addr"),
        diamond_init_addr: extract_yaml_address(content, "diamond_init_addr"),
        executor_facet_addr: extract_yaml_address(content, "executor_facet_addr"),
        genesis_upgrade_addr: extract_yaml_address(content, "genesis_upgrade_addr"),
        getters_facet_addr: extract_yaml_address(content, "getters_facet_addr"),
        mailbox_facet_addr: extract_yaml_address(content, "mailbox_facet_addr"),
        force_deployments_data: extract_yaml_hex(content, "force_deployments_data"),
    }
}

/// Extract an address value from YAML (skips zero addresses).
fn extract_yaml_address(content: &str, key: &str) -> Option<Address> {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.contains(key) {
            continue;
        }
        if let Some(addr_hex) = extract_hex_from_line(trimmed) {
            // Skip zero addresses
            if addr_hex.trim_start_matches("0x").chars().all(|c| c == '0') {
                continue;
            }
            if let Ok(addr) = addr_hex.parse::<Address>() {
                return Some(addr);
            }
        }
    }
    None
}

/// Extract a hex value (possibly long) from YAML.
fn extract_yaml_hex(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.contains(key) {
            continue;
        }
        return extract_hex_from_line(trimmed);
    }
    None
}

/// Extract 0x-prefixed hex string from a YAML line.
fn extract_hex_from_line(line: &str) -> Option<String> {
    let start = line.find("0x")?;
    let hex_part: String = line
        .get(start..)?
        .chars()
        .take_while(|c| c.is_ascii_hexdigit() || *c == 'x')
        .collect();

    if hex_part.len() > 2 {
        Some(hex_part)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn test_extract_hex_from_line() {
        assert_eq!(
            extract_hex_from_line("admin_facet_addr: \"0xABCDEF1234\""),
            Some("0xABCDEF1234".to_string())
        );
        assert_eq!(extract_hex_from_line("no hex here"), None);
        assert_eq!(extract_hex_from_line("empty: 0x"), None);
    }

    #[test]
    fn test_extract_yaml_address_skips_zero() {
        let content = "admin_facet_addr: \"0x0000000000000000000000000000000000000000\"";
        assert_eq!(extract_yaml_address(content, "admin_facet_addr"), None);
    }

    #[test]
    fn test_extract_yaml_address_returns_nonzero() {
        let content = "admin_facet_addr: \"0x1111111111111111111111111111111111111111\"";
        assert_eq!(
            extract_yaml_address(content, "admin_facet_addr"),
            Some(address!("1111111111111111111111111111111111111111"))
        );
    }

    #[test]
    fn test_parse_upgrade_yaml() {
        let yaml = "\
admin_facet_addr: \"0x1111111111111111111111111111111111111111\"
diamond_init_addr: \"0x2222222222222222222222222222222222222222\"
force_deployments_data: \"0xabcdef\"
";
        let values = parse_upgrade_yaml(yaml);
        assert!(values.admin_facet_addr.is_some());
        assert!(values.diamond_init_addr.is_some());
        assert!(values.force_deployments_data.is_some());
        assert!(values.executor_facet_addr.is_none());
    }
}
