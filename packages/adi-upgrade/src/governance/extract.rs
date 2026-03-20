//! Extract stage1 calls from forge script TOML output.

use crate::error::{Result, UpgradeError};

/// Extract the stage1_calls hex from TOML output content.
///
/// Looks for pattern: `stage1_calls = "0x<hex>"` in the TOML content.
pub fn extract_stage1_calls(toml_content: &str) -> Result<String> {
    for line in toml_content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("stage1_calls") {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let value = trimmed.get(eq_pos + 1..).unwrap_or_default().trim();
            // Strip quotes
            let value = value.trim_matches('"').trim();
            if value.starts_with("0x") || value.starts_with("0X") {
                return Ok(value.to_string());
            }
        }
    }

    Err(UpgradeError::GovernanceFailed(
        "stage1_calls not found in TOML output".to_string(),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_stage1_calls_found() {
        let toml = r#"
[some_section]
stage1_calls = "0xabcdef1234567890"
other_key = "value"
"#;
        let result = extract_stage1_calls(toml).unwrap();
        assert_eq!(result, "0xabcdef1234567890");
    }

    #[test]
    fn test_extract_stage1_calls_not_found() {
        let toml = "other_key = \"value\"";
        let result = extract_stage1_calls(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_stage1_calls_with_spaces() {
        let toml = "  stage1_calls  =  \"0xdeadbeef\"  ";
        let result = extract_stage1_calls(toml).unwrap();
        assert_eq!(result, "0xdeadbeef");
    }
}
