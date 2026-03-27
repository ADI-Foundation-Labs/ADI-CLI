//! Builder methods for contract verification targets.

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;

use super::mappings::ContractRegistry;
use super::target::VerificationTarget;
use super::types::{
    ChainAdminVerificationInfo, ContractType, ContractsRoot, ProxyVerificationInfo,
    VerifierVerificationInfo,
};

impl ContractRegistry {
    /// Build verification target for a TransparentUpgradeableProxy contract.
    /// Uses proxy source and includes constructor args for verification.
    ///
    /// The source path uses the @openzeppelin remapping defined in foundry.toml:
    /// `@openzeppelin/contracts-v4/=lib/openzeppelin-contracts-v4/contracts/`
    pub fn build_proxy_target(
        contract_type: ContractType,
        proxy_addr: Address,
        impl_addr: Address,
        proxy_admin_addr: Address,
    ) -> Option<VerificationTarget> {
        // Skip zero addresses
        if proxy_addr.is_zero() || impl_addr.is_zero() || proxy_admin_addr.is_zero() {
            return None;
        }

        let proxy_info = ProxyVerificationInfo {
            impl_addr,
            proxy_admin_addr,
            init_data: alloy_primitives::Bytes::new(),
        };

        Some(VerificationTarget::new_with_proxy(
            contract_type,
            proxy_addr,
            ContractsRoot::L1Contracts.path(),
            "lib/openzeppelin-contracts-v4/contracts/proxy/transparent/TransparentUpgradeableProxy.sol",
            "TransparentUpgradeableProxy",
            true,
            Some(proxy_info),
        ))
    }

    /// Build verification target for ZKsyncOSDualVerifier contract.
    /// Includes constructor args (fflonk, plonk, owner) for verification.
    pub fn build_verifier_target(
        verifier_addr: Address,
        fflonk_addr: Address,
        plonk_addr: Address,
        owner_addr: Option<Address>,
        is_testnet_verifier: Option<bool>,
    ) -> Option<VerificationTarget> {
        // Skip zero addresses
        if verifier_addr.is_zero() || fflonk_addr.is_zero() || plonk_addr.is_zero() {
            return None;
        }

        let verifier_info = VerifierVerificationInfo {
            fflonk_addr,
            plonk_addr,
            owner_addr,
        };

        // Select source path and contract name based on verifier type
        // - ZKsyncOS with testnet: ZKsyncOSTestnetVerifier
        // - ZKsyncOS without testnet: ZKsyncOSDualVerifier
        // - Era (no owner): EraDualVerifier or EraTestnetVerifier
        let (source_path, contract_name) = match (owner_addr.is_some(), is_testnet_verifier) {
            (true, Some(true)) => (
                "state-transition/verifiers/ZKsyncOSTestnetVerifier.sol",
                "ZKsyncOSTestnetVerifier",
            ),
            (true, _) => (
                "state-transition/verifiers/ZKsyncOSDualVerifier.sol",
                "ZKsyncOSDualVerifier",
            ),
            (false, Some(true)) => (
                "state-transition/verifiers/EraTestnetVerifier.sol",
                "EraTestnetVerifier",
            ),
            (false, _) => (
                "state-transition/verifiers/EraDualVerifier.sol",
                "EraDualVerifier",
            ),
        };

        Some(VerificationTarget::new_with_verifier(
            ContractType::Verifier,
            verifier_addr,
            ContractsRoot::L1Contracts.path(),
            source_path,
            contract_name,
            false,
            Some(verifier_info),
        ))
    }

    /// Build verification target for ChainAdmin contract.
    /// Includes constructor args (restrictions array) for verification.
    pub fn build_chain_admin_target(
        contract_type: ContractType,
        addr: Address,
        owner_addr: Address,
    ) -> Option<VerificationTarget> {
        // Skip zero addresses
        if addr.is_zero() || owner_addr.is_zero() {
            return None;
        }

        // ChainAdminOwnable is deployed with tokenMultiplierSetter = address(0)
        let chain_admin_info = ChainAdminVerificationInfo {
            owner_addr,
            token_multiplier_setter: Address::ZERO,
        };

        Some(VerificationTarget::new_with_chain_admin(
            contract_type,
            addr,
            ContractsRoot::L1Contracts.path(),
            Self::source_path(contract_type),
            Self::contract_name(contract_type),
            false,
            Some(chain_admin_info),
        ))
    }

    /// Build all verification targets from ecosystem contracts.
    /// Skips contracts that are unavailable in the toolkit.
    pub fn build_ecosystem_targets(contracts: &EcosystemContracts) -> Vec<VerificationTarget> {
        let mut targets = Vec::new();

        // Helper macro to add target if available
        macro_rules! add_target {
            ($contract_type:expr, $addr:expr) => {
                if let Some(target) = Self::build_target($contract_type, $addr) {
                    targets.push(target);
                }
            };
        }

        // Helper macro to add proxy target with constructor args
        macro_rules! add_proxy_target {
            ($contract_type:expr, $proxy_addr:expr, $impl_addr:expr, $proxy_admin:expr) => {
                if let Some(target) =
                    Self::build_proxy_target($contract_type, $proxy_addr, $impl_addr, $proxy_admin)
                {
                    targets.push(target);
                }
            };
        }

        // Get proxy admin address (used for all TransparentUpgradeableProxy contracts)
        let proxy_admin_addr = contracts
            .core_ecosystem_contracts
            .as_ref()
            .and_then(|c| c.transparent_proxy_admin_addr);

        // Core ecosystem proxy contracts (with proxy verification info)
        if let (Some(proxy_addr), Some(ctm)) =
            (contracts.bridgehub_addr(), &contracts.zksync_os_ctm)
        {
            if let (Some(impl_addr), Some(admin)) = (ctm.bridgehub_impl_addr, proxy_admin_addr) {
                add_proxy_target!(ContractType::Bridgehub, proxy_addr, impl_addr, admin);
            }
        }

        if let Some(core) = &contracts.core_ecosystem_contracts {
            // Message Root proxy
            if let (Some(proxy_addr), Some(ctm)) =
                (core.message_root_proxy_addr, &contracts.zksync_os_ctm)
            {
                if let (Some(impl_addr), Some(admin)) =
                    (ctm.message_root_impl_addr, proxy_admin_addr)
                {
                    add_proxy_target!(ContractType::MessageRoot, proxy_addr, impl_addr, admin);
                }
            }

            // TransparentProxyAdmin (not a proxy itself, skip)
            if let Some(addr) = core.transparent_proxy_admin_addr {
                add_target!(ContractType::TransparentProxyAdmin, addr);
            }

            // STM Deployment Tracker proxy
            if let (Some(proxy_addr), Some(ctm)) = (
                core.stm_deployment_tracker_proxy_addr,
                &contracts.zksync_os_ctm,
            ) {
                if let (Some(impl_addr), Some(admin)) =
                    (ctm.stm_deployment_tracker_impl_addr, proxy_admin_addr)
                {
                    add_proxy_target!(
                        ContractType::StmDeploymentTracker,
                        proxy_addr,
                        impl_addr,
                        admin
                    );
                }
            }

            // Native Token Vault proxy
            if let (Some(proxy_addr), Some(ctm)) =
                (core.native_token_vault_addr, &contracts.zksync_os_ctm)
            {
                if let (Some(impl_addr), Some(admin)) =
                    (ctm.native_token_vault_impl_addr, proxy_admin_addr)
                {
                    add_proxy_target!(ContractType::NativeTokenVault, proxy_addr, impl_addr, admin);
                }
            }
        }

        // Governance contracts
        if let Some(addr) = contracts.governance_addr() {
            add_target!(ContractType::Governance, addr);
        }

        // ChainAdmin with constructor args (owner, tokenMultiplierSetter)
        if let (Some(addr), Some(owner)) =
            (contracts.chain_admin_addr(), contracts.chain_admin_owner)
        {
            if let Some(target) =
                Self::build_chain_admin_target(ContractType::ChainAdmin, addr, owner)
            {
                targets.push(target);
            }
        }

        // ZkSync OS CTM contracts
        if let Some(ctm) = &contracts.zksync_os_ctm {
            // StateTransitionProxy (ChainTypeManager proxy)
            if let (Some(proxy_addr), Some(impl_addr), Some(admin)) = (
                ctm.state_transition_proxy_addr,
                ctm.chain_type_manager_impl_addr,
                proxy_admin_addr,
            ) {
                add_proxy_target!(
                    ContractType::StateTransitionProxy,
                    proxy_addr,
                    impl_addr,
                    admin
                );
            }

            // ValidatorTimelock proxy
            if let (Some(proxy_addr), Some(impl_addr), Some(admin)) = (
                ctm.validator_timelock_addr,
                ctm.validator_timelock_impl_addr,
                proxy_admin_addr,
            ) {
                add_proxy_target!(
                    ContractType::ValidatorTimelock,
                    proxy_addr,
                    impl_addr,
                    admin
                );
            }

            // ServerNotifier proxy
            if let (Some(proxy_addr), Some(impl_addr), Some(admin)) = (
                ctm.server_notifier_proxy_addr,
                ctm.server_notifier_impl_addr,
                proxy_admin_addr,
            ) {
                add_proxy_target!(ContractType::ServerNotifier, proxy_addr, impl_addr, admin);
            }

            // Verifier with constructor args
            // ZKsyncOSDualVerifier: (fflonk, plonk, owner)
            // ZKsyncOSTestnetVerifier: (fflonk, plonk, owner) - testnet variant
            // EraDualVerifier: (fflonk, plonk) - owner is None
            // EraTestnetVerifier: (fflonk, plonk) - testnet variant without owner
            if let (Some(verifier_addr), Some(fflonk), Some(plonk)) = (
                ctm.verifier_addr,
                ctm.verifier_fflonk_addr,
                ctm.verifier_plonk_addr,
            ) {
                if let Some(target) = Self::build_verifier_target(
                    verifier_addr,
                    fflonk,
                    plonk,
                    ctm.verifier_owner_addr,
                    ctm.is_testnet_verifier,
                ) {
                    targets.push(target);
                }
            }
            if let Some(addr) = ctm.l1_rollup_da_manager {
                add_target!(ContractType::L1RollupDaManager, addr);
            }
            if let Some(addr) = ctm.l1_bytecodes_supplier_addr {
                add_target!(ContractType::L1BytecodesSupplier, addr);
            }
            if let Some(addr) = ctm.rollup_l1_da_validator_addr {
                add_target!(ContractType::RollupL1DaValidator, addr);
            }
            if let Some(addr) = ctm.no_da_validium_l1_validator_addr {
                add_target!(ContractType::NoDaValidiumL1Validator, addr);
            }
            if let Some(addr) = ctm.blobs_zksync_os_l1_da_validator_addr {
                add_target!(ContractType::BlobsZkSyncOsL1DaValidator, addr);
            }
            if let Some(addr) = ctm.avail_l1_da_validator_addr {
                add_target!(ContractType::AvailL1DaValidator, addr);
            }
            if let Some(addr) = ctm.default_upgrade_addr {
                add_target!(ContractType::DefaultUpgrade, addr);
            }
            if let Some(addr) = ctm.genesis_upgrade_addr {
                add_target!(ContractType::GenesisUpgrade, addr);
            }

            // Diamond facets (extracted from diamond_cut_data)
            if let Some(addr) = ctm.admin_facet_addr {
                add_target!(ContractType::AdminFacet, addr);
            }
            if let Some(addr) = ctm.executor_facet_addr {
                add_target!(ContractType::ExecutorFacet, addr);
            }
            if let Some(addr) = ctm.mailbox_facet_addr {
                add_target!(ContractType::MailboxFacet, addr);
            }
            if let Some(addr) = ctm.getters_facet_addr {
                add_target!(ContractType::GettersFacet, addr);
            }
            if let Some(addr) = ctm.diamond_init_addr {
                add_target!(ContractType::DiamondInit, addr);
            }

            // Implementation contracts (read via EIP-1967)
            if let Some(addr) = ctm.bridgehub_impl_addr {
                add_target!(ContractType::BridgehubImpl, addr);
            }
            if let Some(addr) = ctm.message_root_impl_addr {
                add_target!(ContractType::MessageRootImpl, addr);
            }
            if let Some(addr) = ctm.native_token_vault_impl_addr {
                add_target!(ContractType::NativeTokenVaultImpl, addr);
            }
            if let Some(addr) = ctm.stm_deployment_tracker_impl_addr {
                add_target!(ContractType::StmDeploymentTrackerImpl, addr);
            }
            if let Some(addr) = ctm.chain_type_manager_impl_addr {
                add_target!(ContractType::ChainTypeManagerImpl, addr);
            }
            if let Some(addr) = ctm.server_notifier_impl_addr {
                add_target!(ContractType::ServerNotifierImpl, addr);
            }
            if let Some(addr) = ctm.erc20_bridge_impl_addr {
                add_target!(ContractType::Erc20BridgeImpl, addr);
            }
            if let Some(addr) = ctm.shared_bridge_impl_addr {
                add_target!(ContractType::SharedBridgeImpl, addr);
            }
            if let Some(addr) = ctm.l1_nullifier_impl_addr {
                add_target!(ContractType::L1NullifierImpl, addr);
            }
            if let Some(addr) = ctm.validator_timelock_impl_addr {
                add_target!(ContractType::ValidatorTimelockImpl, addr);
            }

            // Verifier components
            if let Some(addr) = ctm.verifier_fflonk_addr {
                add_target!(ContractType::VerifierFflonk, addr);
            }
            if let Some(addr) = ctm.verifier_plonk_addr {
                add_target!(ContractType::VerifierPlonk, addr);
            }

            // Bridge token contracts
            if let Some(addr) = ctm.bridged_standard_erc20_addr {
                add_target!(ContractType::BridgedStandardErc20, addr);
            }
            if let Some(addr) = ctm.bridged_token_beacon_addr {
                add_target!(ContractType::BridgedTokenBeacon, addr);
            }

            // Avail test contracts
            if let Some(addr) = ctm.dummy_avail_bridge_addr {
                add_target!(ContractType::DummyAvailBridge, addr);
            }
            if let Some(addr) = ctm.dummy_vector_x_addr {
                add_target!(ContractType::DummyVectorX, addr);
            }

            // Server notifier proxy admin
            if let Some(addr) = ctm.server_notifier_proxy_admin_addr {
                add_target!(ContractType::ServerNotifierProxyAdmin, addr);
            }

            // L1 Wrapped Base Token Store
            if let Some(addr) = ctm.l1_wrapped_base_token_store {
                add_target!(ContractType::L1WrappedBaseTokenStore, addr);
            }
        }

        // Bridge contracts (proxies)
        if let (Some(bridges), Some(ctm)) = (&contracts.bridges, &contracts.zksync_os_ctm) {
            // ERC20 Bridge proxy
            if let Some(erc20) = &bridges.erc20 {
                if let (Some(proxy_addr), Some(impl_addr), Some(admin)) = (
                    erc20.l1_address,
                    ctm.erc20_bridge_impl_addr,
                    proxy_admin_addr,
                ) {
                    add_proxy_target!(ContractType::Erc20Bridge, proxy_addr, impl_addr, admin);
                }
            }
            // Shared Bridge (L1 Asset Router) proxy
            if let Some(shared) = &bridges.shared {
                if let (Some(proxy_addr), Some(impl_addr), Some(admin)) = (
                    shared.l1_address,
                    ctm.shared_bridge_impl_addr,
                    proxy_admin_addr,
                ) {
                    add_proxy_target!(ContractType::SharedBridge, proxy_addr, impl_addr, admin);
                }
            }
            // L1 Nullifier proxy
            if let (Some(proxy_addr), Some(impl_addr), Some(admin)) = (
                bridges.l1_nullifier_addr,
                ctm.l1_nullifier_impl_addr,
                proxy_admin_addr,
            ) {
                add_proxy_target!(ContractType::L1Nullifier, proxy_addr, impl_addr, admin);
            }
        }

        targets
    }

    /// Build verification targets from chain contracts.
    /// Skips contracts that are unavailable in the toolkit.
    pub fn build_chain_targets(contracts: &ChainContracts) -> Vec<VerificationTarget> {
        let mut targets = Vec::new();

        if let Some(l1) = &contracts.l1 {
            if let Some(addr) = l1.diamond_proxy_addr {
                if let Some(target) = Self::build_target(ContractType::DiamondProxy, addr) {
                    targets.push(target);
                }
            }
            if let Some(addr) = l1.governance_addr {
                if let Some(target) = Self::build_target(ContractType::ChainGovernance, addr) {
                    targets.push(target);
                }
            }
            // Chain-level ChainAdmin with constructor args (owner, tokenMultiplierSetter)
            if let (Some(addr), Some(owner)) = (l1.chain_admin_addr, l1.chain_admin_owner) {
                if let Some(target) =
                    Self::build_chain_admin_target(ContractType::ChainChainAdmin, addr, owner)
                {
                    targets.push(target);
                }
            }
            if let Some(addr) = l1.access_control_restriction_addr {
                if let Some(target) =
                    Self::build_target(ContractType::AccessControlRestriction, addr)
                {
                    targets.push(target);
                }
            }
            if let Some(addr) = l1.chain_proxy_admin_addr {
                if let Some(target) = Self::build_target(ContractType::ChainProxyAdmin, addr) {
                    targets.push(target);
                }
            }
        }

        targets
    }

    /// Build all verification targets from ecosystem and optional chain contracts.
    pub fn build_all_targets(
        ecosystem: &EcosystemContracts,
        chain: Option<&ChainContracts>,
    ) -> Vec<VerificationTarget> {
        let mut targets = Self::build_ecosystem_targets(ecosystem);
        if let Some(chain_contracts) = chain {
            targets.extend(Self::build_chain_targets(chain_contracts));
        }
        targets
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_type_display() {
        assert_eq!(ContractType::Governance.to_string(), "Governance");
        assert_eq!(ContractType::DiamondProxy.to_string(), "Diamond Proxy");
    }

    #[test]
    fn test_is_chain_level() {
        assert!(ContractType::DiamondProxy.is_chain_level());
        assert!(!ContractType::Governance.is_chain_level());
    }

    #[test]
    fn test_forge_contract_path() {
        // Use build_target_unchecked to bypass zero address check
        let target =
            ContractRegistry::build_target_unchecked(ContractType::Governance, Address::ZERO);
        assert_eq!(
            target.forge_contract_path(),
            "governance/Governance.sol:Governance"
        );
    }

    #[test]
    fn test_unavailable_contracts_skipped() {
        // Use a non-zero address for testing
        let test_addr = Address::repeat_byte(0x11);

        // TransparentProxyAdmin should be unavailable
        assert!(!ContractRegistry::is_available(
            ContractType::TransparentProxyAdmin
        ));
        assert!(
            ContractRegistry::build_target(ContractType::TransparentProxyAdmin, test_addr)
                .is_none()
        );

        // Governance should be available
        assert!(ContractRegistry::is_available(ContractType::Governance));
        assert!(ContractRegistry::build_target(ContractType::Governance, test_addr).is_some());
    }

    #[test]
    fn test_zero_address_skipped() {
        // Zero address should be skipped even for available contracts
        assert!(ContractRegistry::build_target(ContractType::Governance, Address::ZERO).is_none());
    }
}
