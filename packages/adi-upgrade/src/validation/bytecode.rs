//! Bytecode validation for upgrade outputs.
//!
//! Validates that forge upgrade output contains expected contract bytecode hashes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The marker prefix that precedes a 64-hex-char bytecode hash in forge output.
const HASH_MARKER: &str = "00000060";

/// Expected length of a bytecode hash in hex characters (32 bytes = 64 hex chars).
const HASH_HEX_LEN: usize = 64;

/// Manifest of expected bytecode hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeManifest {
    /// Contract name -> bytecode entry
    #[serde(flatten)]
    pub contracts: HashMap<String, ContractEntry>,
}

/// Entry for a contract in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEntry {
    /// Bytecode hash (without 0x prefix)
    pub bytecode_hash: String,
}

/// Report from bytecode validation.
#[derive(Debug, Default)]
pub struct ValidationReport {
    /// Contract names that were found in the output
    pub found: Vec<String>,

    /// Contract names and hashes that were NOT found
    pub missing: Vec<(String, String)>,

    /// Extra hashes found in output but not in manifest
    pub extra: Vec<String>,
}

impl ValidationReport {
    /// Check if validation passed (no missing hashes).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.missing.is_empty()
    }

    /// Format report as human-readable string.
    #[must_use]
    pub fn format(&self) -> String {
        let mut lines = vec![format!(
            "Found {}/{} expected hashes",
            self.found.len(),
            self.found.len() + self.missing.len()
        )];

        if !self.missing.is_empty() {
            lines.push(format!("\nMissing {} hashes:", self.missing.len()));
            for (name, hash) in &self.missing {
                lines.push(format!("  - {}: {}", name, hash));
            }
        }

        if !self.extra.is_empty() {
            lines.push(format!("\nExtra {} hashes found:", self.extra.len()));
            for hash in &self.extra {
                lines.push(format!("  - {}", hash));
            }
        }

        lines.join("\n")
    }
}

/// Extract all 64-hex-char bytecode hashes that follow the `00000060` marker.
///
/// Returns lowercase hex strings without the marker prefix.
fn extract_hashes(text: &str) -> Vec<String> {
    let mut hashes = Vec::new();
    let mut search_from = 0;

    while let Some(marker_pos) = text[search_from..].find(HASH_MARKER) {
        let abs_pos = search_from + marker_pos + HASH_MARKER.len();
        let candidate = text.get(abs_pos..abs_pos + HASH_HEX_LEN).unwrap_or("");

        if candidate.len() == HASH_HEX_LEN && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            hashes.push(candidate.to_lowercase());
        }

        search_from = search_from + marker_pos + 1;
    }

    hashes
}

/// Validate upgrade YAML output against expected bytecode hashes.
///
/// # Arguments
///
/// * `upgrade_yaml` - Contents of the forge upgrade output YAML
/// * `manifest` - Expected bytecode manifest
pub fn validate_upgrade_output(
    upgrade_yaml: &str,
    manifest: &BytecodeManifest,
) -> ValidationReport {
    let mut report = ValidationReport::default();
    let yaml_lower = upgrade_yaml.to_lowercase();

    // Check each expected hash
    for (contract_name, entry) in &manifest.contracts {
        let hash = entry.bytecode_hash.trim_start_matches("0x").to_lowercase();

        if yaml_lower.contains(&hash) {
            report.found.push(contract_name.clone());
        } else {
            report.missing.push((contract_name.clone(), hash));
        }
    }

    // Extract 00000060<hash> patterns and find unknowns
    let known_hashes: std::collections::HashSet<_> = manifest
        .contracts
        .values()
        .map(|e| e.bytecode_hash.trim_start_matches("0x").to_lowercase())
        .collect();

    for hash in extract_hashes(upgrade_yaml) {
        // Skip noise patterns
        if hash.starts_with("c37bb1bc") {
            continue;
        }
        if hash.chars().take(24).all(|c| c == '0') {
            continue;
        }

        if !known_hashes.contains(&hash) {
            report.extra.push(hash);
        }
    }

    report.extra.sort();
    report.extra.dedup();

    report
}
