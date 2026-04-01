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

    /// Set proxy verification info.
    #[must_use]
    pub fn with_proxy_info(mut self, proxy_info: ProxyVerificationInfo) -> Self {
        self.proxy_info = Some(proxy_info);
        self
    }

    /// Set verifier verification info.
    #[must_use]
    pub fn with_verifier_info(mut self, verifier_info: VerifierVerificationInfo) -> Self {
        self.verifier_info = Some(verifier_info);
        self
    }

    /// Set chain admin verification info.
    #[must_use]
    pub fn with_chain_admin_info(mut self, chain_admin_info: ChainAdminVerificationInfo) -> Self {
        self.chain_admin_info = Some(chain_admin_info);
        self
    }

    /// Get the full contract path for forge verify-contract.
    /// Format: "path/to/Contract.sol:ContractName"
    pub fn forge_contract_path(&self) -> String {
        format!("{}:{}", self.source_path, self.contract_name)
    }
}
