//! Provider wrapper for alloy HTTP provider.

use crate::error::{FundingError, Result};
use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types::eth::TransactionRequest;

/// Type alias for the HTTP provider.
pub type HttpProvider = RootProvider;

/// Wrapper around alloy provider with funding-specific methods.
pub struct FundingProvider {
    inner: HttpProvider,
    url: String,
}

impl FundingProvider {
    /// Create a new provider connecting to the given RPC URL.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - RPC endpoint URL (e.g., "http://localhost:8545").
    ///
    /// # Errors
    ///
    /// Returns error if the URL is invalid or connection fails.
    pub fn new(rpc_url: &str) -> Result<Self> {
        let url = rpc_url
            .parse()
            .map_err(|e| FundingError::ProviderConnection {
                url: rpc_url.to_string(),
                reason: format!("Invalid URL: {e}"),
            })?;

        let provider = RootProvider::new_http(url);

        Ok(Self {
            inner: provider,
            url: rpc_url.to_string(),
        })
    }

    /// Get the ETH balance of an address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address to check.
    ///
    /// # Errors
    ///
    /// Returns error if the RPC request fails.
    pub async fn get_eth_balance(&self, address: Address) -> Result<U256> {
        self.inner
            .get_balance(address)
            .await
            .map_err(|e| FundingError::RpcError(e.to_string()))
    }

    /// Get current gas price.
    ///
    /// # Errors
    ///
    /// Returns error if the RPC request fails.
    pub async fn get_gas_price(&self) -> Result<u128> {
        self.inner
            .get_gas_price()
            .await
            .map_err(|e| FundingError::RpcError(e.to_string()))
    }

    /// Estimate gas for a transaction.
    ///
    /// # Arguments
    ///
    /// * `tx` - The transaction request to estimate.
    ///
    /// # Errors
    ///
    /// Returns error if gas estimation fails.
    pub async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u64> {
        self.inner
            .estimate_gas(tx.clone())
            .await
            .map_err(|e| FundingError::GasEstimationFailed(e.to_string()))
    }

    /// Get the chain ID.
    ///
    /// # Errors
    ///
    /// Returns error if the RPC request fails.
    pub async fn get_chain_id(&self) -> Result<u64> {
        self.inner
            .get_chain_id()
            .await
            .map_err(|e| FundingError::RpcError(e.to_string()))
    }

    /// Get the nonce for an address.
    ///
    /// # Arguments
    ///
    /// * `address` - The address to get the nonce for.
    ///
    /// # Errors
    ///
    /// Returns error if the RPC request fails.
    pub async fn get_nonce(&self, address: Address) -> Result<u64> {
        self.inner
            .get_transaction_count(address)
            .await
            .map_err(|e| FundingError::RpcError(e.to_string()))
    }

    /// Get the inner provider reference.
    pub fn inner(&self) -> &HttpProvider {
        &self.inner
    }

    /// Get the RPC URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}
