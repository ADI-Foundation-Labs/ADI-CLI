//! Contract registry for verification.
//!
//! Maps contract types to their source file paths within era-contracts.

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// Contract type identifier for verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractType {
    // Core ecosystem contracts
    /// Bridgehub proxy contract.
    Bridgehub,
    /// Message root proxy contract.
    MessageRoot,
    /// Transparent proxy admin.
    TransparentProxyAdmin,
    /// STM deployment tracker proxy.
    StmDeploymentTracker,
    /// Native token vault.
    NativeTokenVault,

    // Governance contracts
    /// Governance contract.
    Governance,
    /// Chain admin contract.
    ChainAdmin,

    // State transition contracts
    /// State transition proxy (Chain Type Manager).
    StateTransitionProxy,
    /// Validator timelock.
    ValidatorTimelock,
    /// Server notifier proxy.
    ServerNotifier,
    /// Verifier contract.
    Verifier,

    // DA validators
    /// L1 Rollup DA manager.
    L1RollupDaManager,
    /// L1 bytecodes supplier.
    L1BytecodesSupplier,
    /// Rollup L1 DA validator.
    RollupL1DaValidator,
    /// No DA validium L1 validator.
    NoDaValidiumL1Validator,
    /// Blobs ZkSync OS L1 DA validator.
    BlobsZkSyncOsL1DaValidator,
    /// Avail L1 DA validator.
    AvailL1DaValidator,

    // Upgrade contracts
    /// Default upgrade contract.
    DefaultUpgrade,
    /// Genesis upgrade contract.
    GenesisUpgrade,

    // Bridge contracts
    /// ERC20 bridge.
    Erc20Bridge,
    /// Shared bridge (L1 Asset Router).
    SharedBridge,
    /// L1 Nullifier.
    L1Nullifier,

    // Chain-level contracts
    /// Diamond proxy (chain).
    DiamondProxy,
    /// Chain governance.
    ChainGovernance,
    /// Chain admin (chain-level).
    ChainChainAdmin,
    /// Chain proxy admin.
    ChainProxyAdmin,

    // Diamond facets (extracted from diamond_cut_data)
    /// Admin facet.
    AdminFacet,
    /// Executor facet.
    ExecutorFacet,
    /// Mailbox facet.
    MailboxFacet,
    /// Getters facet.
    GettersFacet,
    /// Diamond init contract.
    DiamondInit,

    // Implementation contracts (read via EIP-1967)
    /// Bridgehub implementation.
    BridgehubImpl,
    /// Message root implementation.
    MessageRootImpl,
    /// Native token vault implementation.
    NativeTokenVaultImpl,
    /// STM deployment tracker implementation.
    StmDeploymentTrackerImpl,
    /// Chain type manager implementation.
    ChainTypeManagerImpl,
    /// Server notifier implementation.
    ServerNotifierImpl,
    /// ERC20 bridge implementation.
    Erc20BridgeImpl,
    /// Shared bridge implementation.
    SharedBridgeImpl,
    /// L1 Nullifier implementation.
    L1NullifierImpl,
    /// Validator timelock implementation.
    ValidatorTimelockImpl,

    // Verifier components
    /// ZKsyncOS Verifier Fflonk.
    VerifierFflonk,
    /// ZKsyncOS Verifier Plonk.
    VerifierPlonk,

    // Bridge token contracts
    /// Bridged Standard ERC20.
    BridgedStandardErc20,
    /// Bridged Token Beacon.
    BridgedTokenBeacon,

    // Avail test contracts
    /// Dummy Avail Bridge.
    DummyAvailBridge,
    /// Dummy VectorX.
    DummyVectorX,

    /// Server notifier proxy admin.
    ServerNotifierProxyAdmin,

    /// L1 Wrapped Base Token Store.
    L1WrappedBaseTokenStore,
}

impl ContractType {
    /// Get the display name for this contract type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Bridgehub => "Bridgehub",
            Self::MessageRoot => "Message Root",
            Self::TransparentProxyAdmin => "Transparent Proxy Admin",
            Self::StmDeploymentTracker => "STM Deployment Tracker",
            Self::NativeTokenVault => "Native Token Vault",
            Self::Governance => "Governance",
            Self::ChainAdmin => "Chain Admin",
            Self::StateTransitionProxy => "State Transition Manager",
            Self::ValidatorTimelock => "Validator Timelock",
            Self::ServerNotifier => "Server Notifier",
            Self::Verifier => "Verifier",
            Self::L1RollupDaManager => "L1 Rollup DA Manager",
            Self::L1BytecodesSupplier => "L1 Bytecodes Supplier",
            Self::RollupL1DaValidator => "Rollup L1 DA Validator",
            Self::NoDaValidiumL1Validator => "No DA Validium L1 Validator",
            Self::BlobsZkSyncOsL1DaValidator => "Blobs ZkSync OS L1 DA Validator",
            Self::AvailL1DaValidator => "Avail L1 DA Validator",
            Self::DefaultUpgrade => "Default Upgrade",
            Self::GenesisUpgrade => "Genesis Upgrade",
            Self::Erc20Bridge => "ERC20 Bridge",
            Self::SharedBridge => "Shared Bridge",
            Self::L1Nullifier => "L1 Nullifier",
            Self::DiamondProxy => "Diamond Proxy",
            Self::ChainGovernance => "Chain Governance",
            Self::ChainChainAdmin => "Chain Admin (Chain)",
            Self::ChainProxyAdmin => "Chain Proxy Admin",
            // Diamond facets
            Self::AdminFacet => "Admin Facet",
            Self::ExecutorFacet => "Executor Facet",
            Self::MailboxFacet => "Mailbox Facet",
            Self::GettersFacet => "Getters Facet",
            Self::DiamondInit => "Diamond Init",
            // Implementation contracts
            Self::BridgehubImpl => "Bridgehub Impl",
            Self::MessageRootImpl => "Message Root Impl",
            Self::NativeTokenVaultImpl => "Native Token Vault Impl",
            Self::StmDeploymentTrackerImpl => "STM Deployment Tracker Impl",
            Self::ChainTypeManagerImpl => "Chain Type Manager Impl",
            Self::ServerNotifierImpl => "Server Notifier Impl",
            Self::Erc20BridgeImpl => "ERC20 Bridge Impl",
            Self::SharedBridgeImpl => "Shared Bridge Impl",
            Self::L1NullifierImpl => "L1 Nullifier Impl",
            Self::ValidatorTimelockImpl => "Validator Timelock Impl",
            // Verifier components
            Self::VerifierFflonk => "Verifier Fflonk",
            Self::VerifierPlonk => "Verifier Plonk",
            // Bridge token contracts
            Self::BridgedStandardErc20 => "Bridged Standard ERC20",
            Self::BridgedTokenBeacon => "Bridged Token Beacon",
            // Avail test contracts
            Self::DummyAvailBridge => "Dummy Avail Bridge",
            Self::DummyVectorX => "Dummy VectorX",
            // Server notifier proxy admin
            Self::ServerNotifierProxyAdmin => "Server Notifier Proxy Admin",
            // L1 Wrapped Base Token Store
            Self::L1WrappedBaseTokenStore => "L1 Wrapped Base Token Store",
        }
    }

    /// Check if this is a chain-level contract.
    pub fn is_chain_level(&self) -> bool {
        matches!(
            self,
            Self::DiamondProxy
                | Self::ChainGovernance
                | Self::ChainChainAdmin
                | Self::ChainProxyAdmin
        )
    }
}

impl std::fmt::Display for ContractType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Contract verification target with address and source info.
#[derive(Debug, Clone)]
pub struct VerificationTarget {
    /// Contract type.
    pub contract_type: ContractType,
    /// Contract address.
    pub address: Address,
    /// Source file path relative to era-contracts/l1-contracts/contracts/.
    pub source_path: &'static str,
    /// Contract name in Solidity.
    pub contract_name: &'static str,
    /// Whether this is a proxy contract.
    pub is_proxy: bool,
}

impl VerificationTarget {
    /// Create a new verification target.
    pub fn new(
        contract_type: ContractType,
        address: Address,
        source_path: &'static str,
        contract_name: &'static str,
        is_proxy: bool,
    ) -> Self {
        Self {
            contract_type,
            address,
            source_path,
            contract_name,
            is_proxy,
        }
    }

    /// Get the full contract path for forge verify-contract.
    /// Format: "path/to/Contract.sol:ContractName"
    pub fn forge_contract_path(&self) -> String {
        format!("{}:{}", self.source_path, self.contract_name)
    }
}

/// Registry for contract source mappings.
pub struct ContractRegistry;

impl ContractRegistry {
    /// Get source file path for a contract type.
    /// Paths are relative to /deps/era-contracts/l1-contracts/contracts/
    pub fn source_path(contract_type: ContractType) -> &'static str {
        match contract_type {
            ContractType::Bridgehub => "bridgehub/L1Bridgehub.sol",
            ContractType::MessageRoot => "bridgehub/L1MessageRoot.sol",
            ContractType::TransparentProxyAdmin => {
                "transparent-proxy/TransparentUpgradeableProxy.sol"
            }
            ContractType::StmDeploymentTracker => "bridgehub/CTMDeploymentTracker.sol",
            ContractType::NativeTokenVault => "bridge/ntv/L1NativeTokenVault.sol",
            ContractType::Governance => "governance/Governance.sol",
            ContractType::ChainAdmin => "governance/ChainAdmin.sol",
            ContractType::StateTransitionProxy => "state-transition/ChainTypeManager.sol",
            ContractType::ValidatorTimelock => "state-transition/ValidatorTimelock.sol",
            ContractType::ServerNotifier => "state-transition/ServerNotifier.sol",
            ContractType::Verifier => "verifier/Verifier.sol",
            ContractType::L1RollupDaManager => {
                "state-transition/data-availability/L1RollupDAManager.sol"
            }
            ContractType::L1BytecodesSupplier => "state-transition/L1BytecodesSupplier.sol",
            ContractType::RollupL1DaValidator => {
                "state-transition/data-availability/RollupL1DAValidator.sol"
            }
            ContractType::NoDaValidiumL1Validator => {
                "state-transition/data-availability/ValidiumL1DAValidator.sol"
            }
            ContractType::BlobsZkSyncOsL1DaValidator => {
                "state-transition/data-availability/BlobsRollupL1DAValidator.sol"
            }
            ContractType::AvailL1DaValidator => {
                "state-transition/data-availability/AvailL1DAValidator.sol"
            }
            ContractType::DefaultUpgrade => "upgrades/DefaultUpgrade.sol",
            ContractType::GenesisUpgrade => "upgrades/GenesisUpgrade.sol",
            ContractType::Erc20Bridge => "bridge/L1ERC20Bridge.sol",
            ContractType::SharedBridge => "bridge/asset-router/L1AssetRouter.sol",
            ContractType::L1Nullifier => "bridge/L1Nullifier.sol",
            ContractType::DiamondProxy => "state-transition/chain-deps/DiamondProxy.sol",
            ContractType::ChainGovernance => "governance/Governance.sol",
            ContractType::ChainChainAdmin => "governance/ChainAdmin.sol",
            ContractType::ChainProxyAdmin => "transparent-proxy/TransparentUpgradeableProxy.sol",
            // Diamond facets
            ContractType::AdminFacet => "state-transition/chain-deps/facets/Admin.sol",
            ContractType::ExecutorFacet => "state-transition/chain-deps/facets/Executor.sol",
            ContractType::MailboxFacet => "state-transition/chain-deps/facets/Mailbox.sol",
            ContractType::GettersFacet => "state-transition/chain-deps/facets/Getters.sol",
            ContractType::DiamondInit => "state-transition/chain-deps/DiamondInit.sol",
            // Implementation contracts (same source as proxy, different instance)
            ContractType::BridgehubImpl => "bridgehub/L1Bridgehub.sol",
            ContractType::MessageRootImpl => "bridgehub/L1MessageRoot.sol",
            ContractType::NativeTokenVaultImpl => "bridge/ntv/L1NativeTokenVault.sol",
            ContractType::StmDeploymentTrackerImpl => "bridgehub/CTMDeploymentTracker.sol",
            ContractType::ChainTypeManagerImpl => "state-transition/ChainTypeManager.sol",
            ContractType::ServerNotifierImpl => "state-transition/ServerNotifier.sol",
            ContractType::Erc20BridgeImpl => "bridge/L1ERC20Bridge.sol",
            ContractType::SharedBridgeImpl => "bridge/asset-router/L1AssetRouter.sol",
            ContractType::L1NullifierImpl => "bridge/L1Nullifier.sol",
            ContractType::ValidatorTimelockImpl => "state-transition/ValidatorTimelock.sol",
            // Verifier components
            ContractType::VerifierFflonk => "verifier/ZKsyncOsVerifierFflonk.sol",
            ContractType::VerifierPlonk => "verifier/ZKsyncOsVerifierPlonk.sol",
            // Bridge token contracts
            ContractType::BridgedStandardErc20 => "bridge/BridgedStandardERC20.sol",
            ContractType::BridgedTokenBeacon => "bridge/BridgedTokenBeacon.sol",
            // Avail test contracts
            ContractType::DummyAvailBridge => {
                "state-transition/data-availability/DummyAvailBridge.sol"
            }
            ContractType::DummyVectorX => "state-transition/data-availability/DummyVectorX.sol",
            // Server notifier proxy admin
            ContractType::ServerNotifierProxyAdmin => {
                "transparent-proxy/TransparentUpgradeableProxy.sol"
            }
            // L1 Wrapped Base Token Store
            ContractType::L1WrappedBaseTokenStore => "bridge/L1WrappedBaseTokenStore.sol",
        }
    }

    /// Get contract name for a contract type.
    pub fn contract_name(contract_type: ContractType) -> &'static str {
        match contract_type {
            ContractType::Bridgehub => "L1Bridgehub",
            ContractType::MessageRoot => "L1MessageRoot",
            ContractType::TransparentProxyAdmin => "TransparentUpgradeableProxy",
            ContractType::StmDeploymentTracker => "CTMDeploymentTracker",
            ContractType::NativeTokenVault => "L1NativeTokenVault",
            ContractType::Governance => "Governance",
            ContractType::ChainAdmin => "ChainAdmin",
            ContractType::StateTransitionProxy => "ChainTypeManager",
            ContractType::ValidatorTimelock => "ValidatorTimelock",
            ContractType::ServerNotifier => "ServerNotifier",
            ContractType::Verifier => "Verifier",
            ContractType::L1RollupDaManager => "L1RollupDAManager",
            ContractType::L1BytecodesSupplier => "L1BytecodesSupplier",
            ContractType::RollupL1DaValidator => "RollupL1DAValidator",
            ContractType::NoDaValidiumL1Validator => "ValidiumL1DAValidator",
            ContractType::BlobsZkSyncOsL1DaValidator => "BlobsRollupL1DAValidator",
            ContractType::AvailL1DaValidator => "AvailL1DAValidator",
            ContractType::DefaultUpgrade => "DefaultUpgrade",
            ContractType::GenesisUpgrade => "GenesisUpgrade",
            ContractType::Erc20Bridge => "L1ERC20Bridge",
            ContractType::SharedBridge => "L1AssetRouter",
            ContractType::L1Nullifier => "L1Nullifier",
            ContractType::DiamondProxy => "DiamondProxy",
            ContractType::ChainGovernance => "Governance",
            ContractType::ChainChainAdmin => "ChainAdmin",
            ContractType::ChainProxyAdmin => "TransparentUpgradeableProxy",
            // Diamond facets
            ContractType::AdminFacet => "AdminFacet",
            ContractType::ExecutorFacet => "ExecutorFacet",
            ContractType::MailboxFacet => "MailboxFacet",
            ContractType::GettersFacet => "GettersFacet",
            ContractType::DiamondInit => "DiamondInit",
            // Implementation contracts
            ContractType::BridgehubImpl => "L1Bridgehub",
            ContractType::MessageRootImpl => "L1MessageRoot",
            ContractType::NativeTokenVaultImpl => "L1NativeTokenVault",
            ContractType::StmDeploymentTrackerImpl => "CTMDeploymentTracker",
            ContractType::ChainTypeManagerImpl => "ChainTypeManager",
            ContractType::ServerNotifierImpl => "ServerNotifier",
            ContractType::Erc20BridgeImpl => "L1ERC20Bridge",
            ContractType::SharedBridgeImpl => "L1AssetRouter",
            ContractType::L1NullifierImpl => "L1Nullifier",
            ContractType::ValidatorTimelockImpl => "ValidatorTimelock",
            // Verifier components
            ContractType::VerifierFflonk => "ZKsyncOsVerifierFflonk",
            ContractType::VerifierPlonk => "ZKsyncOsVerifierPlonk",
            // Bridge token contracts
            ContractType::BridgedStandardErc20 => "BridgedStandardERC20",
            ContractType::BridgedTokenBeacon => "BridgedTokenBeacon",
            // Avail test contracts
            ContractType::DummyAvailBridge => "DummyAvailBridge",
            ContractType::DummyVectorX => "DummyVectorX",
            // Server notifier proxy admin
            ContractType::ServerNotifierProxyAdmin => "TransparentUpgradeableProxy",
            // L1 Wrapped Base Token Store
            ContractType::L1WrappedBaseTokenStore => "L1WrappedBaseTokenStore",
        }
    }

    /// Check if a contract type is a proxy.
    pub fn is_proxy(contract_type: ContractType) -> bool {
        matches!(
            contract_type,
            ContractType::Bridgehub
                | ContractType::MessageRoot
                | ContractType::TransparentProxyAdmin
                | ContractType::StmDeploymentTracker
                | ContractType::NativeTokenVault
                | ContractType::StateTransitionProxy
                | ContractType::ServerNotifier
                | ContractType::DiamondProxy
                | ContractType::ChainProxyAdmin
        )
    }

    /// Check if a contract type is available for verification in the toolkit.
    ///
    /// Some contracts don't exist in the current toolkit image:
    /// - TransparentUpgradeableProxy: External OpenZeppelin contract
    /// - RollupL1DAValidator: Not in v30.x toolkit
    /// - BlobsRollupL1DAValidator: Not in v30.x toolkit
    /// - AvailL1DAValidator: Not in v30.x toolkit
    /// - DummyAvailBridge: Test contract, not in toolkit
    /// - DummyVectorX: Test contract, not in toolkit
    pub fn is_available(contract_type: ContractType) -> bool {
        !matches!(
            contract_type,
            // External contracts (OpenZeppelin)
            ContractType::TransparentProxyAdmin
                | ContractType::ChainProxyAdmin
                | ContractType::ServerNotifierProxyAdmin
                // DA validators not in v30.x toolkit
                | ContractType::RollupL1DaValidator
                | ContractType::BlobsZkSyncOsL1DaValidator
                | ContractType::AvailL1DaValidator
                // Test contracts not in toolkit
                | ContractType::DummyAvailBridge
                | ContractType::DummyVectorX
        )
    }

    /// Get the reason why a contract is unavailable for verification.
    pub fn unavailable_reason(contract_type: ContractType) -> Option<&'static str> {
        match contract_type {
            ContractType::TransparentProxyAdmin
            | ContractType::ChainProxyAdmin
            | ContractType::ServerNotifierProxyAdmin => {
                Some("External OpenZeppelin contract (verify separately)")
            }
            ContractType::RollupL1DaValidator
            | ContractType::BlobsZkSyncOsL1DaValidator
            | ContractType::AvailL1DaValidator => Some("Not available in v30.x toolkit"),
            ContractType::DummyAvailBridge | ContractType::DummyVectorX => {
                Some("Test contract, not in toolkit")
            }
            _ => None,
        }
    }

    /// Build verification target for a contract type and address.
    /// Returns None if the contract is not available in the toolkit.
    pub fn build_target(
        contract_type: ContractType,
        address: Address,
    ) -> Option<VerificationTarget> {
        if !Self::is_available(contract_type) {
            return None;
        }
        Some(VerificationTarget::new(
            contract_type,
            address,
            Self::source_path(contract_type),
            Self::contract_name(contract_type),
            Self::is_proxy(contract_type),
        ))
    }

    /// Build verification target unconditionally (even if unavailable).
    /// Used for listing all contracts regardless of toolkit availability.
    pub fn build_target_unchecked(
        contract_type: ContractType,
        address: Address,
    ) -> VerificationTarget {
        VerificationTarget::new(
            contract_type,
            address,
            Self::source_path(contract_type),
            Self::contract_name(contract_type),
            Self::is_proxy(contract_type),
        )
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

        // Core ecosystem contracts
        if let Some(addr) = contracts.bridgehub_addr() {
            add_target!(ContractType::Bridgehub, addr);
        }
        if let Some(core) = &contracts.core_ecosystem_contracts {
            if let Some(addr) = core.message_root_proxy_addr {
                add_target!(ContractType::MessageRoot, addr);
            }
            if let Some(addr) = core.transparent_proxy_admin_addr {
                add_target!(ContractType::TransparentProxyAdmin, addr);
            }
            if let Some(addr) = core.stm_deployment_tracker_proxy_addr {
                add_target!(ContractType::StmDeploymentTracker, addr);
            }
            if let Some(addr) = core.native_token_vault_addr {
                add_target!(ContractType::NativeTokenVault, addr);
            }
        }

        // Governance contracts
        if let Some(addr) = contracts.governance_addr() {
            add_target!(ContractType::Governance, addr);
        }
        if let Some(addr) = contracts.chain_admin_addr() {
            add_target!(ContractType::ChainAdmin, addr);
        }

        // ZkSync OS CTM contracts
        if let Some(ctm) = &contracts.zksync_os_ctm {
            if let Some(addr) = ctm.state_transition_proxy_addr {
                add_target!(ContractType::StateTransitionProxy, addr);
            }
            if let Some(addr) = ctm.validator_timelock_addr {
                add_target!(ContractType::ValidatorTimelock, addr);
            }
            if let Some(addr) = ctm.server_notifier_proxy_addr {
                add_target!(ContractType::ServerNotifier, addr);
            }
            if let Some(addr) = ctm.verifier_addr {
                add_target!(ContractType::Verifier, addr);
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

        // Bridge contracts
        if let Some(bridges) = &contracts.bridges {
            if let Some(erc20) = &bridges.erc20 {
                if let Some(addr) = erc20.l1_address {
                    add_target!(ContractType::Erc20Bridge, addr);
                }
            }
            if let Some(shared) = &bridges.shared {
                if let Some(addr) = shared.l1_address {
                    add_target!(ContractType::SharedBridge, addr);
                }
            }
            if let Some(addr) = bridges.l1_nullifier_addr {
                add_target!(ContractType::L1Nullifier, addr);
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
            if let Some(addr) = l1.chain_admin_addr {
                if let Some(target) = Self::build_target(ContractType::ChainChainAdmin, addr) {
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
        let target =
            ContractRegistry::build_target(ContractType::Governance, Address::ZERO).unwrap();
        assert_eq!(
            target.forge_contract_path(),
            "governance/Governance.sol:Governance"
        );
    }

    #[test]
    fn test_unavailable_contracts_skipped() {
        // TransparentProxyAdmin should be unavailable
        assert!(!ContractRegistry::is_available(
            ContractType::TransparentProxyAdmin
        ));
        assert!(
            ContractRegistry::build_target(ContractType::TransparentProxyAdmin, Address::ZERO)
                .is_none()
        );

        // Governance should be available
        assert!(ContractRegistry::is_available(ContractType::Governance));
        assert!(ContractRegistry::build_target(ContractType::Governance, Address::ZERO).is_some());
    }
}
