//! ZkStack.yaml path validation and auto-repair.
//!
//! Checks that paths in ZkStack.yaml files match the actual state directory
//! and offers to fix stale paths with user confirmation.

use std::path::Path;

use adi_state::StateManager;
use adi_types::{PartialChainMetadata, PartialEcosystemMetadata};

use crate::error::{Result, WrapErr};
use crate::ui;

/// A detected path mismatch in ZkStack.yaml.
struct PathMismatch {
    /// File containing the mismatch (e.g., "ZkStack.yaml" or "chains/adi/ZkStack.yaml").
    file: String,
    /// Field name (e.g., "chains", "configs").
    field: String,
    /// Current (stale) value.
    current: String,
    /// Expected (correct) value.
    expected: String,
}

/// Validate ZkStack.yaml paths and prompt user to fix mismatches.
///
/// Called after state manager creation in commands that use ecosystem state.
/// If all paths are correct, returns silently. If mismatches are found,
/// shows a preview and prompts the user to confirm the fix.
///
/// # Errors
///
/// Returns error if state cannot be read or updated.
pub async fn validate_and_fix_state_paths(
    state_manager: &StateManager,
    state_dir: &Path,
) -> Result<()> {
    if !state_manager.exists().await.unwrap_or(false) {
        return Ok(());
    }

    let mismatches = collect_mismatches(state_manager, state_dir).await?;
    if mismatches.is_empty() {
        return Ok(());
    }

    // Show preview
    ui::warning(format!(
        "Found {} stale path(s) in ZkStack.yaml files",
        mismatches.len()
    ))?;

    let mut preview = String::new();
    for m in &mismatches {
        preview.push_str(&format!(
            "{} → {}\n  {} → {}\n",
            m.file, m.field, m.current, m.expected
        ));
    }
    ui::note("Path changes", &preview)?;

    let confirmed = ui::confirm("Fix stale paths in ZkStack.yaml?")
        .initial_value(true)
        .interact()
        .wrap_err("Prompt cancelled")?;

    if !confirmed {
        ui::warning("Skipping path fix — commands may fail with stale paths")?;
        return Ok(());
    }

    apply_fixes(state_manager, state_dir, &mismatches).await?;
    ui::success("ZkStack.yaml paths updated")?;
    Ok(())
}

/// Container mount point for the state directory.
const WORKSPACE: &str = "/workspace";
/// Container path to era-contracts source code.
const DEPS_ZKSYNC_ERA: &str = "/deps/zksync-era";

/// Collect all path mismatches from ecosystem and chain ZkStack.yaml files.
async fn collect_mismatches(
    state_manager: &StateManager,
    _state_dir: &Path,
) -> Result<Vec<PathMismatch>> {
    let mut mismatches = Vec::new();

    // Check ecosystem metadata
    let eco_meta = state_manager
        .ecosystem()
        .metadata()
        .await
        .wrap_err("Failed to read ecosystem metadata")?;

    check_field(
        &mut mismatches,
        "ZkStack.yaml",
        "link_to_code",
        &eco_meta.link_to_code,
        DEPS_ZKSYNC_ERA,
    );
    check_field(
        &mut mismatches,
        "ZkStack.yaml",
        "chains",
        &eco_meta.chains,
        &format!("{WORKSPACE}/chains"),
    );
    check_field(
        &mut mismatches,
        "ZkStack.yaml",
        "config",
        &eco_meta.config,
        &format!("{WORKSPACE}/configs/"),
    );

    // Check chain metadata
    let chains = state_manager
        .list_chains()
        .await
        .wrap_err("Failed to list chains")?;

    for chain_name in &chains {
        let chain_meta = state_manager
            .chain(chain_name)
            .metadata()
            .await
            .wrap_err(format!("Failed to read chain '{chain_name}' metadata"))?;

        let chain_dir = format!("{WORKSPACE}/chains/{chain_name}");
        let file_label = format!("chains/{chain_name}/ZkStack.yaml");

        check_field(
            &mut mismatches,
            &file_label,
            "link_to_code",
            &chain_meta.link_to_code,
            DEPS_ZKSYNC_ERA,
        );
        check_field(
            &mut mismatches,
            &file_label,
            "configs",
            &chain_meta.configs,
            &format!("{chain_dir}/configs/"),
        );
        check_field(
            &mut mismatches,
            &file_label,
            "rocks_db_path",
            &chain_meta.rocks_db_path,
            &format!("{chain_dir}/db/"),
        );
        check_field(
            &mut mismatches,
            &file_label,
            "artifacts_path",
            &chain_meta.artifacts_path,
            &format!("{chain_dir}/artifacts/"),
        );
        check_field(
            &mut mismatches,
            &file_label,
            "contracts_path",
            &chain_meta.contracts_path,
            &format!("{DEPS_ZKSYNC_ERA}/contracts/"),
        );
        check_field(
            &mut mismatches,
            &file_label,
            "default_configs_path",
            &chain_meta.default_configs_path,
            &format!("{DEPS_ZKSYNC_ERA}/etc/env/file_based"),
        );
    }

    Ok(mismatches)
}

/// Check a single field and push a mismatch if values differ.
fn check_field(
    mismatches: &mut Vec<PathMismatch>,
    file: &str,
    field: &str,
    current: &str,
    expected: &str,
) {
    if normalize_path(current) != normalize_path(expected) {
        mismatches.push(PathMismatch {
            file: file.to_string(),
            field: field.to_string(),
            current: current.to_string(),
            expected: expected.to_string(),
        });
    }
}

/// Apply path fixes to state via partial metadata updates.
async fn apply_fixes(
    state_manager: &StateManager,
    _state_dir: &Path,
    mismatches: &[PathMismatch],
) -> Result<()> {
    // Collect ecosystem-level fixes
    let mut eco_partial = PartialEcosystemMetadata::default();
    let mut eco_changed = false;

    for m in mismatches {
        if m.file == "ZkStack.yaml" {
            match m.field.as_str() {
                "link_to_code" => eco_partial.link_to_code = Some(m.expected.clone()),
                "chains" => eco_partial.chains = Some(m.expected.clone()),
                "config" => eco_partial.config = Some(m.expected.clone()),
                _ => {}
            }
            eco_changed = true;
        }
    }

    if eco_changed {
        state_manager
            .ecosystem()
            .update_metadata(&eco_partial)
            .await
            .wrap_err("Failed to update ecosystem metadata")?;
    }

    // Collect chain-level fixes per chain
    let chains = state_manager
        .list_chains()
        .await
        .wrap_err("Failed to list chains")?;

    for chain_name in &chains {
        let file_label = format!("chains/{chain_name}/ZkStack.yaml");
        let mut chain_partial = PartialChainMetadata::default();
        let mut chain_changed = false;

        for m in mismatches {
            if m.file == file_label {
                match m.field.as_str() {
                    "link_to_code" => chain_partial.link_to_code = Some(m.expected.clone()),
                    "configs" => chain_partial.configs = Some(m.expected.clone()),
                    "rocks_db_path" => chain_partial.rocks_db_path = Some(m.expected.clone()),
                    "artifacts_path" => chain_partial.artifacts_path = Some(m.expected.clone()),
                    "contracts_path" => chain_partial.contracts_path = Some(m.expected.clone()),
                    "default_configs_path" => {
                        chain_partial.default_configs_path = Some(m.expected.clone());
                    }
                    _ => {}
                }
                chain_changed = true;
            }
        }

        if chain_changed {
            state_manager
                .chain(chain_name)
                .update_metadata(&chain_partial)
                .await
                .wrap_err(format!("Failed to update chain '{chain_name}' metadata"))?;
        }
    }

    Ok(())
}

/// Normalize a path string for comparison (remove trailing slashes, `./` prefixes).
fn normalize_path(path: &str) -> String {
    path.replace("/./", "/").trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_removes_trailing_slash() {
        assert_eq!(normalize_path("/workspace/chains/"), "/workspace/chains");
    }

    #[test]
    fn normalize_path_removes_dot_segments() {
        assert_eq!(normalize_path("/workspace/./chains"), "/workspace/chains");
    }

    #[test]
    fn normalize_path_clean_path_unchanged() {
        assert_eq!(normalize_path("/workspace/chains"), "/workspace/chains");
    }

    #[test]
    fn normalize_path_multiple_trailing_slashes() {
        // trim_end_matches removes all trailing slashes
        assert_eq!(normalize_path("/workspace///"), "/workspace");
    }

    #[test]
    fn normalize_path_empty_string() {
        assert_eq!(normalize_path(""), "");
    }
}
