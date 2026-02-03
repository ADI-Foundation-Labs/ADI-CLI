//! Error types for funding operations.

use alloy_primitives::{Address, U256};
use thiserror::Error;

/// Result type alias using FundingError.
pub type Result<T> = std::result::Result<T, FundingError>;

/// Errors that can occur during funding operations.
#[derive(Error, Debug)]
pub enum FundingError {
    /// Failed to connect to RPC provider.
    #[error("Failed to connect to RPC endpoint '{url}': {reason}")]
    ProviderConnection {
        /// RPC URL that failed.
        url: String,
        /// Failure reason.
        reason: String,
    },

    /// Invalid private key format.
    #[error("Invalid private key format: {0}")]
    InvalidPrivateKey(String),

    /// Failed to parse address.
    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    /// Funder has insufficient ETH balance.
    #[error("Insufficient ETH balance: have {have}, need {need} (includes {gas_estimate} for gas)")]
    InsufficientEthBalance {
        /// Current balance.
        have: U256,
        /// Required balance.
        need: U256,
        /// Estimated gas cost.
        gas_estimate: U256,
    },

    /// Funder has insufficient ERC20 token balance.
    #[error("Insufficient {symbol} balance: have {have}, need {need}")]
    InsufficientTokenBalance {
        /// Token symbol.
        symbol: String,
        /// Current balance.
        have: U256,
        /// Required balance.
        need: U256,
    },

    /// Target wallet not found in wallets configuration.
    #[error("Wallet role '{role}' not found in wallets configuration")]
    WalletNotFound {
        /// Wallet role name.
        role: String,
    },

    /// RPC request failed.
    #[error("RPC request failed: {0}")]
    RpcError(String),

    /// Transaction failed to execute.
    #[error("Transaction failed for {to}: {reason}")]
    TransactionFailed {
        /// Target address.
        to: Address,
        /// Failure reason.
        reason: String,
    },

    /// Transaction was reverted.
    #[error("Transaction reverted: {0}")]
    TransactionReverted(String),

    /// Gas estimation failed.
    #[error("Gas estimation failed: {0}")]
    GasEstimationFailed(String),

    /// No wallets require funding.
    #[error("No wallets require funding - all balances meet requirements")]
    NoFundingRequired,

    /// Configuration error.
    #[error("Invalid funding configuration: {0}")]
    InvalidConfig(String),
}
