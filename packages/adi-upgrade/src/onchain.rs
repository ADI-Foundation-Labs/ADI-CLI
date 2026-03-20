//! On-chain query helpers using alloy provider.
//!
//! Replaces all `cast call` usage with typed Rust implementations.

use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_sol_types::sol;
use url::Url;

use crate::error::{Result, UpgradeError};

// Solidity function signatures for on-chain queries.
sol! {
    /// Ownable contract interface for querying owner.
    #[sol(rpc)]
    interface Ownable {
        /// Returns the owner address.
        function owner() external view returns (address);
    }

    /// Bridgehub contract interface for chain queries.
    #[sol(rpc)]
    interface IBridgehub {
        /// Returns the chain type manager for a given chain ID.
        function chainTypeManager(uint256 chainId) external view returns (address);
        /// Returns the ZK chain diamond proxy for a given chain ID.
        function getZKChain(uint256 chainId) external view returns (address);
    }

    /// ZkSync Hyperchain (diamond proxy) interface.
    #[sol(rpc)]
    interface IZkSyncHyperchain {
        /// Returns the admin address.
        function getAdmin() external view returns (address);
        /// Returns the verifier address.
        function getVerifier() external view returns (address);
        /// Returns the current protocol version.
        function getProtocolVersion() external view returns (uint256);
    }

    /// Chain type manager interface.
    #[sol(rpc)]
    interface IChainTypeManager {
        /// Returns the current protocol version.
        function protocolVersion() external view returns (uint256);
    }
}

/// Create an alloy HTTP provider from an RPC URL.
pub fn create_provider(rpc_url: &Url) -> impl Provider + Clone {
    ProviderBuilder::new().connect_http(rpc_url.clone())
}

/// Query owner of a contract (`Ownable.owner()`).
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn query_owner(provider: &(impl Provider + Clone), contract: Address) -> Result<Address> {
    let instance = Ownable::new(contract, provider);
    instance
        .owner()
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query owner of {contract}: {e}")))
}

/// Query chain type manager address from bridgehub.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn query_ctm(
    provider: &(impl Provider + Clone),
    bridgehub: Address,
    chain_id: u64,
) -> Result<Address> {
    let instance = IBridgehub::new(bridgehub, provider);
    instance
        .chainTypeManager(U256::from(chain_id))
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query CTM for chain {chain_id}: {e}")))
}

/// Query ZK chain diamond proxy address from bridgehub.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn query_zk_chain(
    provider: &(impl Provider + Clone),
    bridgehub: Address,
    chain_id: u64,
) -> Result<Address> {
    let instance = IBridgehub::new(bridgehub, provider);
    instance
        .getZKChain(U256::from(chain_id))
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query ZK chain for {chain_id}: {e}")))
}

/// Query admin of a diamond proxy.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn query_admin(provider: &(impl Provider + Clone), diamond: Address) -> Result<Address> {
    let instance = IZkSyncHyperchain::new(diamond, provider);
    instance
        .getAdmin()
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query admin of {diamond}: {e}")))
}

/// Query verifier address of a diamond proxy.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn query_verifier(
    provider: &(impl Provider + Clone),
    diamond: Address,
) -> Result<Address> {
    let instance = IZkSyncHyperchain::new(diamond, provider);
    instance
        .getVerifier()
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query verifier of {diamond}: {e}")))
}

/// Query protocol version from a chain type manager.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn query_ctm_protocol_version(
    provider: &(impl Provider + Clone),
    ctm: Address,
) -> Result<U256> {
    let instance = IChainTypeManager::new(ctm, provider);
    instance
        .protocolVersion()
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query CTM protocol version: {e}")))
}

/// Query protocol version from a diamond proxy.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn query_diamond_protocol_version(
    provider: &(impl Provider + Clone),
    diamond: Address,
) -> Result<U256> {
    let instance = IZkSyncHyperchain::new(diamond, provider);
    instance
        .getProtocolVersion()
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query diamond protocol version: {e}")))
}
