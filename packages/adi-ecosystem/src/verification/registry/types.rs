//! Contract types and verification info structs.

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
