//! Verification target definition.

use alloy_primitives::Address;

use super::types::{
    ChainAdminVerificationInfo, ContractType, ProxyVerificationInfo, VerifierVerificationInfo,
};

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
