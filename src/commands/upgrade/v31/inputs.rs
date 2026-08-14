//! Build the v31 env inputs from config, on-chain reads, and ecosystem state.

use std::time::{SystemTime, UNIX_EPOCH};

use alloy_primitives::{keccak256, Address, B256};
use alloy_provider::Provider;

use adi_upgrade::onchain;
use adi_upgrade::v31::{CtmInput, OwnershipTargets, V31EnvInputs};
use adi_upgrade::UpgradeConfig;

use crate::error::{Result, WrapErr};

/// Build [`V31EnvInputs`] from config, on-chain reads, and ecosystem state.
pub(super) async fn resolve_inputs(
    env_name: String,
    config: &UpgradeConfig,
    provider: &(impl Provider + Clone),
    state_manager: &adi_state::StateManager,
    chain_meta: &adi_types::ChainMetadata,
) -> Result<(V31EnvInputs, OwnershipTargets)> {
    let handler = adi_upgrade::versions::V0_31_0Handler;
    let bh = config.bridgehub_address;
    let ctm_proxy = onchain::query_ctm(provider, bh, chain_meta.chain_id)
        .await
        .wrap_err("resolve CTM proxy")?;

    // owner_address is the ecosystem's governance owner (bridgehub.owner(), the
    // Governance contract), NOT the governor EOA that signs on its behalf.
    let governance = onchain::query_owner(provider, bh)
        .await
        .wrap_err("resolve bridgehub owner (governance contract)")?;

    // Detect the verifier kind on-chain instead of assuming testnet.
    let diamond = onchain::query_zk_chain(provider, bh, chain_meta.chain_id)
        .await
        .wrap_err("resolve chain diamond")?;
    let verifier = onchain::query_verifier(provider, diamond)
        .await
        .wrap_err("resolve verifier")?;
    let testnet_verifier = onchain::query_is_testnet_verifier(provider, verifier).await;

    let eco = state_manager.ecosystem();
    let era_chain_id = eco
        .metadata()
        .await
        .wrap_err("load ecosystem metadata")?
        .era_chain_id;
    let contracts = eco.contracts().await.wrap_err("load ecosystem contracts")?;
    let bytecodes_supplier = contracts
        .zksync_os_ctm
        .as_ref()
        .and_then(|c| c.l1_bytecodes_supplier_addr)
        .ok_or_else(|| eyre::eyre!("ecosystem contracts missing l1_bytecodes_supplier_addr"))?;
    let rollup_da_manager = contracts
        .l1_rollup_da_manager_addr()
        .ok_or_else(|| eyre::eyre!("ecosystem contracts missing l1_rollup_da_manager"))?;

    let targets = OwnershipTargets {
        validator_timelock: contracts
            .validator_timelock_addr()
            .ok_or_else(|| eyre::eyre!("ecosystem contracts missing validator_timelock"))?,
        native_token_vault: contracts
            .native_token_vault_addr()
            .ok_or_else(|| eyre::eyre!("ecosystem contracts missing native_token_vault"))?,
        l1_nullifier: contracts
            .l1_nullifier_addr()
            .ok_or_else(|| eyre::eyre!("ecosystem contracts missing l1_nullifier"))?,
    };

    // Per-run nonce so re-runs deploy at fresh CREATE2 addresses and use fresh
    // governance op ids (avoids collisions with a prior attempt on the chain).
    let nonce = now_nonce();
    let inputs = V31EnvInputs {
        env_name,
        era_chain_id,
        chain_id: chain_meta.chain_id,
        bridgehub: bh,
        owner_address: governance,
        create2_factory: handler.create2_factory(),
        zk_token_asset_id: B256::with_last_byte(1),
        old_protocol_version: handler.old_protocol_version(),
        testnet_verifier,
        governance_upgrade_timer_initial_delay: handler.governance_upgrade_timer_initial_delay(),
        governance_min_delay: 0,
        ctms: vec![CtmInput {
            proxy: ctm_proxy,
            is_zk_sync_os: true,
            bytecodes_supplier,
            rollup_da_manager,
            create2_salt: salt(bh, ctm_proxy, "ctm", nonce),
        }],
        create2_factory_salt: salt(bh, bh, "core", nonce),
        legacy_gov_salt: salt(bh, bh, "gov", nonce),
    };
    Ok((inputs, targets))
}

/// Millisecond wall-clock nonce, folded into salts to rotate them per run.
pub(super) fn now_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Deterministic CREATE2 / governance-op salt from two addresses, a tag, and a nonce.
pub(super) fn salt(a: Address, b: Address, tag: &str, nonce: u128) -> B256 {
    let mut seed = Vec::with_capacity(40 + tag.len() + 16);
    seed.extend_from_slice(a.as_slice());
    seed.extend_from_slice(b.as_slice());
    seed.extend_from_slice(tag.as_bytes());
    seed.extend_from_slice(&nonce.to_le_bytes());
    keccak256(seed)
}

#[cfg(test)]
mod tests {
    use super::salt;
    use alloy_primitives::Address;

    #[test]
    fn salt_is_deterministic_and_input_sensitive() {
        let a = Address::from([0x11; 20]);
        let b = Address::from([0x22; 20]);
        assert_eq!(salt(a, b, "ctm", 7), salt(a, b, "ctm", 7));
        assert_ne!(salt(a, b, "ctm", 7), salt(a, b, "ctm", 8));
        assert_ne!(salt(a, b, "ctm", 7), salt(a, b, "core", 7));
        assert_ne!(salt(a, b, "ctm", 7), salt(b, a, "ctm", 7));
    }
}
