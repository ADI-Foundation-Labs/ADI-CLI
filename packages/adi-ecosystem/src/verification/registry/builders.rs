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
        let proxy_admin = contracts
            .core_ecosystem_contracts
            .as_ref()
            .and_then(|c| c.transparent_proxy_admin_addr);

        let mut targets = Self::build_core_proxy_targets(contracts, proxy_admin);
        targets.extend(Self::build_governance_targets(contracts));

        if let Some(ctm) = &contracts.zksync_os_ctm {
            targets.extend(Self::build_ctm_proxy_targets(ctm, proxy_admin));
            targets.extend(Self::build_ctm_simple_targets(ctm));
        }

        if let Some((bridges, ctm)) = contracts
            .bridges
            .as_ref()
            .zip(contracts.zksync_os_ctm.as_ref())
        {
            targets.extend(Self::build_bridge_proxy_targets(bridges, ctm, proxy_admin));
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
