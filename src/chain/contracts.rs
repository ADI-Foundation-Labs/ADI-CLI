//! Chain contract addresses.
//!
//! This module defines the contract address structures for chain-level
//! contracts deployed on the settlement layer.

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// Contract addresses for a specific chain deployed on the settlement layer.
///
/// These contracts are specific to an individual chain and include the
/// Diamond proxy (main L2 contract), admin contracts, and bridges.
///
/// # Note
///
/// The actual contracts deployed vary by protocol version. This struct captures
/// the core contracts used in v0.29.x-v0.30.x. Additional contracts are tracked
/// separately in deployment output files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainContracts {
    // ========================================
    // Diamond (Main L2 Contract)
    // ========================================
    /// Diamond proxy contract - the main chain contract on settlement layer.
    ///
    /// This is the primary contract that represents the chain on L1.
    /// It contains facets for execution, admin, mailbox, and getters.
    pub diamond_proxy_addr: Address,

    // ========================================
    // Admin Contracts
    // ========================================
    /// Governance contract for this chain.
    pub governance_addr: Address,

    /// Chain Admin contract for chain-level administration.
    pub chain_admin_addr: Address,

    // ========================================
    // Settlement Layer Bridges
    // ========================================
    /// Shared bridge contract on settlement layer.
    pub settlement_shared_bridge: Address,

    /// ERC20 bridge contract on settlement layer.
    pub settlement_erc20_bridge: Address,

    // ========================================
    // L2 Contracts (deployed on the ZK chain)
    // ========================================
    /// L2 shared bridge contract address.
    pub l2_shared_bridge: Address,

    /// L2 ERC20 bridge contract address.
    pub l2_erc20_bridge: Address,

    /// L2 legacy shared bridge (optional, for backwards compatibility).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l2_legacy_shared_bridge: Option<Address>,

    // ========================================
    // Base Token Bridge (CGT only)
    // ========================================
    /// Base token bridge contract (only set for custom gas token chains).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_token_bridge: Option<Address>,

    // TODO: Include additional contracts for database storage (chain deployment creates ~4-7 contracts):
    //
    // Chain Deployment - Settlement Layer (L1) contracts:
    // - chain_proxy_admin_addr: Address           // ProxyAdmin (per-chain, via Create2AndTransfer)
    // - access_control_restriction_addr: Option<Address> // Access control restriction
    // - multicall3_addr: Option<Address>          // Multicall3 (optional, may pre-exist)
    //
    // Chain Deployment - References to ecosystem contracts (for convenience):
    // - verifier_addr: Address                    // Verifier address (from ecosystem)
    // - validator_timelock_addr: Address          // ValidatorTimelock (from ecosystem)
    // - rollup_l1_da_validator_addr: Address      // RollupL1DAValidator (from ecosystem)
    // - avail_l1_da_validator_addr: Option<Address> // AvailL1DAValidator (from ecosystem)
    // - no_da_validium_l1_validator_addr: Option<Address> // ValidiumL1DAValidator (from ecosystem)
    //
    // Token configuration:
    // - base_token_addr: Address                  // Base token address (0x01 for ETH, ERC20 for CGT)
    // - base_token_asset_id: B256                 // Base token asset ID (keccak256 hash)
    //
    // L2 Contracts (deployed on the ZK chain itself):
    // - testnet_paymaster_addr: Option<Address>   // Testnet paymaster
    // - default_l2_upgrader: Address              // Default L2 upgrader
    // - da_validator_addr: Address                // DA validator (L2 side)
    // - l2_native_token_vault_proxy_addr: Address // L2NativeTokenVault proxy
    // - consensus_registry_addr: Address          // Consensus registry
    // - l2_multicall3_addr: Address               // Multicall3 on L2
    // - timestamp_asserter_addr: Address          // Timestamp asserter
}

impl ChainContracts {
    /// Validates that all required contract addresses are non-zero.
    ///
    /// # Errors
    ///
    /// Returns an error if any required address is zero.
    pub fn validate(&self) -> eyre::Result<()> {
        use eyre::ensure;

        ensure!(
            !self.diamond_proxy_addr.is_zero(),
            "Diamond proxy address cannot be zero"
        );

        ensure!(
            !self.governance_addr.is_zero(),
            "Governance address cannot be zero"
        );

        ensure!(
            !self.chain_admin_addr.is_zero(),
            "Chain admin address cannot be zero"
        );

        Ok(())
    }
}
