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

    /// Build verification target for a contract type and address.
    pub fn build_target(contract_type: ContractType, address: Address) -> VerificationTarget {
        VerificationTarget::new(
            contract_type,
            address,
            Self::source_path(contract_type),
            Self::contract_name(contract_type),
            Self::is_proxy(contract_type),
        )
    }

    /// Build all verification targets from ecosystem contracts.
    pub fn build_ecosystem_targets(contracts: &EcosystemContracts) -> Vec<VerificationTarget> {
        let mut targets = Vec::new();

        // Core ecosystem contracts
        if let Some(addr) = contracts.bridgehub_addr() {
            targets.push(Self::build_target(ContractType::Bridgehub, addr));
        }
        if let Some(core) = &contracts.core_ecosystem_contracts {
            if let Some(addr) = core.message_root_proxy_addr {
                targets.push(Self::build_target(ContractType::MessageRoot, addr));
            }
            if let Some(addr) = core.transparent_proxy_admin_addr {
                targets.push(Self::build_target(
                    ContractType::TransparentProxyAdmin,
                    addr,
                ));
            }
            if let Some(addr) = core.stm_deployment_tracker_proxy_addr {
                targets.push(Self::build_target(ContractType::StmDeploymentTracker, addr));
            }
            if let Some(addr) = core.native_token_vault_addr {
                targets.push(Self::build_target(ContractType::NativeTokenVault, addr));
            }
        }

        // Governance contracts
        if let Some(addr) = contracts.governance_addr() {
            targets.push(Self::build_target(ContractType::Governance, addr));
        }
        if let Some(addr) = contracts.chain_admin_addr() {
            targets.push(Self::build_target(ContractType::ChainAdmin, addr));
        }

        // ZkSync OS CTM contracts
        if let Some(ctm) = &contracts.zksync_os_ctm {
            if let Some(addr) = ctm.state_transition_proxy_addr {
                targets.push(Self::build_target(ContractType::StateTransitionProxy, addr));
            }
            if let Some(addr) = ctm.validator_timelock_addr {
                targets.push(Self::build_target(ContractType::ValidatorTimelock, addr));
            }
            if let Some(addr) = ctm.server_notifier_proxy_addr {
                targets.push(Self::build_target(ContractType::ServerNotifier, addr));
            }
            if let Some(addr) = ctm.verifier_addr {
                targets.push(Self::build_target(ContractType::Verifier, addr));
            }
            if let Some(addr) = ctm.l1_rollup_da_manager {
                targets.push(Self::build_target(ContractType::L1RollupDaManager, addr));
            }
            if let Some(addr) = ctm.l1_bytecodes_supplier_addr {
                targets.push(Self::build_target(ContractType::L1BytecodesSupplier, addr));
            }
            if let Some(addr) = ctm.rollup_l1_da_validator_addr {
                targets.push(Self::build_target(ContractType::RollupL1DaValidator, addr));
            }
            if let Some(addr) = ctm.no_da_validium_l1_validator_addr {
                targets.push(Self::build_target(
                    ContractType::NoDaValidiumL1Validator,
                    addr,
                ));
            }
            if let Some(addr) = ctm.blobs_zksync_os_l1_da_validator_addr {
                targets.push(Self::build_target(
                    ContractType::BlobsZkSyncOsL1DaValidator,
                    addr,
                ));
            }
            if let Some(addr) = ctm.avail_l1_da_validator_addr {
                targets.push(Self::build_target(ContractType::AvailL1DaValidator, addr));
            }
            if let Some(addr) = ctm.default_upgrade_addr {
                targets.push(Self::build_target(ContractType::DefaultUpgrade, addr));
            }
            if let Some(addr) = ctm.genesis_upgrade_addr {
                targets.push(Self::build_target(ContractType::GenesisUpgrade, addr));
            }
        }

        // Bridge contracts
        if let Some(bridges) = &contracts.bridges {
            if let Some(erc20) = &bridges.erc20 {
                if let Some(addr) = erc20.l1_address {
                    targets.push(Self::build_target(ContractType::Erc20Bridge, addr));
                }
            }
            if let Some(shared) = &bridges.shared {
                if let Some(addr) = shared.l1_address {
                    targets.push(Self::build_target(ContractType::SharedBridge, addr));
                }
            }
            if let Some(addr) = bridges.l1_nullifier_addr {
                targets.push(Self::build_target(ContractType::L1Nullifier, addr));
            }
        }

        targets
    }

    /// Build verification targets from chain contracts.
    pub fn build_chain_targets(contracts: &ChainContracts) -> Vec<VerificationTarget> {
        let mut targets = Vec::new();

        if let Some(l1) = &contracts.l1 {
            if let Some(addr) = l1.diamond_proxy_addr {
                targets.push(Self::build_target(ContractType::DiamondProxy, addr));
            }
            if let Some(addr) = l1.governance_addr {
                targets.push(Self::build_target(ContractType::ChainGovernance, addr));
            }
            if let Some(addr) = l1.chain_admin_addr {
                targets.push(Self::build_target(ContractType::ChainChainAdmin, addr));
            }
            if let Some(addr) = l1.chain_proxy_admin_addr {
                targets.push(Self::build_target(ContractType::ChainProxyAdmin, addr));
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
        let target = ContractRegistry::build_target(ContractType::Governance, Address::ZERO);
        assert_eq!(
            target.forge_contract_path(),
            "governance/Governance.sol:Governance"
        );
    }
}
