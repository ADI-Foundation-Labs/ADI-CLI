//! Pure builders for the `sh -c` bodies that drive protocol_ops in-container.
//
// State persists between ephemeral containers only via `/workspace` (the state-dir
// bind mount); the baked l1-contracts live under `/deps`. Each phase copies the env
// pair plus any prior output into the baked tree, runs protocol_ops there, then
// copies the fresh output back to `/workspace`.

use alloy_primitives::Address;

use crate::v31::env_toml::V31_ENV_DIR;

/// Env vars (container-side) that carry the broadcast keys, kept off argv.
pub const GOV_KEY_VAR: &str = "V31_GOV_KEY";
/// Deployer key env var (prepare bundles are deployer-signed).
pub const DEP_KEY_VAR: &str = "V31_DEP_KEY";
/// Chain-governor key env var (chain-level phases: set-ts, upgrade, total-supply
/// bundles are signed by the chain's ChainAdmin owner, not the ecosystem governor).
pub const CHAIN_KEY_VAR: &str = "V31_CHAIN_KEY";

const CONTRACTS_ROOT: &str = "/deps/era-contracts";
const IMG_ENVS: &str = "/deps/era-contracts/l1-contracts/upgrade-envs";
const WS_ENVS: &str = "/workspace/l1-contracts/upgrade-envs";
const OUT_BASE: &str = "/deps/era-contracts/l1-contracts/upgrade-envs/v0.31.0-interopB/output";

/// One signer: an EOA plus the env var (passed to the container) holding its key.
#[derive(Debug, Clone, Copy)]
pub struct Signer<'a> {
    /// Address that signs the broadcast.
    pub address: Address,
    /// Name of the env var holding the private key.
    pub key_var: &'a str,
}

fn preamble(env: &str) -> String {
    format!(
        "set -e\n\
         mkdir -p {IMG_ENVS}/permanent-values {IMG_ENVS}/{V31_ENV_DIR} {IMG_ENVS}/{V31_ENV_DIR}/output\n\
         cp {WS_ENVS}/permanent-values/{env}.toml {IMG_ENVS}/permanent-values/\n\
         cp {WS_ENVS}/{V31_ENV_DIR}/{env}.toml {IMG_ENVS}/{V31_ENV_DIR}/\n\
         if [ -d {WS_ENVS}/{V31_ENV_DIR}/output/{env} ]; then \
           cp -r {WS_ENVS}/{V31_ENV_DIR}/output/{env} {IMG_ENVS}/{V31_ENV_DIR}/output/; fi\n\
         if [ -f {WS_ENVS}/{V31_ENV_DIR}/{env}-bridged-tokens.toml ]; then \
           cp {WS_ENVS}/{V31_ENV_DIR}/{env}-bridged-tokens.toml {IMG_ENVS}/{V31_ENV_DIR}/; fi\n\
         export PROTOCOL_CONTRACTS_ROOT={CONTRACTS_ROOT}\n"
    )
}

fn copy_output(env: &str) -> String {
    format!(
        "mkdir -p {WS_ENVS}/{V31_ENV_DIR}/output\n\
         if [ ! -d {OUT_BASE}/{env} ]; then \
           echo 'protocol_ops produced no output to persist' >&2; exit 1; fi\n\
         cp -r {OUT_BASE}/{env} {WS_ENVS}/{V31_ENV_DIR}/output/\n"
    )
}

/// `--key A=$VA --key B=$VB` for every signer the manifest may reference.
fn key_args(signers: &[Signer<'_>]) -> String {
    signers
        .iter()
        .map(|s| format!("--key {}=${}", s.address.to_checksum(None), s.key_var))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Phase 1: deploy all new ecosystem contracts on an anvil fork; emit bundles +
/// `ecosystem.toml`. Signed by the deployer EOA (fork auto-impersonation).
#[must_use]
pub fn prepare_all(env: &str, bridgehub: Address, deployer: Address, l1_rpc: &str) -> String {
    let cmd = format!(
        "protocol_ops ecosystem upgrade-prepare-all --env {env} --bridgehub {bh} \
         --l1-rpc-url {l1_rpc} --deployer-address {d} --out {OUT_BASE}/{env}/prepare \
         --additional-args=--skip-simulation\n",
        bh = bridgehub.to_checksum(None),
        d = deployer.to_checksum(None),
    );
    format!("{}{cmd}{}", preamble(env), copy_output(env))
}

/// Inputs for [`broadcast`].
pub struct BroadcastParams<'a> {
    /// Env preset name.
    pub env: &'a str,
    /// Manifest path under the env output dir (`prepare`, `governance`, `stage3`, `upgrade/<chain>`, ...).
    pub rel: &'a str,
    /// L1 RPC reachable from inside the container.
    pub l1_rpc: &'a str,
    /// Signer set offered to protocol_ops (address + key env var per bundle).
    pub signers: &'a [Signer<'a>],
    /// Impersonate each bundle's target on the node instead of signing (fork rehearsal).
    pub unlocked: bool,
    /// Broadcast only bundles whose target has a key; the caller surfaces the rest.
    pub skip_unkeyed: bool,
}

/// Broadcast a prepared bundle set. With `unlocked`, protocol_ops impersonates each
/// bundle's target (fork rehearsal) and ignores keys; otherwise it signs with the
/// per-signer keys. See [`BroadcastParams`].
#[must_use]
pub fn broadcast(p: &BroadcastParams<'_>) -> String {
    let (env, rel, l1_rpc) = (p.env, p.rel, p.l1_rpc);
    let auth = if p.unlocked {
        "--unlocked".to_string()
    } else {
        key_args(p.signers)
    };
    // A phase manifest can mix signers (e.g. prepare = deployer deploys + a
    // governance-owner routing tx). `--skip-unkeyed` broadcasts only the bundles
    // whose target has a key; the caller surfaces the rest.
    let skip = if p.skip_unkeyed {
        " --skip-unkeyed"
    } else {
        ""
    };
    let cmd = format!(
        "protocol_ops ecosystem upgrade-broadcast \
         --manifest {OUT_BASE}/{env}/{rel}/manifest.json --l1-rpc-url {l1_rpc} \
         {auth}{skip} --out {OUT_BASE}/{env}/{rel}/executed.json\n",
    );
    format!("{}{cmd}{}", preamble(env), copy_output(env))
}

/// Governance stages 0+1+2: run `protocol_ops ecosystem upgrade-governance`, emitting
/// the bundle set that cuts the ecosystem over to v31.
#[must_use]
pub fn governance_prepare(env: &str, l1_rpc: &str) -> String {
    let cmd = format!(
        "protocol_ops ecosystem upgrade-governance --env {env} --l1-rpc-url {l1_rpc} \
         --out {OUT_BASE}/{env}/governance\n"
    );
    format!("{}{cmd}{}", preamble(env), copy_output(env))
}

/// Phase 3: legacy-token registration (`stage3`). `sender` is a signable EOA (the
/// deployer) protocol_ops requires for the stage3 script.
#[must_use]
pub fn stage3_prepare(env: &str, bridgehub: Address, sender: Address, l1_rpc: &str) -> String {
    let cmd = format!(
        "protocol_ops ecosystem stage3 --env {env} --bridgehub {bh} --sender {s} \
         --l1-rpc-url {l1_rpc} --out {OUT_BASE}/{env}/stage3\n",
        bh = bridgehub.to_checksum(None),
        s = sender.to_checksum(None),
    );
    format!("{}{cmd}{}", preamble(env), copy_output(env))
}

/// Scan L1 history for legacy bridged tokens and write the per-env
/// bridged-tokens TOML that stage3 registers. The list is persisted back to
/// `/workspace` so the stage3 phase reads it (per-env override, not the
/// committed ETH-only default). Read-only on-chain; no signer, no broadcast.
#[must_use]
pub fn discover_bridged_tokens(env: &str, l1_rpc: &str, from_block: u64) -> String {
    let toml = format!("{IMG_ENVS}/{V31_ENV_DIR}/{env}-bridged-tokens.toml");
    let cmd = format!(
        "cd {CONTRACTS_ROOT}/l1-contracts\n\
         yarn --silent ts-node scripts/discover-legacy-bridged-tokens.ts discover \
           --env {env} --rpc {l1_rpc} --from-block {from_block}\n\
         cd /\n\
         if [ -f {toml} ]; then cp {toml} {WS_ENVS}/{V31_ENV_DIR}/; fi\n"
    );
    format!("{}{cmd}", preamble(env))
}

fn chain_prepare(env: &str, sub: &str, chain_id: u64, extra: &str, l1_rpc: &str) -> String {
    let cmd = format!(
        "protocol_ops chain {sub} --env {env} --chain-id {chain_id} --l1-rpc-url {l1_rpc}{extra} \
         --out {OUT_BASE}/{env}/{sub}/{chain_id}\n"
    );
    format!("{}{cmd}{}", preamble(env), copy_output(env))
}

/// Inputs for [`set_upgrade_timestamp`].
pub struct UpgradeTimestampParams<'a> {
    /// Env preset name.
    pub env: &'a str,
    /// Chain being upgraded.
    pub chain_id: u64,
    /// Packed v31 protocol version.
    pub new_protocol_version: u64,
    /// Upgrade timestamp in unix seconds (`1` = immediately, used locally).
    pub upgrade_timestamp: u64,
    /// L1 RPC reachable from inside the container.
    pub l1_rpc: &'a str,
}

/// `chain set-upgrade-timestamp`: let the server detect the pending upgrade.
#[must_use]
pub fn set_upgrade_timestamp(p: &UpgradeTimestampParams<'_>) -> String {
    let extra = format!(
        " --new-protocol-version {} --upgrade-timestamp {}",
        p.new_protocol_version, p.upgrade_timestamp
    );
    chain_prepare(p.env, "set-upgrade-timestamp", p.chain_id, &extra, p.l1_rpc)
}

/// `chain upgrade`: finalize the diamond cut. `da` optionally re-sets the DA pair.
#[must_use]
pub fn chain_upgrade(
    env: &str,
    chain_id: u64,
    da: Option<(Address, &str)>,
    l1_rpc: &str,
) -> String {
    let extra = da.map_or_else(String::new, |(validator, scheme)| {
        format!(
            " --l1-da-validator {v} --l2-da-commitment-scheme {scheme}",
            v = validator.to_checksum(None),
        )
    });
    chain_prepare(env, "upgrade", chain_id, &extra, l1_rpc)
}

/// Inputs for [`set_total_supply`].
pub struct TotalSupplyParams<'a> {
    /// Env preset name.
    pub env: &'a str,
    /// Chain being upgraded.
    pub chain_id: u64,
    /// Bridgehub the calc reads deposits from.
    pub bridgehub: Address,
    /// L1 RPC reachable from inside the container.
    pub l1_rpc: &'a str,
    /// L2 RPC reachable from inside the container.
    pub l2_rpc: &'a str,
    /// Chain-upgrade (finalize) tx the calc scans chain history from.
    pub finalize_tx: &'a str,
}

/// `chain set-zkos-pre-v31-total-supply`: compute the pre-v31 base-token supply
/// with the era-contracts ts helper (needs the finalize tx + L2 RPC), then record
/// it. One-time and irreversible on-chain. The raw value never leaves the
/// container — it is parsed from the helper and passed straight to the setter.
#[must_use]
pub fn set_total_supply(p: &TotalSupplyParams<'_>) -> String {
    let (env, chain_id, l1_rpc, l2_rpc, finalize_tx) =
        (p.env, p.chain_id, p.l1_rpc, p.l2_rpc, p.finalize_tx);
    let sub = "set-zkos-pre-v31-total-supply";
    let logdir = format!("{OUT_BASE}/{env}/{sub}/{chain_id}");
    // Tee the calc so the RAW value is persisted (total-supply.log) rather than
    // living only in the run log; awk parses RAW from the same stream.
    let calc = format!(
        "mkdir -p {logdir}\n\
         cd {CONTRACTS_ROOT}/l1-contracts\n\
         raw=$(yarn --silent ts-node scripts/calculate-zkos-pre-v31-total-supply.ts \
           --chain-id {chain_id} --bridgehub {bh} --l1-rpc {l1_rpc} --l2-rpc {l2_rpc} \
           --upgrade-l1-tx {finalize_tx} | tee {logdir}/total-supply.log \
           | awk -F'[[:space:]]+' '/raw uint256:/{{print $NF}}')\n\
         if [ -z \"$raw\" ]; then echo 'failed to compute pre-v31 total supply'; exit 1; fi\n\
         echo \"captured RAW=$raw\" | tee -a {logdir}/total-supply.log\n\
         cd /\n",
        bh = p.bridgehub.to_checksum(None),
    );
    let set = format!(
        "protocol_ops chain {sub} --env {env} --chain-id {chain_id} --l1-rpc-url {l1_rpc} \
         --pre-v31-total-supply $raw --out {OUT_BASE}/{env}/{sub}/{chain_id}\n",
    );
    format!("{}{calc}{set}{}", preamble(env), copy_output(env))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn signers() -> Vec<Signer<'static>> {
        vec![
            Signer {
                address: "0xeDd8D461D918B37849eb2EBb77763cB1e8838afc"
                    .parse()
                    .unwrap(),
                key_var: GOV_KEY_VAR,
            },
            Signer {
                address: "0xbbd9e63ec7dafdf93ab0df3e544f0b6c99c13439"
                    .parse()
                    .unwrap(),
                key_var: DEP_KEY_VAR,
            },
        ]
    }

    #[test]
    fn prepare_all_command_and_output_restore() {
        let bh: Address = "0x12a71836e0Fe9434389bbFDaEB8Ea497D0593BD6"
            .parse()
            .unwrap();
        let d: Address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .unwrap();
        let s = prepare_all("mig-base", bh, d, "http://localhost:8545");
        assert!(s.contains("export PROTOCOL_CONTRACTS_ROOT=/deps/era-contracts"));
        assert!(s.contains("ecosystem upgrade-prepare-all --env mig-base"));
        assert!(s.contains("--additional-args=--skip-simulation"));
        assert!(!s.contains("--offline"));
        // the preamble restores any prior output into the baked tree
        assert!(s.contains("upgrade-envs/v0.31.0-interopB/output/mig-base"));
    }

    #[test]
    fn broadcast_passes_both_keys_by_env_var() {
        let s = broadcast(&BroadcastParams {
            env: "e",
            rel: "prepare",
            l1_rpc: "http://l1",
            signers: &signers(),
            unlocked: false,
            skip_unkeyed: false,
        })
        .to_lowercase();
        assert!(s.contains("--manifest /deps/era-contracts/l1-contracts/upgrade-envs/v0.31.0-interopb/output/e/prepare/manifest.json"));
        assert!(s.contains("--key 0xedd8d461d918b37849eb2ebb77763cb1e8838afc=$v31_gov_key"));
        assert!(s.contains("--key 0xbbd9e63ec7dafdf93ab0df3e544f0b6c99c13439=$v31_dep_key"));
        assert!(!s.contains("--unlocked"));
    }

    #[test]
    fn broadcast_unlocked_impersonates_no_keys() {
        let s = broadcast(&BroadcastParams {
            env: "e",
            rel: "prepare",
            l1_rpc: "http://l1",
            signers: &signers(),
            unlocked: true,
            skip_unkeyed: false,
        });
        assert!(s.contains("upgrade-broadcast"));
        assert!(s.contains("--unlocked"));
        // no signer keys are passed in unlocked (fork) mode
        assert!(!s.contains("--key"));
    }

    #[test]
    fn set_upgrade_timestamp_carries_version_and_ts() {
        let s = set_upgrade_timestamp(&UpgradeTimestampParams {
            env: "e",
            chain_id: 6565,
            new_protocol_version: 133_143_986_176,
            upgrade_timestamp: 1,
            l1_rpc: "http://l1",
        });
        assert!(s.contains("chain set-upgrade-timestamp --env e --chain-id 6565"));
        assert!(s.contains("--new-protocol-version 133143986176"));
        assert!(s.contains("--upgrade-timestamp 1"));
        // prepare-only: no broadcast tail
        assert!(!s.contains("upgrade-broadcast"));
    }

    #[test]
    fn chain_upgrade_da_pair_is_optional() {
        let plain = chain_upgrade("e", 6565, None, "http://l1");
        assert!(!plain.contains("--l1-da-validator"));
        let da: Address = "0x712c558755958afE7Ed7BBd5B69EF6100c2F23a3"
            .parse()
            .unwrap();
        let with = chain_upgrade("e", 6565, Some((da, "rollup")), "http://l1");
        assert!(with
            .to_lowercase()
            .contains("--l1-da-validator 0x712c558755958afe7ed7bbd5b69ef6100c2f23a3"));
        assert!(with.contains("--l2-da-commitment-scheme rollup"));
    }

    #[test]
    fn set_total_supply_computes_then_sets() {
        let bh: Address = "0x12a71836e0Fe9434389bbFDaEB8Ea497D0593BD6"
            .parse()
            .unwrap();
        let s = set_total_supply(&TotalSupplyParams {
            env: "e",
            chain_id: 6565,
            bridgehub: bh,
            l1_rpc: "http://l1",
            l2_rpc: "http://l2",
            finalize_tx: "0xabc",
        });
        // computes the raw value in-container from the ts helper + finalize tx
        assert!(s.contains("calculate-zkos-pre-v31-total-supply.ts"));
        assert!(s.contains("--upgrade-l1-tx 0xabc"));
        assert!(s.contains("--l2-rpc http://l2"));
        assert!(s.contains("awk -F'[[:space:]]+' '/raw uint256:/{print $NF}'"));
        // tees the RAW computation to a persisted total-supply.log
        assert!(s.contains("tee /deps/era-contracts/l1-contracts/upgrade-envs/v0.31.0-interopB/output/e/set-zkos-pre-v31-total-supply/6565/total-supply.log"));
        // then feeds it straight to the setter (prepare-only, no broadcast)
        assert!(s.contains("set-zkos-pre-v31-total-supply --env e --chain-id 6565"));
        assert!(s.contains("--pre-v31-total-supply $raw"));
        assert!(!s.contains("upgrade-broadcast"));
    }

    #[test]
    fn discover_bridged_tokens_scans_and_persists() {
        let s = discover_bridged_tokens("e", "http://l1", 0);
        assert!(s.contains("discover-legacy-bridged-tokens.ts discover --env e --rpc http://l1"));
        // persists the list back to /workspace so stage3 reads it
        assert!(s.contains("cp /deps/era-contracts/l1-contracts/upgrade-envs/v0.31.0-interopB/e-bridged-tokens.toml /workspace/l1-contracts/upgrade-envs/v0.31.0-interopB/"));
        // read-only: no signer, no broadcast
        assert!(!s.contains("--key"));
        assert!(!s.contains("upgrade-broadcast"));
    }
}
