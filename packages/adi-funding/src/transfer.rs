//! Transfer building and gas estimation.

use crate::config::{WalletRole, ETH_DECIMALS};
use crate::error::Result;
use crate::provider::FundingProvider;
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types::eth::TransactionRequest;
use alloy_sol_types::{sol, SolCall};
use colored::Colorize;

// ERC20 transfer interface
sol! {
    /// ERC20 transfer function interface.
    #[sol(rpc)]
    interface IERC20Transfer {
        /// Transfer tokens to a recipient.
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

/// Represents a pending transfer.
#[derive(Clone, Debug)]
pub struct Transfer {
    /// Wallet role being funded.
    pub role: WalletRole,
    /// Source address.
    pub from: Address,
    /// Destination address.
    pub to: Address,
    /// Transfer type.
    pub transfer_type: TransferType,
    /// Estimated gas units.
    pub gas_estimate: u64,
}

/// Type of transfer.
#[derive(Clone, Debug)]
pub enum TransferType {
    /// ETH transfer with amount in wei.
    Eth {
        /// Amount in wei.
        amount: U256,
    },
    /// ERC20 token transfer.
    Token {
        /// Token contract address.
        token_address: Address,
        /// Amount in token units.
        amount: U256,
        /// Token symbol for display.
        symbol: String,
    },
}

impl Transfer {
    /// Create an ETH transfer.
    pub fn eth(
        role: WalletRole,
        from: Address,
        to: Address,
        amount: U256,
        gas_estimate: u64,
    ) -> Self {
        Self {
            role,
            from,
            to,
            transfer_type: TransferType::Eth { amount },
            gas_estimate,
        }
    }

    /// Create a token transfer.
    pub fn token(
        role: WalletRole,
        from: Address,
        to: Address,
        token_address: Address,
        amount: U256,
        symbol: String,
        gas_estimate: u64,
    ) -> Self {
        Self {
            role,
            from,
            to,
            transfer_type: TransferType::Token {
                token_address,
                amount,
                symbol,
            },
            gas_estimate,
        }
    }

    /// Get the transfer amount.
    pub fn amount(&self) -> U256 {
        match &self.transfer_type {
            TransferType::Eth { amount } => *amount,
            TransferType::Token { amount, .. } => *amount,
        }
    }

    /// Check if this is an ETH transfer.
    pub fn is_eth(&self) -> bool {
        matches!(self.transfer_type, TransferType::Eth { .. })
    }

    /// Get a human-readable description of the transfer.
    pub fn description(&self) -> String {
        match &self.transfer_type {
            TransferType::Eth { amount } => {
                format!(
                    "{} to {} ({})",
                    format!("{} ETH", format_eth(*amount)).green(),
                    self.to.to_string().green(),
                    self.role
                )
            }
            TransferType::Token { amount, symbol, .. } => {
                format!(
                    "{} to {} ({})",
                    format!("{} {}", format_token(*amount), symbol).green(),
                    self.to.to_string().green(),
                    self.role
                )
            }
        }
    }
}

/// Estimate gas for an ETH transfer.
///
/// # Arguments
///
/// * `provider` - The funding provider.
/// * `from` - Source address.
/// * `to` - Destination address.
/// * `amount` - Amount in wei.
///
/// # Errors
///
/// Returns error if gas estimation fails.
pub async fn estimate_eth_transfer_gas(
    provider: &FundingProvider,
    from: Address,
    to: Address,
    amount: U256,
) -> Result<u64> {
    let tx = TransactionRequest::default()
        .from(from)
        .to(to)
        .value(amount);

    provider.estimate_gas(&tx).await
}

/// Estimate gas for an ERC20 transfer.
///
/// # Arguments
///
/// * `provider` - The funding provider.
/// * `from` - Source address.
/// * `to` - Destination address.
/// * `token_address` - ERC20 token contract address.
/// * `amount` - Amount in token units.
///
/// # Errors
///
/// Returns error if gas estimation fails.
pub async fn estimate_token_transfer_gas(
    provider: &FundingProvider,
    from: Address,
    to: Address,
    token_address: Address,
    amount: U256,
) -> Result<u64> {
    // Build the transfer call data
    let call = IERC20Transfer::transferCall { to, amount };
    let calldata = call.abi_encode();

    let tx = TransactionRequest::default()
        .from(from)
        .to(token_address)
        .input(Bytes::from(calldata).into());

    provider.estimate_gas(&tx).await
}

/// Build calldata for an ERC20 transfer.
pub fn build_token_transfer_calldata(to: Address, amount: U256) -> Bytes {
    let call = IERC20Transfer::transferCall { to, amount };
    Bytes::from(call.abi_encode())
}

/// Number of decimal places to display.
const DISPLAY_DECIMALS: usize = 4;

/// Format ETH amount for display (converts from wei to ETH).
fn format_eth(amount: U256) -> String {
    let base = U256::from(10);
    let wei_per_eth = base.pow(U256::from(ETH_DECIMALS));
    let decimal_divisor = base.pow(U256::from(ETH_DECIMALS - DISPLAY_DECIMALS));

    let eth = amount / wei_per_eth;
    let remainder = amount % wei_per_eth;

    if remainder.is_zero() {
        format!("{eth}")
    } else {
        let decimals = remainder / decimal_divisor;
        format!("{eth}.{decimals:0width$}", width = DISPLAY_DECIMALS)
    }
}

/// Format token amount for display (assumes 18 decimals).
fn format_token(amount: U256) -> String {
    format_eth(amount)
}
