//! Render the two protocol_ops env preset TOMLs for a v31 upgrade.
//!
//! protocol_ops selects per-env inputs with `--env <name>` and reads two files
//! under `l1-contracts/upgrade-envs/`:
//!   permanent-values/<env>.toml   chain-permanent addresses
//!   v0.31.0-interopB/<env>.toml   per-regen inputs (unquoted hex literals)
//!
//! The v31 input file carries unquoted hex (`old_protocol_version = 0x1e...`)
//! that the toml crate rejects, so both files are rendered as plain strings
//! rather than serialized. The shape mirrors the committed `adi-local.toml`
//! pair in era-contracts.

use std::fs;
use std::path::{Path, PathBuf};

use alloy_primitives::{Address, B256};

use crate::error::{io_ctx, Result};

/// Upgrade-env subdirectory holding the v31 per-regen input TOMLs.
pub const V31_ENV_DIR: &str = "v0.31.0-interopB";

/// One CTM entry: a chain type manager and its v31 upgrade inputs.
#[derive(Debug, Clone)]
pub struct CtmInput {
    /// CTM proxy address.
    pub proxy: Address,
    /// Whether this CTM runs ZKsync OS (Ownable2Step verifier, zkos getters).
    pub is_zk_sync_os: bool,
    /// L1 bytecodes supplier for this CTM.
    pub bytecodes_supplier: Address,
    /// Rollup DA manager for this CTM.
    pub rollup_da_manager: Address,
    /// Per-CTM CREATE2 salt used to deploy this upgrade's implementations.
    pub create2_salt: B256,
}

/// Everything needed to render the v31 env preset pair for one chain.
#[derive(Debug, Clone)]
pub struct V31EnvInputs {
    /// Env name, e.g. `adi-local`. Selects the file names protocol_ops reads.
    pub env_name: String,
    /// Registered Era chain id baked into the core withdrawal contracts.
    pub era_chain_id: u64,
    /// The chain being upgraded.
    pub chain_id: u64,
    /// Bridgehub proxy.
    pub bridgehub: Address,
    /// Ecosystem governance owner (bridgehub.owner(), the Governance contract) set
    /// as the owner of deployed contracts and the governance-side sender.
    pub owner_address: Address,
    /// Deterministic CREATE2 factory (usually 0x4e59…4956c).
    pub create2_factory: Address,
    /// L2 native token vault asset id for the base token.
    pub zk_token_asset_id: B256,
    /// Old protocol version as a raw uint (rendered unquoted hex).
    pub old_protocol_version: u64,
    /// Whether to deploy the mock/testnet verifier.
    pub testnet_verifier: bool,
    /// Initial delay for the governance upgrade timer.
    pub governance_upgrade_timer_initial_delay: u64,
    /// Governance min delay used by the scripts.
    pub governance_min_delay: u64,
    /// CTMs registered on the bridgehub.
    pub ctms: Vec<CtmInput>,
    /// Per-regen core CREATE2 salt.
    pub create2_factory_salt: B256,
    /// Per-regen legacy Governance ceremony salt.
    pub legacy_gov_salt: B256,
}

/// Render `permanent-values/<env>.toml`.
#[must_use]
pub fn render_permanent_values(i: &V31EnvInputs) -> String {
    let mut s = String::new();
    s.push_str(&format!("era_chain_id = {}\n", i.era_chain_id));
    s.push_str(&format!(
        "zk_token_asset_id = \"{}\"\n\n",
        hex_b256(&i.zk_token_asset_id)
    ));
    s.push_str("[core_contracts]\n");
    s.push_str(&format!(
        "bridgehub_proxy_addr = \"{}\"\n",
        checksum(&i.bridgehub)
    ));
    // ADI's bridgehub is owned by legacy `Governance.sol`; the PUH path is
    // never used here.
    s.push_str("governance_kind = \"legacy\"\n\n");
    for ctm in &i.ctms {
        s.push_str("[[ctm_contracts.ctms]]\n");
        s.push_str(&format!("proxy = \"{}\"\n", checksum(&ctm.proxy)));
        s.push_str(&format!("is_zk_sync_os = {}\n", ctm.is_zk_sync_os));
        s.push_str(&format!(
            "bytecodes_supplier = \"{}\"\n",
            checksum(&ctm.bytecodes_supplier)
        ));
        s.push_str(&format!(
            "rollup_da_manager = \"{}\"\n\n",
            checksum(&ctm.rollup_da_manager)
        ));
    }
    s.push_str("[permanent_contracts]\n");
    s.push_str(&format!(
        "create2_factory_addr = \"{}\"\n",
        checksum(&i.create2_factory)
    ));
    s
}

/// Render `v0.31.0-interopB/<env>.toml`. `old_protocol_version` is emitted as
/// unquoted hex, matching what protocol_ops line-parses.
#[must_use]
pub fn render_v31_input(i: &V31EnvInputs) -> String {
    let mut s = String::new();
    s.push_str(&format!("era_chain_id = {}\n", i.era_chain_id));
    s.push_str(&format!("testnet_verifier = {}\n", i.testnet_verifier));
    s.push_str(&format!(
        "governance_upgrade_timer_initial_delay = {}\n",
        i.governance_upgrade_timer_initial_delay
    ));
    s.push_str(&format!(
        "owner_address = \"{}\"\n",
        checksum(&i.owner_address)
    ));
    s.push_str(&format!(
        "old_protocol_version = 0x{:x}\n",
        i.old_protocol_version
    ));
    s.push_str(&format!(
        "bridgehub_proxy_address = \"{}\"\n\n",
        checksum(&i.bridgehub)
    ));
    s.push_str("[chain]\n");
    s.push_str(&format!("chain_id = {}\n\n", i.chain_id));
    s.push_str("[contracts]\n");
    s.push_str(&format!(
        "governance_min_delay = {}\n",
        i.governance_min_delay
    ));
    s.push_str(&format!(
        "create2_factory_salt = \"{}\"\n",
        hex_b256(&i.create2_factory_salt)
    ));
    s.push_str(&format!(
        "legacy_gov_salt = \"{}\"\n",
        hex_b256(&i.legacy_gov_salt)
    ));
    s.push_str(&format!(
        "create2_factory_addr = \"{}\"\n\n",
        checksum(&i.create2_factory)
    ));
    s.push_str("[create2_factory_salts]\n");
    for ctm in &i.ctms {
        s.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            checksum(&ctm.proxy),
            hex_b256(&ctm.create2_salt)
        ));
    }
    s
}

/// Write both env TOMLs under `<l1_contracts_dir>/upgrade-envs/...` and return
/// their paths. Directories are created as needed.
pub fn write_env_tomls(l1_contracts_dir: &Path, i: &V31EnvInputs) -> Result<(PathBuf, PathBuf)> {
    let pv_dir = l1_contracts_dir.join("upgrade-envs/permanent-values");
    let input_dir = l1_contracts_dir.join(format!("upgrade-envs/{V31_ENV_DIR}"));
    fs::create_dir_all(&pv_dir).map_err(io_ctx("write", &pv_dir))?;
    fs::create_dir_all(&input_dir).map_err(io_ctx("write", &input_dir))?;

    let pv_path = pv_dir.join(format!("{}.toml", i.env_name));
    let input_path = input_dir.join(format!("{}.toml", i.env_name));
    fs::write(&pv_path, render_permanent_values(i)).map_err(io_ctx("write", &pv_path))?;
    fs::write(&input_path, render_v31_input(i)).map_err(io_ctx("write", &input_path))?;
    Ok((pv_path, input_path))
}

fn checksum(a: &Address) -> String {
    a.to_checksum(None)
}

fn hex_b256(b: &B256) -> String {
    format!("0x{}", hex::encode(b.as_slice()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sample() -> V31EnvInputs {
        V31EnvInputs {
            env_name: "adi-local".to_string(),
            era_chain_id: 270,
            chain_id: 6565,
            bridgehub: "0xc0399067d3c5ecdd9a334f1f7538ced4ff9591aa"
                .parse()
                .unwrap(),
            owner_address: "0xbc65333b3d9ec6005f88d507769ec3efe8d839a2"
                .parse()
                .unwrap(),
            create2_factory: "0x4e59b44847b379578588920ca78fbf26c0b4956c"
                .parse()
                .unwrap(),
            zk_token_asset_id: B256::with_last_byte(1),
            old_protocol_version: 0x1e00000001,
            testnet_verifier: true,
            governance_upgrade_timer_initial_delay: 0,
            governance_min_delay: 0,
            ctms: vec![CtmInput {
                proxy: "0x0d0Abb4e5e390158d649E20d1f94E2Dc7bb3C797"
                    .parse()
                    .unwrap(),
                is_zk_sync_os: true,
                bytecodes_supplier: "0x812bb606fe4d0cd9037c67a3d3868ab68fe78f13"
                    .parse()
                    .unwrap(),
                rollup_da_manager: "0xa0E6D98b580fc48bD32b91a95Db7B0f867e8Df59"
                    .parse()
                    .unwrap(),
                create2_salt: B256::repeat_byte(0xab),
            }],
            create2_factory_salt: B256::repeat_byte(0x3b),
            legacy_gov_salt: B256::repeat_byte(0xd5),
        }
    }

    #[test]
    fn permanent_values_has_core_and_ctm() {
        let pv = render_permanent_values(&sample());
        assert!(pv.contains("era_chain_id = 270"));
        assert!(pv.contains("governance_kind = \"legacy\""));
        assert!(pv.contains("[[ctm_contracts.ctms]]"));
        assert!(pv.contains("is_zk_sync_os = true"));
        // parses as valid toml (permanent-values has no unquoted hex)
        let _: toml::Value = toml::from_str(&pv).unwrap();
    }

    #[test]
    fn v31_input_has_unquoted_old_protocol_version() {
        let inp = render_v31_input(&sample());
        assert!(inp.contains("old_protocol_version = 0x1e00000001"));
        assert!(inp.contains("chain_id = 6565"));
        assert!(inp.contains("[create2_factory_salts]"));
        // the unquoted hex must be present exactly as protocol_ops line-parses it
        assert!(!inp.contains("old_protocol_version = \""));
    }

    #[test]
    fn write_env_tomls_creates_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let (pv, input) = write_env_tomls(dir.path(), &sample()).unwrap();
        assert!(pv.ends_with("upgrade-envs/permanent-values/adi-local.toml"));
        assert!(input.ends_with(format!("upgrade-envs/{V31_ENV_DIR}/adi-local.toml")));
        assert!(pv.exists() && input.exists());
        assert!(fs::read_to_string(&pv)
            .unwrap()
            .contains("era_chain_id = 270"));
        assert!(fs::read_to_string(&input)
            .unwrap()
            .contains("chain_id = 6565"));
    }
}
