//! Deployment file scanning and logging.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::ui;

/// Config files that CLI actively parses and uses at ecosystem level.
const KNOWN_ECOSYSTEM_FILES: &[&str] = &[
    "ZkStack.yaml",
    "configs/contracts.yaml",
    "configs/wallets.yaml",
    "configs/initial_deployments.yaml",
    "configs/apps/portal.config.json",
];

/// Config files that CLI actively parses and uses at chain level.
const KNOWN_CHAIN_FILES: &[&str] = &[
    "ZkStack.yaml",
    "configs/contracts.yaml",
    "configs/wallets.yaml",
    "configs/genesis.yaml",
    "configs/genesis.json",
    "configs/general.yaml",
    "configs/secrets.yaml",
    "configs/external_node.yaml",
];

/// Log all deployment files and warn about unhandled ones.
///
/// Scans the state directory for config files (yaml, yml, json) and logs:
/// - files that CLI actively parses as known
/// - files that are saved but not processed by CLI as unknown
pub fn log_deployment_files(state_path: &Path, chain_name: &str) -> Result<()> {
    let mut known_files = Vec::new();
    let mut unknown_files = Vec::new();

    for path in collect_files(state_path)
        .into_iter()
        .filter(|p| is_config_file(p))
    {
        let Ok(relative) = path.strip_prefix(state_path) else {
            continue;
        };
        let rel_str = relative.to_string_lossy();

        let chain_prefix = format!("chains/{}/", chain_name);
        let is_known = if let Some(chain_rel) = rel_str.strip_prefix(&chain_prefix) {
            KNOWN_CHAIN_FILES.contains(&chain_rel)
        } else {
            KNOWN_ECOSYSTEM_FILES.contains(&rel_str.as_ref())
        };

        if is_known {
            known_files.push(relative.display().to_string());
        } else {
            unknown_files.push(relative.display().to_string());
        }
    }

    let mut content = known_files.join("\n");
    if !unknown_files.is_empty() {
        if !content.is_empty() {
            content.push_str("\n\nNot processed by CLI:\n");
        }
        content.push_str(&unknown_files.join("\n"));
    }

    ui::note("Deployment files", content)?;
    Ok(())
}

/// Recursively collect all file paths from a directory.
fn collect_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path));
        } else if path.is_file() {
            files.push(path);
        }
    }
    files
}

/// Check if file is a config file (yaml, yml, json).
fn is_config_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("yaml" | "yml" | "json")
    )
}
