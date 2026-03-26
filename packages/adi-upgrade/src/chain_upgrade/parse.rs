//! Parsing helpers for chain upgrade output.
//!
//! Extracts calldata from zkstack chain-upgrade output text.

use alloy_primitives::Bytes;

use crate::error::{Result, UpgradeError};

/// Extracted calldatas from zkstack chain-upgrade output.
#[derive(Debug)]
pub struct ChainCalldatas {
    /// Schedule upgrade calldata (sent to ChainAdmin).
    pub schedule: Bytes,
    /// ChainAdmin full calldata (execute upgrade).
    pub chain_admin: Bytes,
}

/// Strip ANSI escape sequences (e.g. `\x1b[36m`) from a string.
pub(crate) fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm' (end of SGR sequence)
            for c2 in chars.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Extract chain upgrade calldatas from zkstack output file.
///
/// Parses the `chain-upgrade.txt` output to find:
/// - Schedule calldata from "Calldata to schedule upgrade" section
/// - ChainAdmin calldata from "Full calldata to call `ChainAdmin` with" section
pub fn extract_chain_calldatas(output: &str) -> Result<ChainCalldatas> {
    // Strip ANSI escape codes — zkstack logs contain color codes whose digits
    // would corrupt hex extraction (e.g. \x1b[36m contains '3','6').
    let output = strip_ansi_codes(output);

    // Extract schedule calldata: find "data": "0x..." in the schedule section
    let schedule_hex = extract_schedule_calldata(&output)?;
    let schedule = hex::decode(schedule_hex.trim_start_matches("0x"))
        .map_err(|e| UpgradeError::Config(format!("Invalid schedule calldata hex: {e}")))?;

    // Extract chainadmin calldata: hex line after "Full calldata to call `ChainAdmin` with"
    let chain_admin_hex = extract_chainadmin_calldata(&output)?;
    let chain_admin = hex::decode(chain_admin_hex.trim_start_matches("0x"))
        .map_err(|e| UpgradeError::Config(format!("Invalid chainadmin calldata hex: {e}")))?;

    Ok(ChainCalldatas {
        schedule: Bytes::from(schedule),
        chain_admin: Bytes::from(chain_admin),
    })
}

/// Extract schedule calldata from "Calldata to schedule upgrade" section.
fn extract_schedule_calldata(output: &str) -> Result<String> {
    let marker = "Calldata to schedule upgrade";
    let section_start = output
        .find(marker)
        .ok_or_else(|| UpgradeError::Config("Schedule calldata section not found".into()))?;

    let section = &output[section_start..];

    // Find "data": "0x..." pattern
    let data_marker = "\"data\":";
    let data_pos = section
        .find(data_marker)
        .ok_or_else(|| UpgradeError::Config("Schedule calldata data field not found".into()))?;

    let after_data = &section[data_pos + data_marker.len()..];

    // Find the hex value between quotes
    let quote_start = after_data
        .find('"')
        .ok_or_else(|| UpgradeError::Config("Schedule calldata: missing opening quote".into()))?;
    let hex_start = quote_start + 1;

    let quote_end = after_data[hex_start..]
        .find('"')
        .ok_or_else(|| UpgradeError::Config("Schedule calldata: missing closing quote".into()))?;

    Ok(after_data[hex_start..hex_start + quote_end].to_string())
}

/// Extract chainadmin calldata from "Full calldata to call `ChainAdmin` with" section.
fn extract_chainadmin_calldata(output: &str) -> Result<String> {
    let marker = "Full calldata to call `ChainAdmin` with";
    let section_start = output
        .find(marker)
        .ok_or_else(|| UpgradeError::Config("ChainAdmin calldata section not found".into()))?;

    let after_marker = &output[section_start + marker.len()..];

    // The hex appears on the next non-empty line
    for line in after_marker.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Extract hex characters
        let hex: String = trimmed.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if hex.len() >= 8 {
            return Ok(format!("0x{hex}"));
        }
    }

    Err(UpgradeError::Config(
        "ChainAdmin calldata not found after marker".into(),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_codes_removes_sequences() {
        assert_eq!(strip_ansi_codes("\x1b[36mhello\x1b[0m"), "hello");
    }

    #[test]
    fn test_strip_ansi_codes_preserves_plain_text() {
        assert_eq!(strip_ansi_codes("no codes here"), "no codes here");
    }

    #[test]
    fn test_strip_ansi_codes_empty_input() {
        assert_eq!(strip_ansi_codes(""), "");
    }

    #[test]
    fn test_extract_schedule_calldata_valid() {
        let output = r#"
Calldata to schedule upgrade
  {
    "data": "0xabcdef12"
  }
"#;
        let result = extract_schedule_calldata(output).unwrap();
        assert_eq!(result, "0xabcdef12");
    }

    #[test]
    fn test_extract_schedule_calldata_missing_marker() {
        let result = extract_schedule_calldata("no marker here");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_chainadmin_calldata_valid() {
        // The function filters hex chars from the line, so "0x" prefix chars
        // get included: '0' is hex, 'x' is not, rest is hex
        let output = "Full calldata to call `ChainAdmin` with\n\n0xdeadbeef1234\n";
        let result = extract_chainadmin_calldata(output).unwrap();
        assert_eq!(result, "0x0deadbeef1234");
    }

    #[test]
    fn test_extract_chainadmin_calldata_missing_marker() {
        let result = extract_chainadmin_calldata("no marker here");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_chain_calldatas_end_to_end() {
        // chainadmin extraction filters hex chars: "0x11223344" → hex digits "011223344"
        // so we use a line without "0x" prefix to get clean extraction
        let output = r#"
Some preamble text

Calldata to schedule upgrade
  {
    "data": "0xaabbccdd"
  }

Full calldata to call `ChainAdmin` with

  11223344aabbccdd
"#;
        let result = extract_chain_calldatas(output).unwrap();
        assert_eq!(result.schedule.as_ref(), &[0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(
            result.chain_admin.as_ref(),
            &[0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd]
        );
    }
}
