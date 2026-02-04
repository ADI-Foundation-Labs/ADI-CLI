//! Provider wrapper for alloy HTTP provider.

use crate::error::{FundingError, Result};
use alloy_network::Ethereum;
use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types::eth::TransactionRequest;

/// Type alias for the HTTP provider with Ethereum network.
pub type HttpProvider = RootProvider<Ethereum>;

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
            .map_err(|e| FundingError::RpcError {
                url: self.url.clone(),
                operation: "eth_getBalance",
                reason: format_rpc_error(&e),
            })
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
            .map_err(|e| FundingError::RpcError {
                url: self.url.clone(),
                operation: "eth_gasPrice",
                reason: format_rpc_error(&e),
            })
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
        self.inner.estimate_gas(tx.clone()).await.map_err(|e| {
            let error_str = e.to_string();

            // Detect insufficient funds error and provide clearer message
            if error_str.contains("insufficient funds") {
                let value = tx.value.unwrap_or_default();
                return FundingError::GasEstimationFailed(format!(
                    "Funder has insufficient balance to transfer {} wei. \
                         Top up the funder wallet and try again. RPC error: {}",
                    value, error_str
                ));
            }

            FundingError::GasEstimationFailed(error_str)
        })
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
            .map_err(|e| FundingError::RpcError {
                url: self.url.clone(),
                operation: "eth_chainId",
                reason: format_rpc_error(&e),
            })
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
            .map_err(|e| FundingError::RpcError {
                url: self.url.clone(),
                operation: "eth_getTransactionCount",
                reason: format_rpc_error(&e),
            })
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

/// Format RPC error with helpful context.
///
/// Extracts meaningful information from alloy transport errors
/// and provides suggestions for common issues.
fn format_rpc_error<E: std::fmt::Display>(error: &E) -> String {
    let error_str = error.to_string();

    // Check for common error patterns and provide helpful messages
    if error_str.contains("error sending request") {
        return format!(
            "Connection failed - check if the RPC endpoint is reachable and the URL is correct. \
             Details: {}",
            error_str
        );
    }

    if error_str.contains("timed out") {
        return format!(
            "Request timed out - the RPC endpoint may be slow or unreachable. Details: {}",
            error_str
        );
    }

    if error_str.contains("401") || error_str.contains("403") {
        return format!(
            "Authentication failed - check if your RPC URL includes valid API credentials. \
             Details: {}",
            error_str
        );
    }

    if error_str.contains("429") {
        return format!(
            "Rate limited - too many requests to the RPC endpoint. Try again later. Details: {}",
            error_str
        );
    }

    if error_str.contains("502") || error_str.contains("503") || error_str.contains("504") {
        return format!(
            "RPC endpoint unavailable - the server may be down or overloaded. Details: {}",
            error_str
        );
    }

    // Default: return the original error
    error_str
}
