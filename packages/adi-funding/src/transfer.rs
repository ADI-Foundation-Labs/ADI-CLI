//! Transfer building and gas estimation.

use crate::config::{WalletRole, ETH_DECIMALS};
use crate::error::Result;
use crate::provider::FundingProvider;
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types::eth::TransactionRequest;
use alloy_sol_types::{sol, SolCall};

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
        /// Token decimals for display formatting.
        decimals: u8,
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
        transfer_type: TransferType,
        gas_estimate: u64,
    ) -> Self {
        Self {
            role,
            from,
            to,
            transfer_type,
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
                format!("{} ETH to {} ({})", format_eth(*amount), self.to, self.role)
            }
            TransferType::Token {
                amount,
                symbol,
                decimals,
                ..
            } => {
                format!(
                    "{} {} to {} ({})",
                    format_with_decimals(*amount, usize::from(*decimals)),
                    symbol,
                    self.to,
                    self.role
                )
            }
        }
    }

    /// Get a short description of the amount being transferred.
    pub fn amount_description(&self) -> String {
        match &self.transfer_type {
            TransferType::Eth { amount } => format!("{} ETH", format_eth(*amount)),
            TransferType::Token {
                amount,
                symbol,
                decimals,
                ..
            } => {
                format!(
                    "{} {}",
                    format_with_decimals(*amount, usize::from(*decimals)),
                    symbol
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

/// Format a token amount for display given the number of decimals.
///
/// Converts from smallest units to human-readable form with up to
/// [`DISPLAY_DECIMALS`] fractional digits.
pub fn format_with_decimals(amount: U256, decimals: usize) -> String {
    let base = U256::from(10);
    let unit = base.pow(U256::from(decimals));
    let display_dec = DISPLAY_DECIMALS.min(decimals);
    let decimal_divisor = base.pow(U256::from(decimals - display_dec));

    let whole = amount / unit;
    let remainder = amount % unit;

    if remainder.is_zero() {
        format!("{whole}")
    } else {
        let frac = remainder / decimal_divisor;
        format!("{whole}.{frac:0width$}", width = display_dec)
    }
}

/// Format ETH amount for display (converts from wei to ETH).
pub fn format_eth(amount: U256) -> String {
    format_with_decimals(amount, ETH_DECIMALS)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use alloy_primitives::Address;

    #[test]
    fn format_with_decimals_18_whole() {
        // 1 ETH = 10^18 wei
        let one_eth = U256::from(10).pow(U256::from(18));
        assert_eq!(format_with_decimals(one_eth, 18), "1");
    }

    #[test]
    fn format_with_decimals_18_fractional() {
        // 1.5 ETH
        let amount = U256::from(15) * U256::from(10).pow(U256::from(17));
        assert_eq!(format_with_decimals(amount, 18), "1.5000");
    }

    #[test]
    fn format_with_decimals_18_zero() {
        assert_eq!(format_with_decimals(U256::ZERO, 18), "0");
    }

    #[test]
    fn format_with_decimals_6_usdc() {
        // 100 USDC = 100_000_000 (6 decimals)
        let amount = U256::from(100_000_000u64);
        assert_eq!(format_with_decimals(amount, 6), "100");
    }

    #[test]
    fn format_with_decimals_6_fractional() {
        // 1.5 USDC = 1_500_000
        let amount = U256::from(1_500_000u64);
        assert_eq!(format_with_decimals(amount, 6), "1.5000");
    }

    #[test]
    fn format_with_decimals_8_btc() {
        // 1 WBTC = 100_000_000 (8 decimals)
        let amount = U256::from(100_000_000u64);
        assert_eq!(format_with_decimals(amount, 8), "1");
    }

    #[test]
    fn format_with_decimals_8_fractional() {
        // 0.1234 WBTC = 12_340_000
        let amount = U256::from(12_340_000u64);
        assert_eq!(format_with_decimals(amount, 8), "0.1234");
    }

    #[test]
    fn format_eth_delegates_correctly() {
        let one_eth = U256::from(10).pow(U256::from(18));
        assert_eq!(format_eth(one_eth), "1");
    }

    #[test]
    fn transfer_description_eth() {
        let addr = Address::ZERO;
        let t = Transfer::eth(
            WalletRole::Deployer,
            addr,
            addr,
            U256::from(10).pow(U256::from(18)),
            21000,
        );
        assert!(t.description().contains("ETH"));
        assert!(t.description().contains("deployer"));
    }

    #[test]
    fn transfer_amount_description_eth() {
        let addr = Address::ZERO;
        let t = Transfer::eth(
            WalletRole::Deployer,
            addr,
            addr,
            U256::from(10).pow(U256::from(18)),
            21000,
        );
        assert_eq!(t.amount_description(), "1 ETH");
    }

    #[test]
    fn transfer_amount_description_token_6_decimals() {
        let addr = Address::ZERO;
        let t = Transfer::token(
            WalletRole::Governor,
            addr,
            addr,
            TransferType::Token {
                token_address: addr,
                amount: U256::from(5_000_000u64), // 5 USDC (6 decimals)
                symbol: "USDC".to_string(),
                decimals: 6,
            },
            60000,
        );
        assert_eq!(t.amount_description(), "5 USDC");
    }

    #[test]
    fn transfer_is_eth() {
        let addr = Address::ZERO;
        let eth = Transfer::eth(WalletRole::Deployer, addr, addr, U256::from(1), 21000);
        let tok = Transfer::token(
            WalletRole::Governor,
            addr,
            addr,
            TransferType::Token {
                token_address: addr,
                amount: U256::from(1),
                symbol: "T".to_string(),
                decimals: 18,
            },
            60000,
        );
        assert!(eth.is_eth());
        assert!(!tok.is_eth());
    }

    #[test]
    fn transfer_amount() {
        let addr = Address::ZERO;
        let eth = Transfer::eth(WalletRole::Deployer, addr, addr, U256::from(42), 21000);
        assert_eq!(eth.amount(), U256::from(42));
    }
}
