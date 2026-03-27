//! Static mapping methods for contract registry.

use alloy_primitives::Address;

use super::target::VerificationTarget;
use super::types::{ContractType, ContractsRoot};

/// Registry for contract source mappings.
pub struct ContractRegistry;

impl ContractRegistry {
    /// Get the root directory for a contract type.
    pub fn root_path(contract_type: ContractType) -> ContractsRoot {
        match contract_type {
            ContractType::RollupL1DaValidator
            | ContractType::BlobsZkSyncOsL1DaValidator
            | ContractType::AvailL1DaValidator
            | ContractType::DummyAvailBridge
            | ContractType::DummyVectorX => ContractsRoot::DaContracts,
            _ => ContractsRoot::L1Contracts,
        }
    }

    /// Get source file path for a contract type.
    /// Paths are relative to the contracts/ subdirectory of the root path.
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
            ContractType::ChainAdmin => "governance/ChainAdminOwnable.sol",
            ContractType::AccessControlRestriction => "governance/AccessControlRestriction.sol",
            ContractType::StateTransitionProxy => "state-transition/ZKsyncOSChainTypeManager.sol",
            ContractType::ValidatorTimelock => "state-transition/ValidatorTimelock.sol",
            ContractType::ServerNotifier => "governance/ServerNotifier.sol",
            ContractType::Verifier => "state-transition/verifiers/ZKsyncOSDualVerifier.sol",
            ContractType::L1RollupDaManager => {
                "state-transition/data-availability/RollupDAManager.sol"
            }
            ContractType::L1BytecodesSupplier => "upgrades/BytecodesSupplier.sol",
            // DA contracts (in da-contracts directory)
            ContractType::RollupL1DaValidator => "RollupL1DAValidator.sol",
            ContractType::BlobsZkSyncOsL1DaValidator => "BlobsL1DAValidatorZKsyncOS.sol",
            ContractType::AvailL1DaValidator => "da-layers/avail/AvailL1DAValidator.sol",
            // L1 contracts (in l1-contracts directory)
            ContractType::NoDaValidiumL1Validator => {
                "state-transition/data-availability/ValidiumL1DAValidator.sol"
            }
            ContractType::DefaultUpgrade => "upgrades/DefaultUpgrade.sol",
            ContractType::GenesisUpgrade => "upgrades/L1GenesisUpgrade.sol",
            ContractType::Erc20Bridge => "bridge/L1ERC20Bridge.sol",
            ContractType::SharedBridge => "bridge/asset-router/L1AssetRouter.sol",
            ContractType::L1Nullifier => "bridge/L1Nullifier.sol",
            ContractType::DiamondProxy => "state-transition/chain-deps/DiamondProxy.sol",
            ContractType::ChainGovernance => "governance/Governance.sol",
            ContractType::ChainChainAdmin => "governance/ChainAdminOwnable.sol",
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
            ContractType::ChainTypeManagerImpl => "state-transition/ZKsyncOSChainTypeManager.sol",
            ContractType::ServerNotifierImpl => "governance/ServerNotifier.sol",
            ContractType::Erc20BridgeImpl => "bridge/L1ERC20Bridge.sol",
            ContractType::SharedBridgeImpl => "bridge/asset-router/L1AssetRouter.sol",
            ContractType::L1NullifierImpl => "bridge/L1Nullifier.sol",
            ContractType::ValidatorTimelockImpl => "state-transition/ValidatorTimelock.sol",
            // Verifier components
            ContractType::VerifierFflonk => "state-transition/verifiers/ZKsyncOSVerifierFflonk.sol",
            ContractType::VerifierPlonk => "state-transition/verifiers/ZKsyncOSVerifierPlonk.sol",
            // Bridge token contracts
            ContractType::BridgedStandardErc20 => "bridge/BridgedStandardERC20.sol",
            ContractType::BridgedTokenBeacon => "bridge/BridgedTokenBeacon.sol",
            // Avail test contracts (in da-contracts directory)
            ContractType::DummyAvailBridge => "da-layers/avail/DummyAvailBridge.sol",
            ContractType::DummyVectorX => "da-layers/avail/DummyVectorX.sol",
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
            ContractType::ChainAdmin => "ChainAdminOwnable",
            ContractType::AccessControlRestriction => "AccessControlRestriction",
            ContractType::StateTransitionProxy => "ZKsyncOSChainTypeManager",
            ContractType::ValidatorTimelock => "ValidatorTimelock",
            ContractType::ServerNotifier => "ServerNotifier",
            ContractType::Verifier => "ZKsyncOSDualVerifier",
            ContractType::L1RollupDaManager => "RollupDAManager",
            ContractType::L1BytecodesSupplier => "BytecodesSupplier",
            ContractType::RollupL1DaValidator => "RollupL1DAValidator",
            ContractType::NoDaValidiumL1Validator => "ValidiumL1DAValidator",
            ContractType::BlobsZkSyncOsL1DaValidator => "BlobsL1DAValidatorZKsyncOS",
            ContractType::AvailL1DaValidator => "AvailL1DAValidator",
            ContractType::DefaultUpgrade => "DefaultUpgrade",
            ContractType::GenesisUpgrade => "L1GenesisUpgrade",
            ContractType::Erc20Bridge => "L1ERC20Bridge",
            ContractType::SharedBridge => "L1AssetRouter",
            ContractType::L1Nullifier => "L1Nullifier",
            ContractType::DiamondProxy => "DiamondProxy",
            ContractType::ChainGovernance => "Governance",
            ContractType::ChainChainAdmin => "ChainAdminOwnable",
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
            ContractType::ChainTypeManagerImpl => "ZKsyncOSChainTypeManager",
            ContractType::ServerNotifierImpl => "ServerNotifier",
            ContractType::Erc20BridgeImpl => "L1ERC20Bridge",
            ContractType::SharedBridgeImpl => "L1AssetRouter",
            ContractType::L1NullifierImpl => "L1Nullifier",
            ContractType::ValidatorTimelockImpl => "ValidatorTimelock",
            // Verifier components
            ContractType::VerifierFflonk => "ZKsyncOSVerifierFflonk",
            ContractType::VerifierPlonk => "ZKsyncOSVerifierPlonk",
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
    /// - BridgedTokenBeacon: External OpenZeppelin UpgradeableBeacon
    /// - L1WrappedBaseTokenStore: Only L2 version exists
    pub fn is_available(contract_type: ContractType) -> bool {
        !matches!(
            contract_type,
            // External contracts (OpenZeppelin)
            ContractType::TransparentProxyAdmin
                | ContractType::ChainProxyAdmin
                | ContractType::ServerNotifierProxyAdmin
                | ContractType::BridgedTokenBeacon // Uses OpenZeppelin UpgradeableBeacon
                // Contracts that don't exist in v30.x
                | ContractType::L1WrappedBaseTokenStore // Only L2 version exists
        )
    }

    /// Get the reason why a contract is unavailable for verification.
    pub fn unavailable_reason(contract_type: ContractType) -> Option<&'static str> {
        match contract_type {
            ContractType::TransparentProxyAdmin
            | ContractType::ChainProxyAdmin
            | ContractType::ServerNotifierProxyAdmin
            | ContractType::BridgedTokenBeacon => {
                Some("External OpenZeppelin contract (verify separately)")
            }
            ContractType::L1WrappedBaseTokenStore => Some("Only L2 version exists in v30.x"),
            _ => None,
        }
    }

    /// Build verification target for a contract type and address.
    /// Returns None if the contract is not available in the toolkit or address is zero.
    pub fn build_target(
        contract_type: ContractType,
        address: Address,
    ) -> Option<VerificationTarget> {
        // Skip zero addresses (contract not deployed)
        if address.is_zero() {
            return None;
        }
        if !Self::is_available(contract_type) {
            return None;
        }
        Some(VerificationTarget::new(
            contract_type,
            address,
            Self::root_path(contract_type).path(),
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
            Self::root_path(contract_type).path(),
            Self::source_path(contract_type),
            Self::contract_name(contract_type),
            Self::is_proxy(contract_type),
        )
    }
}
