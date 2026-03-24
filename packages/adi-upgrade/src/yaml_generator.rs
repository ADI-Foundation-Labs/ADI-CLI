//! Upgrade YAML output generator.
//!
//! Replaces the TypeScript `upgrade-yaml-output-generator` script.
//! Reads forge TOML output and broadcast JSON, merges transaction
//! hashes, and writes the combined result as YAML.

use std::path::Path;

use crate::error::{Result, UpgradeError};

/// Generate upgrade YAML from forge TOML output and broadcast JSON.
///
/// Reads the TOML upgrade output, extracts transaction hashes from
/// the broadcast JSON, merges them, and writes the result as YAML.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if files cannot be read, parsed,
/// or if the output cannot be written.
pub fn generate_upgrade_yaml_from_files(
    toml_path: &Path,
    broadcast_json_path: &Path,
    yaml_output_path: &Path,
) -> Result<()> {
    log::info!("Generating upgrade YAML from TOML and broadcast output");

    // Read and parse TOML output
    let toml_content = std::fs::read_to_string(toml_path).map_err(|e| {
        UpgradeError::Config(format!(
            "Failed to read TOML output at {}: {e}",
            toml_path.display()
        ))
    })?;
    let mut toml_value: toml::Value = toml::from_str(&toml_content)
        .map_err(|e| UpgradeError::Config(format!("Failed to parse TOML output: {e}")))?;

    // Read and parse broadcast JSON, extract transaction hashes
    let json_content = std::fs::read_to_string(broadcast_json_path).map_err(|e| {
        UpgradeError::Config(format!(
            "Failed to read broadcast JSON at {}: {e}",
            broadcast_json_path.display()
        ))
    })?;
    let json_value: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| UpgradeError::Config(format!("Failed to parse broadcast JSON: {e}")))?;

    let tx_hashes = extract_transaction_hashes(&json_value)?;
    log::info!(
        "Extracted {} transaction hashes from broadcast",
        tx_hashes.len()
    );

    // Add transactions array to TOML value
    let hashes_array: Vec<toml::Value> = tx_hashes
        .into_iter()
        .map(|h| toml::Value::String(h.to_string()))
        .collect();

    if let toml::Value::Table(ref mut table) = toml_value {
        table.insert("transactions".to_string(), toml::Value::Array(hashes_array));
    }

    // Convert TOML value to serde_yaml value and write
    let yaml_value = toml_to_yaml_value(&toml_value);
    let yaml_output = serde_yaml::to_string(&yaml_value)
        .map_err(|e| UpgradeError::Config(format!("Failed to serialize YAML: {e}")))?;

    // Ensure parent directory exists
    if let Some(parent) = yaml_output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            UpgradeError::Config(format!(
                "Failed to create output directory {}: {e}",
                parent.display()
            ))
        })?;
    }

    std::fs::write(yaml_output_path, &yaml_output).map_err(|e| {
        UpgradeError::Config(format!(
            "Failed to write YAML output to {}: {e}",
            yaml_output_path.display()
        ))
    })?;

    log::info!("Upgrade YAML written to {}", yaml_output_path.display());
    Ok(())
}

/// Extract transaction hashes from broadcast JSON.
fn extract_transaction_hashes(json: &serde_json::Value) -> Result<Vec<&str>> {
    let transactions = json
        .get("transactions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            UpgradeError::Config("Broadcast JSON missing 'transactions' array".to_string())
        })?;

    let mut hashes = Vec::with_capacity(transactions.len());
    for tx in transactions {
        let hash = tx
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| UpgradeError::Config("Transaction missing 'hash' field".to_string()))?;
        hashes.push(hash);
    }
    Ok(hashes)
}

/// Convert a TOML value to a serde_yaml value.
fn toml_to_yaml_value(toml: &toml::Value) -> serde_yaml::Value {
    match toml {
        toml::Value::String(s) => serde_yaml::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_yaml::Value::Number(serde_yaml::Number::from(*i)),
        toml::Value::Float(f) => serde_yaml::Value::Number(serde_yaml::Number::from(*f)),
        toml::Value::Boolean(b) => serde_yaml::Value::Bool(*b),
        toml::Value::Datetime(d) => serde_yaml::Value::String(d.to_string()),
        toml::Value::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(toml_to_yaml_value).collect())
        }
        toml::Value::Table(table) => {
            let map = table
                .iter()
                .map(|(k, v)| (serde_yaml::Value::String(k.clone()), toml_to_yaml_value(v)))
                .collect();
            serde_yaml::Value::Mapping(map)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_transaction_hashes() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
            "transactions": [
                {"hash": "0xabc123"},
                {"hash": "0xdef456"}
            ]
        }"#,
        )
        .unwrap();

        let hashes = extract_transaction_hashes(&json).unwrap();
        assert_eq!(hashes, vec!["0xabc123", "0xdef456"]);
    }

    #[test]
    fn test_toml_to_yaml_roundtrip() {
        let toml_str = r#"
            name = "test"
            value = 42

            [section]
            key = "val"
        "#;
        let toml_val: toml::Value = toml::from_str(toml_str).unwrap();
        let yaml_val = toml_to_yaml_value(&toml_val);
        let yaml_str = serde_yaml::to_string(&yaml_val).unwrap();

        assert!(yaml_str.contains("name: test"));
        assert!(yaml_str.contains("value: 42"));
        assert!(yaml_str.contains("key: val"));
    }
}
