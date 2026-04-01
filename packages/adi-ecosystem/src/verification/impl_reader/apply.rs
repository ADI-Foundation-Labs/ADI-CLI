//! Apply collected implementation addresses to ecosystem contracts.

use super::types::ImplementationAddresses;
use adi_types::EcosystemContracts;

/// Mutates the ecosystem contracts to include the implementation addresses.
pub fn apply_implementations(ecosystem: &mut EcosystemContracts, impls: &ImplementationAddresses) {
    if let Some(ref mut ctm) = ecosystem.zksync_os_ctm {
        ctm.bridgehub_impl_addr = impls.bridgehub_impl;
        ctm.message_root_impl_addr = impls.message_root_impl;
        ctm.native_token_vault_impl_addr = impls.native_token_vault_impl;
        ctm.stm_deployment_tracker_impl_addr = impls.stm_deployment_tracker_impl;
        ctm.chain_type_manager_impl_addr = impls.chain_type_manager_impl;
        ctm.server_notifier_impl_addr = impls.server_notifier_impl;
        ctm.erc20_bridge_impl_addr = impls.erc20_bridge_impl;
        ctm.shared_bridge_impl_addr = impls.shared_bridge_impl;
        ctm.l1_nullifier_impl_addr = impls.l1_nullifier_impl;
        ctm.validator_timelock_impl_addr = impls.validator_timelock_impl;
        ctm.verifier_fflonk_addr = impls.verifier_fflonk;
        ctm.verifier_plonk_addr = impls.verifier_plonk;
        ctm.bridged_token_beacon_addr = impls.bridged_token_beacon;
        ctm.bridged_standard_erc20_addr = impls.bridged_standard_erc20;
        ctm.dummy_avail_bridge_addr = impls.dummy_avail_bridge;
        ctm.dummy_vector_x_addr = impls.dummy_vector_x;
        ctm.server_notifier_proxy_admin_addr = impls.server_notifier_proxy_admin;
        ctm.verifier_owner_addr = impls.verifier_owner;
        ctm.is_testnet_verifier = impls.is_testnet_verifier;
    }

    ecosystem.chain_admin_owner = impls.chain_admin_owner;
}
