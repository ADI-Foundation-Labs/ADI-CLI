//! Helper methods for building ecosystem verification targets.
//!
//! Each function handles a logical group of contracts from [`EcosystemContracts`],
//! returning collected targets rather than mutating a shared accumulator.

use adi_types::{BridgesConfig, EcosystemContracts, ZkSyncOsCtm};
use alloy_primitives::Address;

use super::mappings::ContractRegistry;
use super::target::VerificationTarget;
use super::types::ContractType;

impl ContractRegistry {
    /// Core ecosystem proxy contracts: Bridgehub, MessageRoot, TransparentProxyAdmin,
    /// StmDeploymentTracker, NativeTokenVault.
    pub(super) fn build_core_proxy_targets(
        contracts: &EcosystemContracts,
        proxy_admin: Option<Address>,
    ) -> Vec<VerificationTarget> {
        let ctm = contracts.zksync_os_ctm.as_ref();
        let core = contracts.core_ecosystem_contracts.as_ref();

        let bridgehub = contracts
            .bridgehub_addr()
            .zip(ctm.and_then(|c| c.bridgehub_impl_addr))
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::Bridgehub, proxy, imp, admin)
            });

        let message_root = core
            .and_then(|c| c.message_root_proxy_addr)
            .zip(ctm.and_then(|c| c.message_root_impl_addr))
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::MessageRoot, proxy, imp, admin)
            });

        let transparent_proxy_admin = core
            .and_then(|c| c.transparent_proxy_admin_addr)
            .and_then(|addr| Self::build_target(ContractType::TransparentProxyAdmin, addr));

        let stm_deployment_tracker = core
            .and_then(|c| c.stm_deployment_tracker_proxy_addr)
            .zip(ctm.and_then(|c| c.stm_deployment_tracker_impl_addr))
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::StmDeploymentTracker, proxy, imp, admin)
            });

        let native_token_vault = core
            .and_then(|c| c.native_token_vault_addr)
            .zip(ctm.and_then(|c| c.native_token_vault_impl_addr))
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::NativeTokenVault, proxy, imp, admin)
            });

        [
            bridgehub,
            message_root,
            transparent_proxy_admin,
            stm_deployment_tracker,
            native_token_vault,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Governance and ChainAdmin contracts.
    pub(super) fn build_governance_targets(
        contracts: &EcosystemContracts,
    ) -> Vec<VerificationTarget> {
        let governance = contracts
            .governance_addr()
            .and_then(|addr| Self::build_target(ContractType::Governance, addr));

        let chain_admin = contracts
            .chain_admin_addr()
            .zip(contracts.chain_admin_owner)
            .and_then(|(addr, owner)| {
                Self::build_chain_admin_target(ContractType::ChainAdmin, addr, owner)
            });

        [governance, chain_admin].into_iter().flatten().collect()
    }

    /// CTM proxy contracts: StateTransitionProxy, ValidatorTimelock, ServerNotifier,
    /// and the Verifier (with constructor args).
    pub(super) fn build_ctm_proxy_targets(
        ctm: &ZkSyncOsCtm,
        proxy_admin: Option<Address>,
    ) -> Vec<VerificationTarget> {
        let state_transition = ctm
            .state_transition_proxy_addr
            .zip(ctm.chain_type_manager_impl_addr)
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::StateTransitionProxy, proxy, imp, admin)
            });

        let validator_timelock = ctm
            .validator_timelock_addr
            .zip(ctm.validator_timelock_impl_addr)
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::ValidatorTimelock, proxy, imp, admin)
            });

        let server_notifier = ctm
            .server_notifier_proxy_addr
            .zip(ctm.server_notifier_impl_addr)
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::ServerNotifier, proxy, imp, admin)
            });

        let verifier = ctm
            .verifier_addr
            .zip(ctm.verifier_fflonk_addr)
            .zip(ctm.verifier_plonk_addr)
            .and_then(|((verifier, fflonk), plonk)| {
                Self::build_verifier_target(
                    verifier,
                    fflonk,
                    plonk,
                    ctm.verifier_owner_addr,
                    ctm.is_testnet_verifier,
                )
            });

        [
            state_transition,
            validator_timelock,
            server_notifier,
            verifier,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Simple CTM contracts with no constructor args — data-driven via a single iterator.
    pub(super) fn build_ctm_simple_targets(ctm: &ZkSyncOsCtm) -> Vec<VerificationTarget> {
        let pairs: &[(ContractType, Option<Address>)] = &[
            (ContractType::L1RollupDaManager, ctm.l1_rollup_da_manager),
            (
                ContractType::L1BytecodesSupplier,
                ctm.l1_bytecodes_supplier_addr,
            ),
            (
                ContractType::RollupL1DaValidator,
                ctm.rollup_l1_da_validator_addr,
            ),
            (
                ContractType::NoDaValidiumL1Validator,
                ctm.no_da_validium_l1_validator_addr,
            ),
            (
                ContractType::BlobsZkSyncOsL1DaValidator,
                ctm.blobs_zksync_os_l1_da_validator_addr,
            ),
            (
                ContractType::AvailL1DaValidator,
                ctm.avail_l1_da_validator_addr,
            ),
            (ContractType::DefaultUpgrade, ctm.default_upgrade_addr),
            (ContractType::GenesisUpgrade, ctm.genesis_upgrade_addr),
            // Diamond facets
            (ContractType::AdminFacet, ctm.admin_facet_addr),
            (ContractType::ExecutorFacet, ctm.executor_facet_addr),
            (ContractType::MailboxFacet, ctm.mailbox_facet_addr),
            (ContractType::GettersFacet, ctm.getters_facet_addr),
            (ContractType::DiamondInit, ctm.diamond_init_addr),
            // Implementation contracts
            (ContractType::BridgehubImpl, ctm.bridgehub_impl_addr),
            (ContractType::MessageRootImpl, ctm.message_root_impl_addr),
            (
                ContractType::NativeTokenVaultImpl,
                ctm.native_token_vault_impl_addr,
            ),
            (
                ContractType::StmDeploymentTrackerImpl,
                ctm.stm_deployment_tracker_impl_addr,
            ),
            (
                ContractType::ChainTypeManagerImpl,
                ctm.chain_type_manager_impl_addr,
            ),
            (
                ContractType::ServerNotifierImpl,
                ctm.server_notifier_impl_addr,
            ),
            (ContractType::Erc20BridgeImpl, ctm.erc20_bridge_impl_addr),
            (ContractType::SharedBridgeImpl, ctm.shared_bridge_impl_addr),
            (ContractType::L1NullifierImpl, ctm.l1_nullifier_impl_addr),
            (
                ContractType::ValidatorTimelockImpl,
                ctm.validator_timelock_impl_addr,
            ),
            // Verifier components
            (ContractType::VerifierFflonk, ctm.verifier_fflonk_addr),
            (ContractType::VerifierPlonk, ctm.verifier_plonk_addr),
            // Bridge token contracts
            (
                ContractType::BridgedStandardErc20,
                ctm.bridged_standard_erc20_addr,
            ),
            (
                ContractType::BridgedTokenBeacon,
                ctm.bridged_token_beacon_addr,
            ),
            // Avail test contracts
            (ContractType::DummyAvailBridge, ctm.dummy_avail_bridge_addr),
            (ContractType::DummyVectorX, ctm.dummy_vector_x_addr),
            // Server notifier proxy admin
            (
                ContractType::ServerNotifierProxyAdmin,
                ctm.server_notifier_proxy_admin_addr,
            ),
            // L1 Wrapped Base Token Store
            (
                ContractType::L1WrappedBaseTokenStore,
                ctm.l1_wrapped_base_token_store,
            ),
        ];

        pairs
            .iter()
            .filter_map(|&(ct, addr)| addr.and_then(|a| Self::build_target(ct, a)))
            .collect()
    }

    /// Bridge proxy contracts: ERC20Bridge, SharedBridge, L1Nullifier.
    pub(super) fn build_bridge_proxy_targets(
        bridges: &BridgesConfig,
        ctm: &ZkSyncOsCtm,
        proxy_admin: Option<Address>,
    ) -> Vec<VerificationTarget> {
        let erc20_bridge = bridges
            .erc20
            .as_ref()
            .and_then(|b| b.l1_address)
            .zip(ctm.erc20_bridge_impl_addr)
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::Erc20Bridge, proxy, imp, admin)
            });

        let shared_bridge = bridges
            .shared
            .as_ref()
            .and_then(|b| b.l1_address)
            .zip(ctm.shared_bridge_impl_addr)
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::SharedBridge, proxy, imp, admin)
            });

        let l1_nullifier = bridges
            .l1_nullifier_addr
            .zip(ctm.l1_nullifier_impl_addr)
            .zip(proxy_admin)
            .and_then(|((proxy, imp), admin)| {
                Self::build_proxy_target(ContractType::L1Nullifier, proxy, imp, admin)
            });

        [erc20_bridge, shared_bridge, l1_nullifier]
            .into_iter()
            .flatten()
            .collect()
    }
}
