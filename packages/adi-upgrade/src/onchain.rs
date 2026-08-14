//! On-chain query helpers using alloy provider.
//!
//! Replaces all `cast call` usage with typed Rust implementations.

use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockId, TransactionRequest};
use alloy_sol_types::{sol, SolCall};
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

    /// Verifier mockVerify interface for testnet detection.
    /// Production verifier reverts with MockVerifierNotSupported.
    /// Testnet verifier returns true for valid inputs.
    interface IVerifierMock {
        function mockVerify(uint256[] memory _publicInputs, uint256[] memory _proof) external view returns (bool);
    }

    /// Minimal ERC20 total-supply interface (L2 base-token read-back).
    #[sol(rpc)]
    interface ITotalSupply {
        /// Returns the token total supply.
        function totalSupply() external view returns (uint256);
    }
}

/// Create an alloy HTTP provider from an RPC URL.
pub fn create_provider(rpc_url: &Url) -> impl Provider + Clone {
    ProviderBuilder::new().connect_http(rpc_url.clone())
}

/// Lowest L1 block at which `contract` has code, found by binary search. The v31
/// discover scan starts here instead of block 0, skipping the empty pre-deployment
/// history that otherwise dominates the phase.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] on RPC failure.
pub async fn deploy_block(provider: &(impl Provider + Clone), contract: Address) -> Result<u64> {
    if has_code_at(provider, contract, 0).await? {
        return Ok(0);
    }
    let mut lo = 0u64;
    let mut hi = provider
        .get_block_number()
        .await
        .map_err(|e| UpgradeError::Config(format!("get_block_number: {e}")))?;
    // The binary search assumes code at `hi`; without it a codeless contract would
    // converge to and return the head block. Surface that so the caller can fall back.
    if !has_code_at(provider, contract, hi).await? {
        return Err(UpgradeError::Config(format!(
            "{contract} has no code at head block {hi}"
        )));
    }
    // Invariant: no code at `lo`, code at `hi`; converge on the first block with code.
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if has_code_at(provider, contract, mid).await? {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(hi)
}

async fn has_code_at(
    provider: &(impl Provider + Clone),
    contract: Address,
    block: u64,
) -> Result<bool> {
    let code = provider
        .get_code_at(contract)
        .block_id(BlockId::from(block))
        .await
        .map_err(|e| UpgradeError::Config(format!("get_code_at {contract}@{block}: {e}")))?;
    Ok(!code.is_empty())
}

/// Returns the owner address of an `Ownable` contract.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] on RPC failure.
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

/// Query a token's total supply (`ERC20.totalSupply()`).
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails or the read reverts.
pub async fn query_total_supply(
    provider: &(impl Provider + Clone),
    token: Address,
) -> Result<U256> {
    ITotalSupply::new(token, provider)
        .totalSupply()
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("Failed to query totalSupply of {token}: {e}")))
}

/// Detect whether the verifier contract is a testnet verifier.
///
/// Calls `mockVerify` on the verifier contract. Testnet verifiers
/// (`ZKsyncOSTestnetVerifier`) accept the call, while production verifiers
/// (`ZKsyncOSDualVerifier`) revert with `MockVerifierNotSupported`.
///
/// Returns `true` for testnet, `false` for production.
pub async fn query_is_testnet_verifier(
    provider: &(impl Provider + Clone),
    verifier: Address,
) -> bool {
    let public_inputs = vec![U256::from(1)];
    let proof = vec![U256::from(13), U256::from(1)];

    let call = IVerifierMock::mockVerifyCall {
        _publicInputs: public_inputs,
        _proof: proof,
    };
    let data = Bytes::from(call.abi_encode());

    let tx = TransactionRequest::default()
        .to(verifier)
        .input(data.into());

    provider.call(tx).await.is_ok()
}
