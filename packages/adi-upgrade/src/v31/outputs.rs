//! Read protocol_ops output artifacts back from the state dir (host side).

use std::path::{Path, PathBuf};

use crate::error::{io_ctx, Result, UpgradeError};
use crate::v31::env_toml::V31_ENV_DIR;

fn output_dir(state_dir: &Path, env: &str) -> PathBuf {
    state_dir
        .join("l1-contracts/upgrade-envs")
        .join(V31_ENV_DIR)
        .join("output")
        .join(env)
}

/// The finalize (`upgradeChainFromVersion`) tx hash from the chain-upgrade
/// broadcast — the total-supply calc needs it as `--upgrade-l1-tx`.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the file is missing or has no tx hash.
pub fn finalize_tx_hash(state_dir: &Path, env: &str, chain_id: u64) -> Result<String> {
    let path = output_dir(state_dir, env)
        .join("upgrade")
        .join(chain_id.to_string())
        .join("executed.json");
    let raw = std::fs::read_to_string(&path).map_err(io_ctx("read", &path))?;
    let doc: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| UpgradeError::Config(format!("parse executed.json: {e}")))?;
    doc.get("transactions")
        .and_then(|t| t.get(0))
        .and_then(|t| t.get("tx_hash"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| UpgradeError::Config("finalize tx_hash not found in executed.json".into()))
}

/// Count the legacy bridged tokens (0 = stage3 registers ETH only).
/// Reads the `<env>-bridged-tokens.toml` the discover phase persisted.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the file is missing or unreadable.
pub fn bridged_tokens_count(state_dir: &Path, env: &str) -> Result<usize> {
    let path = state_dir
        .join("l1-contracts/upgrade-envs")
        .join(V31_ENV_DIR)
        .join(format!("{env}-bridged-tokens.toml"));
    let raw = std::fs::read_to_string(&path).map_err(io_ctx("read", &path))?;
    Ok(count_bridged(&raw))
}

/// Count `0x` addresses inside the `bridged_tokens = [ .. ]` array. Header
/// comments carry 0x addresses too, so slice from the key before counting.
fn count_bridged(raw: &str) -> usize {
    raw.split_once("bridged_tokens")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map_or("", |(inside, _)| inside)
        .matches("0x")
        .count()
}

/// The new `BytecodesSupplier` deployed by prepare, from the merged
/// `ecosystem.toml` — surfaced for the server-config operator step.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the file is missing or has no address.
pub fn new_bytecodes_supplier(state_dir: &Path, env: &str) -> Result<String> {
    let path = output_dir(state_dir, env).join("ecosystem.toml");
    let raw = std::fs::read_to_string(&path).map_err(io_ctx("read", &path))?;
    raw.lines()
        .find(|l| l.trim_start().starts_with("bytecodes_supplier_addr"))
        .and_then(|l| l.split('"').nth(1))
        .map(str::to_owned)
        .ok_or_else(|| {
            UpgradeError::Config("bytecodes_supplier_addr not found in ecosystem.toml".into())
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn counts_bridged_tokens_ignoring_header_addresses() {
        let empty =
            "# Bridgehub: 0x77892fF9\n# AssetRouter: 0xD4CdA460\n[tokens]\nbridged_tokens = []\n";
        assert_eq!(count_bridged(empty), 0);

        let two = "# Bridgehub: 0x77892fF9\n[tokens]\nbridged_tokens = [\n  \"0xaaaa\",\n  \"0xbbbb\",\n]\n";
        assert_eq!(count_bridged(two), 2);
    }

    fn upgrade_out(root: &Path, env: &str, chain_id: u64) -> PathBuf {
        let dir = output_dir(root, env)
            .join("upgrade")
            .join(chain_id.to_string());
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finalize_tx_hash_reads_first_tx() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            upgrade_out(dir.path(), "e", 6565).join("executed.json"),
            r#"{"transactions":[{"tx_hash":"0xabc123"}]}"#,
        )
        .unwrap();
        assert_eq!(finalize_tx_hash(dir.path(), "e", 6565).unwrap(), "0xabc123");
    }

    #[test]
    fn finalize_tx_hash_errors_without_hash() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            upgrade_out(dir.path(), "e", 6565).join("executed.json"),
            r#"{"transactions":[]}"#,
        )
        .unwrap();
        assert!(finalize_tx_hash(dir.path(), "e", 6565).is_err());
    }

    #[test]
    fn new_bytecodes_supplier_reads_ecosystem_toml() {
        let dir = tempfile::tempdir().unwrap();
        let out = output_dir(dir.path(), "e");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            out.join("ecosystem.toml"),
            "other = 1\nbytecodes_supplier_addr = \"0x817e3f0C1d833917725d4576acDDF4686d5423dC\"\n",
        )
        .unwrap();
        assert_eq!(
            new_bytecodes_supplier(dir.path(), "e").unwrap(),
            "0x817e3f0C1d833917725d4576acDDF4686d5423dC"
        );
    }
}
