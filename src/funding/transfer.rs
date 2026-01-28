//! Transfer operations for ETH and ERC-20 tokens.
//!
//! This module provides functions for checking balances and transferring
//! ETH and ERC-20 tokens on the settlement layer using the `cast` CLI.

use alloy_primitives::{Address, U256};
use eyre::WrapErr;

use crate::error::Result;
use crate::external::CastCli;

/// Result of a balance check operation.
#[derive(Debug, Clone)]
pub struct BalanceResult {
    /// The address that was checked.
    #[allow(dead_code)]
    pub address: Address,
    /// The balance in wei (for ETH) or token units (for ERC-20).
    pub balance: U256,
}

#[allow(dead_code)]
impl BalanceResult {
    /// Returns the balance in ETH (for display purposes).
    ///
    /// Converts wei to ETH by dividing by 10^18.
    pub fn balance_in_eth(&self) -> f64 {
        let wei = self.balance.to_string().parse::<f64>().unwrap_or(0.0);
        wei / 1e18
    }

    /// Checks if the balance meets the required amount.
    pub fn meets_requirement(&self, required: U256) -> bool {
        self.balance >= required
    }
}

/// Check the ETH balance of an address using cast.
///
/// # Arguments
///
/// * `cast` - The CastCli instance.
/// * `address` - The address to check.
/// * `rpc_url` - RPC endpoint URL.
///
/// # Returns
///
/// A `BalanceResult` containing the address and balance in wei.
///
/// # Errors
///
/// Returns an error if the balance check fails.
///
/// # Example
///
/// ```rust,ignore
/// let cast = CastCli::new();
/// let result = check_eth_balance(&cast, address, "http://localhost:8545").await?;
/// println!("Balance: {} ETH", result.balance_in_eth());
/// ```
pub async fn check_eth_balance(
    cast: &CastCli,
    address: Address,
    rpc_url: &str,
) -> Result<BalanceResult> {
    let address_str = format!("{address}");

    let output = cast
        .balance(&address_str, rpc_url)
        .await
        .wrap_err_with(|| format!("Failed to check ETH balance for {address}"))?;

    if !output.success() {
        return Err(eyre::eyre!(
            "Failed to check ETH balance for {}: {}",
            address,
            output.stderr.trim()
        ));
    }

    // Parse the balance from stdout (cast returns balance in wei)
    let balance_str = output.stdout.trim();
    let balance = parse_balance(balance_str)
        .wrap_err_with(|| format!("Failed to parse ETH balance: {balance_str}"))?;

    Ok(BalanceResult { address, balance })
}

/// Check the ERC-20 token balance of an address using cast.
///
/// # Arguments
///
/// * `cast` - The CastCli instance.
/// * `token_contract` - The ERC-20 token contract address.
/// * `address` - The address to check.
/// * `rpc_url` - RPC endpoint URL.
///
/// # Returns
///
/// A `BalanceResult` containing the address and token balance.
///
/// # Errors
///
/// Returns an error if the balance check fails.
///
/// # Example
///
/// ```rust,ignore
/// let cast = CastCli::new();
/// let result = check_token_balance(&cast, token_addr, wallet_addr, "http://localhost:8545").await?;
/// println!("Token balance: {}", result.balance);
/// ```
pub async fn check_token_balance(
    cast: &CastCli,
    token_contract: Address,
    address: Address,
    rpc_url: &str,
) -> Result<BalanceResult> {
    let token_str = format!("{token_contract}");
    let address_str = format!("{address}");

    let output = cast
        .token_balance(&token_str, &address_str, rpc_url)
        .await
        .wrap_err_with(|| {
            format!(
                "Failed to check token balance for {} on contract {}",
                address, token_contract
            )
        })?;

    if !output.success() {
        return Err(eyre::eyre!(
            "Failed to check token balance for {} on contract {}: {}",
            address,
            token_contract,
            output.stderr.trim()
        ));
    }

    // Parse the balance from stdout
    let balance_str = output.stdout.trim();
    let balance = parse_balance(balance_str)
        .wrap_err_with(|| format!("Failed to parse token balance: {balance_str}"))?;

    Ok(BalanceResult { address, balance })
}

/// Result of a transfer operation.
#[derive(Debug, Clone)]
pub struct TransferResult {
    /// The sender address.
    #[allow(dead_code)]
    pub from: Address,
    /// The recipient address.
    #[allow(dead_code)]
    pub to: Address,
    /// The amount transferred.
    #[allow(dead_code)]
    pub amount: U256,
    /// Whether the transfer was successful.
    pub success: bool,
    /// Transaction output (may contain tx hash).
    pub output: String,
}

/// Transfer ETH to an address using cast send.
///
/// # Arguments
///
/// * `cast` - The CastCli instance.
/// * `to` - Recipient address.
/// * `amount` - Amount to send in wei.
/// * `private_key` - Private key for signing (without 0x prefix is fine).
/// * `rpc_url` - RPC endpoint URL.
///
/// # Returns
///
/// A `TransferResult` with the transfer details.
///
/// # Errors
///
/// Returns an error if the transfer fails.
///
/// # Example
///
/// ```rust,ignore
/// let cast = CastCli::new();
/// let result = transfer_eth(
///     &cast,
///     recipient,
///     U256::from(1_000_000_000_000_000_000u128), // 1 ETH
///     "0xprivate_key",
///     "http://localhost:8545",
/// ).await?;
/// ```
pub async fn transfer_eth(
    cast: &CastCli,
    from: Address,
    to: Address,
    amount: U256,
    private_key: &str,
    rpc_url: &str,
) -> Result<TransferResult> {
    let to_str = format!("{to}");
    let amount_str = amount.to_string();

    let output = cast
        .send_eth(&to_str, &amount_str, private_key, rpc_url)
        .await
        .wrap_err_with(|| format!("Failed to send ETH to {to}"))?;

    Ok(TransferResult {
        from,
        to,
        amount,
        success: output.success(),
        output: if output.success() {
            output.stdout
        } else {
            output.stderr
        },
    })
}

/// Transfer ERC-20 tokens to an address using cast send.
///
/// # Arguments
///
/// * `cast` - The CastCli instance.
/// * `token_contract` - The ERC-20 token contract address.
/// * `to` - Recipient address.
/// * `amount` - Amount to send in token units.
/// * `private_key` - Private key for signing.
/// * `rpc_url` - RPC endpoint URL.
///
/// # Returns
///
/// A `TransferResult` with the transfer details.
///
/// # Errors
///
/// Returns an error if the transfer fails.
///
/// # Example
///
/// ```rust,ignore
/// let cast = CastCli::new();
/// let result = transfer_token(
///     &cast,
///     token_addr,
///     recipient,
///     U256::from(5_000_000_000_000_000_000u128), // 5 tokens (18 decimals)
///     "0xprivate_key",
///     "http://localhost:8545",
/// ).await?;
/// ```
pub async fn transfer_token(
    cast: &CastCli,
    token_contract: Address,
    from: Address,
    to: Address,
    amount: U256,
    private_key: &str,
    rpc_url: &str,
) -> Result<TransferResult> {
    let token_str = format!("{token_contract}");
    let to_str = format!("{to}");
    let amount_str = amount.to_string();

    let output = cast
        .send_token(&token_str, &to_str, &amount_str, private_key, rpc_url)
        .await
        .wrap_err_with(|| format!("Failed to send tokens to {to}"))?;

    Ok(TransferResult {
        from,
        to,
        amount,
        success: output.success(),
        output: if output.success() {
            output.stdout
        } else {
            output.stderr
        },
    })
}

/// Parse a balance string from cast output.
///
/// Handles both decimal strings and hex strings (with 0x prefix).
fn parse_balance(balance_str: &str) -> Result<U256> {
    let trimmed = balance_str.trim();

    if let Some(stripped) = trimmed.strip_prefix("0x") {
        // Hex format
        U256::from_str_radix(stripped, 16)
            .map_err(|e| eyre::eyre!("Failed to parse hex balance: {e}"))
    } else {
        // Decimal format
        trimmed
            .parse::<U256>()
            .map_err(|e| eyre::eyre!("Failed to parse decimal balance: {e}"))
    }
}

/// Funding requirement for a wallet.
#[derive(Debug, Clone)]
pub struct FundingRequirement {
    /// The wallet address to fund.
    pub address: Address,
    /// Human-readable wallet name (e.g., "deployer", "governor").
    pub name: String,
    /// Required ETH balance in wei.
    pub required_eth: U256,
    /// Required token balance (if CGT is used).
    pub required_token: Option<U256>,
}

impl FundingRequirement {
    /// Creates a new funding requirement.
    pub fn new(address: Address, name: impl Into<String>, required_eth: U256) -> Self {
        Self {
            address,
            name: name.into(),
            required_eth,
            required_token: None,
        }
    }

    /// Sets the required token balance (for CGT chains).
    pub fn with_token(mut self, amount: U256) -> Self {
        self.required_token = Some(amount);
        self
    }

    /// Returns the required ETH in displayable format.
    #[allow(dead_code)]
    pub fn required_eth_display(&self) -> String {
        let eth = self.required_eth.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        format!("{eth:.2} ETH")
    }
}

/// Funding status for a wallet.
#[derive(Debug, Clone)]
pub struct FundingStatus {
    /// The wallet being checked.
    pub requirement: FundingRequirement,
    /// Current ETH balance.
    pub current_eth: U256,
    /// Current token balance (if applicable).
    pub current_token: Option<U256>,
}

impl FundingStatus {
    /// Checks if the wallet has sufficient ETH.
    pub fn has_sufficient_eth(&self) -> bool {
        self.current_eth >= self.requirement.required_eth
    }

    /// Checks if the wallet has sufficient tokens (if required).
    pub fn has_sufficient_token(&self) -> bool {
        match (self.requirement.required_token, self.current_token) {
            (Some(required), Some(current)) => current >= required,
            (Some(_), None) => false,
            (None, _) => true,
        }
    }

    /// Checks if all funding requirements are met.
    pub fn is_funded(&self) -> bool {
        self.has_sufficient_eth() && self.has_sufficient_token()
    }

    /// Returns the ETH deficit (how much more is needed).
    pub fn eth_deficit(&self) -> U256 {
        if self.current_eth >= self.requirement.required_eth {
            U256::ZERO
        } else {
            self.requirement.required_eth - self.current_eth
        }
    }

    /// Returns the token deficit (if applicable).
    pub fn token_deficit(&self) -> Option<U256> {
        match (self.requirement.required_token, self.current_token) {
            (Some(required), Some(current)) => {
                if current >= required {
                    Some(U256::ZERO)
                } else {
                    Some(required - current)
                }
            }
            (Some(required), None) => Some(required),
            (None, _) => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_result_meets_requirement() {
        let result = BalanceResult {
            address: Address::ZERO,
            balance: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
        };

        assert!(result.meets_requirement(U256::from(500_000_000_000_000_000u128)));
        assert!(!result.meets_requirement(U256::from(2_000_000_000_000_000_000u128)));
    }

    #[test]
    fn test_balance_in_eth() {
        let result = BalanceResult {
            address: Address::ZERO,
            balance: U256::from(1_500_000_000_000_000_000u128), // 1.5 ETH
        };

        let eth = result.balance_in_eth();
        assert!((eth - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_parse_balance_decimal() {
        let balance = parse_balance("1000000000000000000").unwrap();
        assert_eq!(balance, U256::from(1_000_000_000_000_000_000u128));
    }

    #[test]
    fn test_parse_balance_hex() {
        let balance = parse_balance("0xde0b6b3a7640000").unwrap();
        assert_eq!(balance, U256::from(1_000_000_000_000_000_000u128));
    }

    #[test]
    fn test_funding_status() {
        let requirement = FundingRequirement::new(
            Address::ZERO,
            "deployer",
            U256::from(1_000_000_000_000_000_000u128),
        );

        let status = FundingStatus {
            requirement,
            current_eth: U256::from(500_000_000_000_000_000u128),
            current_token: None,
        };

        assert!(!status.has_sufficient_eth());
        assert!(status.has_sufficient_token());
        assert!(!status.is_funded());
        assert_eq!(
            status.eth_deficit(),
            U256::from(500_000_000_000_000_000u128)
        );
    }
}
