//! Balance checking for ETH and ERC20 tokens.

use crate::error::{FundingError, Result};
use crate::provider::FundingProvider;
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;

// Define minimal ERC20 interface for balance queries
sol! {
    /// Minimal ERC20 interface for balance and metadata queries.
    #[sol(rpc)]
    interface IERC20 {
        /// Returns the balance of an account.
        function balanceOf(address account) external view returns (uint256);
        /// Returns the number of decimals.
        function decimals() external view returns (uint8);
        /// Returns the token symbol.
        function symbol() external view returns (string);
    }
}

/// Balance information for a wallet.
#[derive(Clone, Debug)]
pub struct WalletBalance {
    /// Wallet address.
    pub address: Address,
    /// ETH balance in wei.
    pub eth_balance: U256,
    /// ERC20 token balance (if applicable).
    pub token_balance: Option<U256>,
}

/// Check ETH balance for an address.
///
/// # Arguments
///
/// * `provider` - The funding provider.
/// * `address` - The address to check.
///
/// # Errors
///
/// Returns error if the RPC request fails.
pub async fn get_eth_balance(provider: &FundingProvider, address: Address) -> Result<U256> {
    provider.get_eth_balance(address).await
}

/// Check ERC20 token balance for an address.
///
/// # Arguments
///
/// * `provider` - The funding provider.
/// * `token_address` - The ERC20 token contract address.
/// * `wallet_address` - The wallet address to check.
///
/// # Errors
///
/// Returns error if the RPC request fails.
pub async fn get_token_balance(
    provider: &FundingProvider,
    token_address: Address,
    wallet_address: Address,
) -> Result<U256> {
    let contract = IERC20::new(token_address, provider.inner());

    let balance = contract
        .balanceOf(wallet_address)
        .call()
        .await
        .map_err(|e| FundingError::RpcError(format!("ERC20 balanceOf failed: {e}")))?;

    Ok(balance)
}

/// Get token decimals from the contract.
///
/// # Arguments
///
/// * `provider` - The funding provider.
/// * `token_address` - The ERC20 token contract address.
///
/// # Errors
///
/// Returns error if the RPC request fails.
pub async fn get_token_decimals(provider: &FundingProvider, token_address: Address) -> Result<u8> {
    let contract = IERC20::new(token_address, provider.inner());

    let decimals = contract
        .decimals()
        .call()
        .await
        .map_err(|e| FundingError::RpcError(format!("ERC20 decimals failed: {e}")))?;

    Ok(decimals)
}

/// Get token symbol from the contract.
///
/// # Arguments
///
/// * `provider` - The funding provider.
/// * `token_address` - The ERC20 token contract address.
///
/// # Errors
///
/// Returns error if the RPC request fails.
pub async fn get_token_symbol(provider: &FundingProvider, token_address: Address) -> Result<String> {
    let contract = IERC20::new(token_address, provider.inner());

    let symbol = contract
        .symbol()
        .call()
        .await
        .map_err(|e| FundingError::RpcError(format!("ERC20 symbol failed: {e}")))?;

    Ok(symbol)
}

/// Get complete balance information for a wallet.
///
/// # Arguments
///
/// * `provider` - The funding provider.
/// * `wallet_address` - The wallet address to check.
/// * `token_address` - Optional ERC20 token address to also check.
///
/// # Errors
///
/// Returns error if any RPC request fails.
pub async fn get_wallet_balance(
    provider: &FundingProvider,
    wallet_address: Address,
    token_address: Option<Address>,
) -> Result<WalletBalance> {
    let eth_balance = get_eth_balance(provider, wallet_address).await?;

    let token_balance = match token_address {
        Some(token) => Some(get_token_balance(provider, token, wallet_address).await?),
        None => None,
    };

    Ok(WalletBalance {
        address: wallet_address,
        eth_balance,
        token_balance,
    })
}
