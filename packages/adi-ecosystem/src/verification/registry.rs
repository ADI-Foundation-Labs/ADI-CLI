//! Contract registry for verification.
//!
//! Maps contract types to their source file paths within zksync-era contracts.

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// Root directory for contract sources within zksync-era.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractsRoot {
    /// L1 contracts: /deps/zksync-era/contracts/l1-contracts
    L1Contracts,
    /// DA contracts: /deps/zksync-era/contracts/da-contracts
    DaContracts,
}

impl ContractsRoot {
    /// Get the filesystem path for this root.
    pub fn path(self) -> &'static str {
        match self {
            Self::L1Contracts => "/deps/zksync-era/contracts/l1-contracts",
            Self::DaContracts => "/deps/zksync-era/contracts/da-contracts",
        }
    }
}

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
    /// Access control restriction.
    AccessControlRestriction,

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
            Self::AccessControlRestriction => "Access Control Restriction",
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

/// Proxy verification parameters for TransparentUpgradeableProxy contracts.
#[derive(Debug, Clone)]
pub struct ProxyVerificationInfo {
    /// Implementation contract address (_logic parameter).
    pub impl_addr: Address,
    /// Proxy admin address (initialOwner parameter).
    pub proxy_admin_addr: Address,
    /// Initialization calldata (_data parameter). Empty bytes if no init.
    pub init_data: alloy_primitives::Bytes,
}

/// Verifier verification parameters for DualVerifier contracts.
/// Supports both ZKsyncOSDualVerifier (has owner) and EraDualVerifier (no owner).
#[derive(Debug, Clone)]
pub struct VerifierVerificationInfo {
    /// Fflonk verifier address.
    pub fflonk_addr: Address,
    /// Plonk verifier address.
    pub plonk_addr: Address,
    /// Initial owner address. Some = ZKsyncOSDualVerifier, None = EraDualVerifier.
    pub owner_addr: Option<Address>,
}

/// ChainAdminOwnable verification parameters.
#[derive(Debug, Clone)]
pub struct ChainAdminVerificationInfo {
    /// Initial owner address.
    pub owner_addr: Address,
    /// Token multiplier setter address (typically zero).
    pub token_multiplier_setter: Address,
}

/// Contract verification target with address and source info.
#[derive(Debug, Clone)]
pub struct VerificationTarget {
    /// Contract type.
    pub contract_type: ContractType,
    /// Contract address.
    pub address: Address,
    /// Root path for the contract sources.
    pub root_path: &'static str,
    /// Source file path relative to the root's contracts/ subdirectory.
    pub source_path: &'static str,
    /// Contract name in Solidity.
    pub contract_name: &'static str,
    /// Whether this is a proxy contract.
    pub is_proxy: bool,
    /// Proxy verification info (for TransparentUpgradeableProxy contracts).
    pub proxy_info: Option<ProxyVerificationInfo>,
    /// Verifier verification info (for ZKsyncOSDualVerifier).
    pub verifier_info: Option<VerifierVerificationInfo>,
    /// ChainAdmin verification info.
    pub chain_admin_info: Option<ChainAdminVerificationInfo>,
}

impl VerificationTarget {
    /// Create a new verification target.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        contract_type: ContractType,
        address: Address,
        root_path: &'static str,
        source_path: &'static str,
        contract_name: &'static str,
        is_proxy: bool,
    ) -> Self {
        Self {
            contract_type,
            address,
            root_path,
            source_path,
            contract_name,
            is_proxy,
            proxy_info: None,
            verifier_info: None,
            chain_admin_info: None,
        }
    }

    /// Create a new verification target with proxy info.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_proxy(
        contract_type: ContractType,
        address: Address,
        root_path: &'static str,
        source_path: &'static str,
        contract_name: &'static str,
        is_proxy: bool,
        proxy_info: Option<ProxyVerificationInfo>,
    ) -> Self {
        Self {
            contract_type,
            address,
            root_path,
            source_path,
            contract_name,
            is_proxy,
            proxy_info,
            verifier_info: None,
            chain_admin_info: None,
        }
    }

    /// Create a new verification target with verifier info.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_verifier(
        contract_type: ContractType,
        address: Address,
        root_path: &'static str,
        source_path: &'static str,
        contract_name: &'static str,
        is_proxy: bool,
        verifier_info: Option<VerifierVerificationInfo>,
    ) -> Self {
        Self {
            contract_type,
            address,
            root_path,
            source_path,
            contract_name,
            is_proxy,
            proxy_info: None,
            verifier_info,
            chain_admin_info: None,
        }
    }

    /// Create a new verification target with chain admin info.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_chain_admin(
        contract_type: ContractType,
        address: Address,
        root_path: &'static str,
        source_path: &'static str,
        contract_name: &'static str,
        is_proxy: bool,
        chain_admin_info: Option<ChainAdminVerificationInfo>,
    ) -> Self {
        Self {
            contract_type,
            address,
            root_path,
            source_path,
            contract_name,
            is_proxy,
            proxy_info: None,
            verifier_info: None,
            chain_admin_info,
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
