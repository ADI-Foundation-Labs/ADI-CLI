//! Contract counting functions for ecosystem and chain contracts.

use adi_types::{ChainContracts, EcosystemContracts, ZkSyncOsCtm};
use alloy_primitives::Address;

/// Count non-None addresses in ecosystem contracts.
pub(super) fn count_ecosystem_contracts(contracts: &EcosystemContracts) -> usize {
    let mut count = 0;

    // Top-level addresses
    if contracts.create2_factory_addr.is_some() {
        count += 1;
    }
    if contracts.multicall3_addr.is_some() {
        count += 1;
    }

    // Core ecosystem contracts
    if let Some(ref core) = contracts.core_ecosystem_contracts {
        if core.bridgehub_proxy_addr.is_some() {
            count += 1;
        }
        if core.message_root_proxy_addr.is_some() {
            count += 1;
        }
        if core.transparent_proxy_admin_addr.is_some() {
            count += 1;
        }
        if core.stm_deployment_tracker_proxy_addr.is_some() {
            count += 1;
        }
        if core.native_token_vault_addr.is_some() {
            count += 1;
        }
    }

    // Bridges
    if let Some(ref bridges) = contracts.bridges {
        if let Some(ref erc20) = bridges.erc20 {
            if erc20.l1_address.is_some() {
                count += 1;
            }
            if erc20.l2_address.is_some() {
                count += 1;
            }
        }
        if let Some(ref shared) = bridges.shared {
            if shared.l1_address.is_some() {
                count += 1;
            }
            if shared.l2_address.is_some() {
                count += 1;
            }
        }
        if bridges.l1_nullifier_addr.is_some() {
            count += 1;
        }
    }

    // L1 contracts
    if let Some(ref l1) = contracts.l1 {
        if l1.governance_addr.is_some() {
            count += 1;
        }
        if l1.chain_admin_addr.is_some() {
            count += 1;
        }
        if l1.transaction_filterer_addr.is_some() {
            count += 1;
        }
    }

    // ZkSync OS CTM
    if let Some(ref ctm) = contracts.zksync_os_ctm {
        count += count_ctm_contracts(ctm);
    }

    count
}

/// Count non-None addresses in a slice of optional addresses.
pub(super) fn count_some(addrs: &[Option<Address>]) -> usize {
    addrs.iter().filter(|a| a.is_some()).count()
}

/// Count non-None addresses in ZkSyncOsCtm.
pub(super) fn count_ctm_contracts(ctm: &ZkSyncOsCtm) -> usize {
    let core = count_some(&[
        ctm.governance,
        ctm.chain_admin,
        ctm.proxy_admin,
        ctm.state_transition_proxy_addr,
        ctm.validator_timelock_addr,
        ctm.server_notifier_proxy_addr,
        ctm.verifier_addr,
        ctm.l1_rollup_da_manager,
        ctm.l1_bytecodes_supplier_addr,
        ctm.l1_wrapped_base_token_store,
        ctm.default_upgrade_addr,
        ctm.genesis_upgrade_addr,
        ctm.rollup_l1_da_validator_addr,
        ctm.no_da_validium_l1_validator_addr,
        ctm.blobs_zksync_os_l1_da_validator_addr,
        ctm.avail_l1_da_validator_addr,
    ]);

    let facets = count_some(&[
        ctm.admin_facet_addr,
        ctm.executor_facet_addr,
        ctm.mailbox_facet_addr,
        ctm.getters_facet_addr,
        ctm.diamond_init_addr,
    ]);

    let impls = count_some(&[
        ctm.bridgehub_impl_addr,
        ctm.message_root_impl_addr,
        ctm.native_token_vault_impl_addr,
        ctm.stm_deployment_tracker_impl_addr,
        ctm.chain_type_manager_impl_addr,
        ctm.server_notifier_impl_addr,
        ctm.erc20_bridge_impl_addr,
        ctm.shared_bridge_impl_addr,
        ctm.l1_nullifier_impl_addr,
        ctm.validator_timelock_impl_addr,
    ]);

    let other = count_some(&[
        ctm.verifier_fflonk_addr,
        ctm.verifier_plonk_addr,
        ctm.bridged_standard_erc20_addr,
        ctm.bridged_token_beacon_addr,
        ctm.dummy_avail_bridge_addr,
        ctm.dummy_vector_x_addr,
        ctm.server_notifier_proxy_admin_addr,
    ]);

    core + facets + impls + other
}

/// Count unique chain-specific contract addresses.
///
/// Only counts addresses that are unique to the chain (not ecosystem references).
pub(super) fn count_chain_contracts(contracts: &ChainContracts) -> usize {
    let mut count = 0;

    // L1 contracts (chain-specific)
    if let Some(ref l1) = contracts.l1 {
        if l1.diamond_proxy_addr.is_some() {
            count += 1;
        }
        if l1.governance_addr.is_some() {
            count += 1;
        }
        if l1.chain_admin_addr.is_some() {
            count += 1;
        }
        if l1.chain_proxy_admin_addr.is_some() {
            count += 1;
        }
    }

    // L2 contracts
    if let Some(ref l2) = contracts.l2 {
        if l2.testnet_paymaster_addr.is_some() {
            count += 1;
        }
        if l2.default_l2_upgrader.is_some() {
            count += 1;
        }
        if l2.l2_native_token_vault_proxy_addr.is_some() {
            count += 1;
        }
        if l2.consensus_registry.is_some() {
            count += 1;
        }
        if l2.multicall3.is_some() {
            count += 1;
        }
        if l2.timestamp_asserter_addr.is_some() {
            count += 1;
        }
    }

    count
}
