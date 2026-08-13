//! Operation bundles surfaced in output-only mode (`--safe` / `--calldata`).
//!
//! In manual mode the CLI never broadcasts; each operation instead yields the
//! transactions plus the address that must execute them, for the operator to run
//! in their Safe or wallet.

use std::path::Path;
use std::str::FromStr;

use alloy_primitives::{Address, Bytes, U256};
use serde::Deserialize;

use crate::error::{io_ctx, Result, UpgradeError};

/// One transaction in a bundle: `to` / `value` / `data`.
#[derive(Debug, Clone)]
pub struct BundleTx {
    /// Call target.
    pub to: Address,
    /// Wei value.
    pub value: U256,
    /// Calldata.
    pub data: Bytes,
}

/// One operation the operator must execute out-of-band.
#[derive(Debug, Clone)]
pub struct BundleInfo {
    /// Human label (e.g. `ecosystem.upgrade-prepare-all`).
    pub label: String,
    /// Address that must sign/execute this bundle.
    pub target: Address,
    /// The transactions.
    pub txs: Vec<BundleTx>,
    /// Canonical Safe Transaction Builder file emitted by protocol_ops, if any.
    /// Native bundles have `None` and synthesize their JSON.
    pub safe_json_path: Option<std::path::PathBuf>,
}

impl BundleInfo {
    /// Safe Transaction Builder JSON: the on-disk protocol_ops file when present,
    /// else synthesized from `txs`.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::Config`] if the on-disk file cannot be read.
    pub fn safe_json(&self, chain_id: u64) -> Result<String> {
        if let Some(path) = &self.safe_json_path {
            return std::fs::read_to_string(path).map_err(io_ctx("read", path));
        }
        Ok(synth_safe_json(chain_id, &self.label, &self.txs))
    }
}

/// Synthesize a Safe Transaction Builder bundle from raw txs.
#[must_use]
pub fn synth_safe_json(chain_id: u64, name: &str, txs: &[BundleTx]) -> String {
    let transactions: Vec<_> = txs
        .iter()
        .map(|t| {
            serde_json::json!({
                "to": t.to.to_checksum(None),
                "value": t.value.to_string(),
                "data": t.data.to_string(),
                "contractMethod": serde_json::Value::Null,
                "contractInputsValues": serde_json::Value::Null,
            })
        })
        .collect();
    serde_json::json!({
        "chainId": chain_id.to_string(),
        "meta": { "name": name },
        "transactions": transactions,
        "version": "1.0",
    })
    .to_string()
}

#[derive(Deserialize)]
struct Manifest {
    bundles: Vec<ManifestBundle>,
}

#[derive(Deserialize)]
struct ManifestBundle {
    file: String,
    target: String,
    #[serde(default)]
    steps: Vec<String>,
}

#[derive(Deserialize)]
struct SafeFile {
    transactions: Vec<SafeTx>,
}

#[derive(Deserialize)]
struct SafeTx {
    to: String,
    value: String,
    data: String,
}

/// Read a protocol_ops `manifest.json` and its Safe files into bundles.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the manifest or a Safe file is missing or
/// malformed.
pub fn read_bundles(manifest_path: &Path) -> Result<Vec<BundleInfo>> {
    let raw = std::fs::read_to_string(manifest_path).map_err(io_ctx("read", manifest_path))?;
    let manifest: Manifest = serde_json::from_str(&raw)
        .map_err(|e| UpgradeError::Config(format!("parse manifest: {e}")))?;
    let dir = manifest_path.parent().unwrap_or(Path::new("."));

    manifest
        .bundles
        .into_iter()
        .map(|b| {
            let target = Address::from_str(&b.target)
                .map_err(|e| UpgradeError::Config(format!("bad target {}: {e}", b.target)))?;
            let file = dir.join(&b.file);
            let safe_raw = std::fs::read_to_string(&file).map_err(io_ctx("read", &file))?;
            let safe: SafeFile = serde_json::from_str(&safe_raw)
                .map_err(|e| UpgradeError::Config(format!("parse {}: {e}", file.display())))?;
            let txs = safe
                .transactions
                .iter()
                .map(parse_tx)
                .collect::<Result<Vec<_>>>()?;
            Ok(BundleInfo {
                label: b.steps.into_iter().next().unwrap_or(b.file),
                target,
                txs,
                safe_json_path: Some(file),
            })
        })
        .collect()
}

fn parse_tx(t: &SafeTx) -> Result<BundleTx> {
    Ok(BundleTx {
        to: Address::from_str(&t.to)
            .map_err(|e| UpgradeError::Config(format!("bad to {}: {e}", t.to)))?,
        value: U256::from_str(&t.value)
            .map_err(|e| UpgradeError::Config(format!("bad value {}: {e}", t.value)))?,
        data: Bytes::from_str(&t.data)
            .map_err(|e| UpgradeError::Config(format!("bad data: {e}")))?,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn synthesized_safe_json_carries_txs() {
        let bundle = BundleInfo {
            label: "ownership: transfer".to_string(),
            target: "0xbbd9e63ec7dafdf93ab0df3e544f0b6c99c13439"
                .parse()
                .unwrap(),
            txs: vec![BundleTx {
                to: "0x12a71836e0Fe9434389bbFDaEB8Ea497D0593BD6"
                    .parse()
                    .unwrap(),
                value: U256::ZERO,
                data: Bytes::from_str("0xf2fde38b").unwrap(),
            }],
            safe_json_path: None,
        };
        let json: serde_json::Value =
            serde_json::from_str(&bundle.safe_json(6565).unwrap()).unwrap();
        assert_eq!(json["chainId"], "6565");
        assert_eq!(json["transactions"][0]["data"], "0xf2fde38b");
        assert_eq!(json["transactions"][0]["value"], "0");
        assert_eq!(
            json["transactions"][0]["to"],
            "0x12a71836e0Fe9434389bbFDaEB8Ea497D0593BD6"
        );
    }

    #[test]
    fn read_bundles_parses_manifest_and_safe_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("01.safe.json"),
            r#"{"transactions":[{"to":"0x12a71836e0Fe9434389bbFDaEB8Ea497D0593BD6","value":"0","data":"0xf2fde38b"}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("manifest.json"),
            r#"{"bundles":[{"file":"01.safe.json","target":"0xbbd9e63ec7dafdf93ab0df3e544f0b6c99c13439","steps":["ownership: transfer"]}]}"#,
        )
        .unwrap();
        let bundles = read_bundles(&dir.path().join("manifest.json")).unwrap();
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].label, "ownership: transfer");
        assert_eq!(bundles[0].txs.len(), 1);
        assert_eq!(bundles[0].txs[0].value, U256::ZERO);
        assert_eq!(
            bundles[0].txs[0].data,
            Bytes::from_str("0xf2fde38b").unwrap()
        );
        assert!(bundles[0].safe_json_path.is_some());
    }

    #[test]
    fn read_bundles_errors_on_bad_target() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("01.safe.json"), r#"{"transactions":[]}"#).unwrap();
        std::fs::write(
            dir.path().join("manifest.json"),
            r#"{"bundles":[{"file":"01.safe.json","target":"not-an-address"}]}"#,
        )
        .unwrap();
        assert!(read_bundles(&dir.path().join("manifest.json")).is_err());
    }
}
