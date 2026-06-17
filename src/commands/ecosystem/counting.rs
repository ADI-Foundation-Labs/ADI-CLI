//! Contract counting functions for ecosystem and chain contracts.

use adi_types::{ChainContracts, ChainEcosystemContracts, EcosystemContracts, ZkSyncOsCtm};
use alloy_primitives::Address;

/// Returns `true` if an address is present and non-zero (a real deployed
/// contract). The zero address (`0x0000…0000`) means "not deployed".
fn is_set(addr: Option<Address>) -> bool {
    matches!(addr, Some(a) if !a.is_zero())
}

/// Count addresses that are present and non-zero in ecosystem contracts.
pub(super) fn count_ecosystem_contracts(contracts: &EcosystemContracts) -> usize {
    let mut count = count_set(&[contracts.create2_factory_addr, contracts.multicall3_addr]);

    if let Some(ref core) = contracts.core_ecosystem_contracts {
        count += count_set(&[
            core.bridgehub_proxy_addr,
            core.message_root_proxy_addr,
            core.transparent_proxy_admin_addr,
            core.stm_deployment_tracker_proxy_addr,
            core.native_token_vault_addr,
        ]);
    }

    if let Some(ref bridges) = contracts.bridges {
        if let Some(ref erc20) = bridges.erc20 {
            count += count_set(&[erc20.l1_address, erc20.l2_address]);
        }
        if let Some(ref shared) = bridges.shared {
            count += count_set(&[shared.l1_address, shared.l2_address]);
        }
        count += count_set(&[bridges.l1_nullifier_addr]);
    }

    if let Some(ref l1) = contracts.l1 {
        count += count_set(&[
            l1.governance_addr,
            l1.chain_admin_addr,
            l1.transaction_filterer_addr,
        ]);
    }

    if let Some(ref ctm) = contracts.zksync_os_ctm {
        count += count_ctm_contracts(ctm);
    }

    count
}

/// Count addresses that are present and non-zero in a slice.
pub(super) fn count_set(addrs: &[Option<Address>]) -> usize {
    addrs.iter().filter(|a| is_set(**a)).count()
}

/// Count ecosystem-owned ZkSync OS CTM addresses.
pub(super) fn count_ctm_contracts(ctm: &ZkSyncOsCtm) -> usize {
    let core = count_set(&[
        ctm.governance,
        ctm.chain_admin,
        ctm.proxy_admin,
        ctm.l1_wrapped_base_token_store,
    ]);

    let impls = count_set(&[
        ctm.bridgehub_impl_addr,
        ctm.message_root_impl_addr,
        ctm.native_token_vault_impl_addr,
        ctm.stm_deployment_tracker_impl_addr,
        ctm.erc20_bridge_impl_addr,
        ctm.shared_bridge_impl_addr,
        ctm.l1_nullifier_impl_addr,
    ]);

    let tokens = count_set(&[
        ctm.bridged_standard_erc20_addr,
        ctm.bridged_token_beacon_addr,
    ]);

    core + impls + tokens
}

/// Count chain L1 deployment addresses copied from zkstack's CTM reference.
fn count_chain_ecosystem_contracts(contracts: &ChainEcosystemContracts) -> usize {
    count_set(&[
        contracts.state_transition_proxy_addr,
        contracts.validator_timelock_addr,
        contracts.server_notifier_proxy_addr,
        contracts.verifier_addr,
        contracts.l1_rollup_da_manager,
        contracts.l1_bytecodes_supplier_addr,
        contracts.default_upgrade_addr,
        contracts.genesis_upgrade_addr,
        contracts.rollup_l1_da_validator_addr,
        contracts.no_da_validium_l1_validator_addr,
        contracts.blobs_zksync_os_l1_da_validator_addr,
        contracts.avail_l1_da_validator_addr,
    ])
}

/// Count derived chain deployment addresses from the ecosystem CTM.
fn count_chain_ctm_contracts(ctm: &ZkSyncOsCtm) -> usize {
    count_set(&[
        ctm.admin_facet_addr,
        ctm.executor_facet_addr,
        ctm.mailbox_facet_addr,
        ctm.getters_facet_addr,
        ctm.diamond_init_addr,
        ctm.chain_type_manager_impl_addr,
        ctm.server_notifier_impl_addr,
        ctm.validator_timelock_impl_addr,
        ctm.verifier_fflonk_addr,
        ctm.verifier_plonk_addr,
        ctm.dummy_avail_bridge_addr,
        ctm.dummy_vector_x_addr,
        ctm.server_notifier_proxy_admin_addr,
    ])
}

/// Count chain-specific contract addresses.
///
/// Mirrors the address fields rendered by the chain panel (chain L1 + L2),
/// so the footer total matches what is displayed. The `base_token_asset_id`
/// hash is excluded — it is an asset identifier, not a deployed contract.
/// Count chain-specific contract addresses plus optional derived CTM fields.
pub(super) fn count_chain_contracts_with_ctm(
    contracts: &ChainContracts,
    ctm: Option<&ZkSyncOsCtm>,
) -> usize {
    let mut count = 0;

    if let Some(ref l1) = contracts.l1 {
        count += count_set(&[l1.diamond_proxy_addr]);
    }

    if let Some(ref ecosystem_contracts) = contracts.ecosystem_contracts {
        count += count_chain_ecosystem_contracts(ecosystem_contracts);
    }

    if let Some(ctm) = ctm {
        count += count_chain_ctm_contracts(ctm);
    }

    if let Some(ref l2) = contracts.l2 {
        count += count_set(&[
            l2.testnet_paymaster_addr,
            l2.default_l2_upgrader,
            l2.l2_native_token_vault_proxy_addr,
            l2.consensus_registry,
            l2.multicall3,
            l2.timestamp_asserter_addr,
        ]);
    }

    count
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use adi_types::{
        ChainEcosystemContracts, ChainL1Contracts, ChainL2Contracts, EcosystemContracts,
        ZkSyncOsCtm,
    };
    use alloy_primitives::{address, b256};

    #[test]
    fn ecosystem_count_excludes_chain_ctm_deployment_artifacts() {
        let contracts = EcosystemContracts {
            zksync_os_ctm: Some(ZkSyncOsCtm {
                governance: Some(address!("0000000000000000000000000000000000000001")),
                chain_admin: Some(address!("0000000000000000000000000000000000000002")),
                proxy_admin: Some(address!("0000000000000000000000000000000000000003")),
                bridgehub_impl_addr: Some(address!("0000000000000000000000000000000000000004")),
                bridged_standard_erc20_addr: Some(address!(
                    "0000000000000000000000000000000000000005"
                )),
                state_transition_proxy_addr: Some(address!(
                    "0000000000000000000000000000000000000011"
                )),
                validator_timelock_addr: Some(address!("0000000000000000000000000000000000000012")),
                admin_facet_addr: Some(address!("0000000000000000000000000000000000000013")),
                verifier_fflonk_addr: Some(address!("0000000000000000000000000000000000000014")),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(count_ecosystem_contracts(&contracts), 5);
    }

    #[test]
    fn chain_count_includes_l1_deployments_and_excludes_roles_and_hashes() {
        let contracts = ChainContracts {
            ecosystem_contracts: Some(ChainEcosystemContracts {
                state_transition_proxy_addr: Some(address!(
                    "0000000000000000000000000000000000000011"
                )),
                validator_timelock_addr: Some(address!("0000000000000000000000000000000000000012")),
                verifier_addr: Some(address!("0000000000000000000000000000000000000013")),
                l1_bytecodes_supplier_addr: Some(address!(
                    "0000000000000000000000000000000000000014"
                )),
                default_upgrade_addr: Some(address!("0000000000000000000000000000000000000015")),
                genesis_upgrade_addr: Some(address!("0000000000000000000000000000000000000016")),
                rollup_l1_da_validator_addr: Some(address!(
                    "0000000000000000000000000000000000000017"
                )),
                no_da_validium_l1_validator_addr: Some(address!(
                    "0000000000000000000000000000000000000018"
                )),
                avail_l1_da_validator_addr: Some(address!(
                    "0000000000000000000000000000000000000019"
                )),
                ..Default::default()
            }),
            l1: Some(ChainL1Contracts {
                diamond_proxy_addr: Some(address!("0000000000000000000000000000000000000021")),
                governance_addr: Some(address!("0000000000000000000000000000000000000022")),
                chain_admin_addr: Some(address!("0000000000000000000000000000000000000023")),
                base_token_addr: Some(address!("0000000000000000000000000000000000000024")),
                base_token_asset_id: Some(b256!(
                    "0000000000000000000000000000000000000000000000000000000000000031"
                )),
                ..Default::default()
            }),
            l2: Some(ChainL2Contracts {
                default_l2_upgrader: Some(address!("0000000000000000000000000000000000000041")),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(count_chain_contracts_with_ctm(&contracts, None), 11);
    }

    #[test]
    fn chain_count_includes_derived_ctm_deployments_when_available() {
        let contracts = ChainContracts {
            l1: Some(ChainL1Contracts {
                diamond_proxy_addr: Some(address!("0000000000000000000000000000000000000021")),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctm = ZkSyncOsCtm {
            admin_facet_addr: Some(address!("0000000000000000000000000000000000000031")),
            executor_facet_addr: Some(address!("0000000000000000000000000000000000000032")),
            chain_type_manager_impl_addr: Some(address!(
                "0000000000000000000000000000000000000033"
            )),
            verifier_fflonk_addr: Some(address!("0000000000000000000000000000000000000034")),
            server_notifier_proxy_admin_addr: Some(address!(
                "0000000000000000000000000000000000000035"
            )),
            ..Default::default()
        };

        assert_eq!(count_chain_contracts_with_ctm(&contracts, Some(&ctm)), 6);
    }
}
