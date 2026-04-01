//! Funding configuration and target amounts.

use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// ETH has 18 decimal places.
pub const ETH_DECIMALS: usize = 18;

/// Convert ETH amount to wei using pow.
fn eth(amount: u64) -> U256 {
    U256::from(amount) * U256::from(10).pow(U256::from(ETH_DECIMALS))
}

/// Convert fractional ETH (in tenths) to wei.
fn eth_tenths(tenths: u64) -> U256 {
    U256::from(tenths) * U256::from(10).pow(U256::from(ETH_DECIMALS - 1))
}

/// Default ETH amounts per wallet role (in wei).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DefaultAmounts {
    /// Deployer: 1 ETH.
    pub deployer_eth: U256,
    /// Governor: 1 ETH.
    pub governor_eth: U256,
    /// Governor custom gas token amount in token units (supports fractional amounts).
    /// Only used when custom base token is configured.
    /// Converted to actual amount at runtime using token decimals.
    pub governor_cgt_units: f64,
    /// Operator: 5 ETH.
    pub operator_eth: U256,
    /// Blob operator: 5 ETH.
    pub blob_operator_eth: U256,
    /// Prove operator: 5 ETH.
    pub prove_operator_eth: U256,
    /// Execute operator: 5 ETH.
    pub execute_operator_eth: U256,
    /// Fee account: 0 ETH (typically funded by fees).
    pub fee_account_eth: U256,
    /// Token multiplier setter: 0.1 ETH.
    pub token_multiplier_setter_eth: U256,
}

impl DefaultAmounts {
    /// Convert governor CGT units to smallest token units using token decimals.
    ///
    /// Uses precise multiplication to avoid floating-point errors:
    /// 1. Multiply f64 by 10^decimals
    /// 2. Round to nearest integer
    /// 3. Clamp to prevent overflow
    pub fn governor_cgt_amount(&self, token_decimals: u8) -> U256 {
        if self.governor_cgt_units <= 0.0 {
            return U256::ZERO;
        }
        let multiplier = 10_f64.powi(i32::from(token_decimals));
        let smallest_units = self.governor_cgt_units * multiplier;
        let clamped = smallest_units.min(u128::MAX as f64);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let amount_u128 = clamped.round() as u128;
        U256::from(amount_u128)
    }
}

impl Default for DefaultAmounts {
    fn default() -> Self {
        Self {
            deployer_eth: eth(100),
            governor_eth: eth(40),
            governor_cgt_units: 5.0, // 5 tokens (converted using actual decimals at runtime)
            operator_eth: eth(30),
            blob_operator_eth: eth(5),
            prove_operator_eth: eth(30),
            execute_operator_eth: eth(30),
            fee_account_eth: U256::ZERO,
            token_multiplier_setter_eth: eth_tenths(1), // 0.1 ETH
        }
    }
}

/// Wallet role enumeration for funding targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletRole {
    /// Contract deployer.
    Deployer,
    /// Chain operator.
    Operator,
    /// Blob operations operator.
    BlobOperator,
    /// Proof submission operator.
    ProveOperator,
    /// Transaction execution operator.
    ExecuteOperator,
    /// Fee collection account.
    FeeAccount,
    /// Governance wallet.
    Governor,
    /// Token multiplier configuration.
    TokenMultiplierSetter,
}

impl WalletRole {
    /// Returns a human-readable name for the role.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Deployer => "deployer",
            Self::Operator => "operator",
            Self::BlobOperator => "blob_operator",
            Self::ProveOperator => "prove_operator",
            Self::ExecuteOperator => "execute_operator",
            Self::FeeAccount => "fee_account",
            Self::Governor => "governor",
            Self::TokenMultiplierSetter => "token_multiplier_setter",
        }
    }
}

impl std::fmt::Display for WalletRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Source of a wallet (ecosystem-level or chain-level).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalletSource {
    /// Ecosystem-level wallet.
    Ecosystem,
    /// Chain-level wallet.
    Chain,
}

impl WalletSource {
    /// Returns a short prefix for display.
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Ecosystem => "eco",
            Self::Chain => "chain",
        }
    }
}

/// Target funding requirement for a single wallet.
#[derive(Clone, Debug)]
pub struct FundingTarget {
    /// Wallet role.
    pub role: WalletRole,
    /// Wallet source (ecosystem or chain).
    pub source: WalletSource,
    /// Target wallet address.
    pub address: Address,
    /// Required ETH amount (in wei).
    pub eth_amount: U256,
    /// Required ERC20 token amount (in token units). None if no token funding needed.
    pub token_amount: Option<U256>,
}

impl FundingTarget {
    /// Create a new funding target.
    pub fn new(role: WalletRole, source: WalletSource, address: Address, eth_amount: U256) -> Self {
        Self {
            role,
            source,
            address,
            eth_amount,
            token_amount: None,
        }
    }

    /// Add token funding requirement.
    pub fn with_token(mut self, amount: U256) -> Self {
        self.token_amount = Some(amount);
        self
    }
}

/// A funding target with current balance and status for display.
///
/// This struct captures both the required funding amounts and current balances,
/// allowing unified display of funding plans (similar to `AnvilFundingTarget`).
#[derive(Clone, Debug)]
pub struct FundingTargetStatus {
    /// Wallet role (deployer, governor, operator, etc.).
    pub role: WalletRole,
    /// Wallet source (ecosystem or chain).
    pub source: WalletSource,
    /// Wallet address.
    pub address: Address,
    /// Required ETH amount (in wei).
    pub required_eth: U256,
    /// Required token amount (if custom gas token is configured).
    pub required_token: Option<U256>,
    /// Current ETH balance.
    pub current_eth: U256,
    /// Current token balance (if custom gas token is configured).
    pub current_token: Option<U256>,
    /// Whether ETH funding is needed.
    pub needs_eth_funding: bool,
    /// Whether token funding is needed.
    pub needs_token_funding: bool,
}

/// Funding configuration for an operation.
#[derive(Clone, Debug)]
pub struct FundingConfig {
    /// RPC endpoint URL.
    pub rpc_url: String,
    /// Optional ERC20 token address (for custom gas token funding).
    pub token_address: Option<Address>,
    /// Optional token symbol for display (queried from chain if not set).
    pub token_symbol: Option<String>,
    /// Default amounts per role.
    pub default_amounts: DefaultAmounts,
    /// Gas price multiplier (percentage, e.g., 120 = 20% buffer).
    pub gas_price_multiplier: u64,
}

impl Default for FundingConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::new(),
            token_address: None,
            token_symbol: None,
            default_amounts: DefaultAmounts::default(),
            gas_price_multiplier: 120, // 20% buffer
        }
    }
}

impl FundingConfig {
    /// Create a new funding config with the given RPC URL.
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            ..Default::default()
        }
    }

    /// Set the custom gas token address and optional symbol.
    ///
    /// If symbol is not provided, it will be queried from the chain.
    pub fn with_token(mut self, address: Address, symbol: Option<String>) -> Self {
        self.token_address = Some(address);
        self.token_symbol = symbol;
        self
    }

    /// Set the gas price multiplier (percentage).
    pub fn with_gas_multiplier(mut self, multiplier: u64) -> Self {
        self.gas_price_multiplier = multiplier;
        self
    }

    /// Override default amounts.
    pub fn with_amounts(mut self, amounts: DefaultAmounts) -> Self {
        self.default_amounts = amounts;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governor_cgt_amount_18_decimals() {
        let amounts = DefaultAmounts::default();
        // 5.0 tokens with 18 decimals = 5 * 10^18
        let result = amounts.governor_cgt_amount(18);
        let expected = U256::from(5) * U256::from(10).pow(U256::from(18));
        assert_eq!(result, expected);
    }

    #[test]
    fn governor_cgt_amount_6_decimals() {
        let amounts = DefaultAmounts::default();
        // 5.0 tokens with 6 decimals = 5_000_000
        let result = amounts.governor_cgt_amount(6);
        assert_eq!(result, U256::from(5_000_000u64));
    }

    #[test]
    fn governor_cgt_amount_zero_units() {
        let amounts = DefaultAmounts {
            governor_cgt_units: 0.0,
            ..DefaultAmounts::default()
        };
        assert_eq!(amounts.governor_cgt_amount(18), U256::ZERO);
    }

    #[test]
    fn governor_cgt_amount_negative_units() {
        let amounts = DefaultAmounts {
            governor_cgt_units: -1.0,
            ..DefaultAmounts::default()
        };
        assert_eq!(amounts.governor_cgt_amount(18), U256::ZERO);
    }

    #[test]
    fn wallet_role_display_name_all_variants() {
        assert_eq!(WalletRole::Deployer.display_name(), "deployer");
        assert_eq!(WalletRole::Operator.display_name(), "operator");
        assert_eq!(WalletRole::BlobOperator.display_name(), "blob_operator");
        assert_eq!(WalletRole::ProveOperator.display_name(), "prove_operator");
        assert_eq!(
            WalletRole::ExecuteOperator.display_name(),
            "execute_operator"
        );
        assert_eq!(WalletRole::FeeAccount.display_name(), "fee_account");
        assert_eq!(WalletRole::Governor.display_name(), "governor");
        assert_eq!(
            WalletRole::TokenMultiplierSetter.display_name(),
            "token_multiplier_setter"
        );
    }

    #[test]
    fn default_amounts_non_zero() {
        let amounts = DefaultAmounts::default();
        assert!(!amounts.deployer_eth.is_zero());
        assert!(!amounts.governor_eth.is_zero());
        assert!(!amounts.operator_eth.is_zero());
        assert!(!amounts.blob_operator_eth.is_zero());
        assert!(amounts.fee_account_eth.is_zero()); // Intentionally zero
        assert!(!amounts.token_multiplier_setter_eth.is_zero());
    }

    #[test]
    fn funding_target_with_token() {
        let target = FundingTarget::new(
            WalletRole::Governor,
            WalletSource::Ecosystem,
            Address::ZERO,
            U256::from(1),
        )
        .with_token(U256::from(500));

        assert_eq!(target.token_amount, Some(U256::from(500)));
    }

    #[test]
    fn funding_config_builder() {
        let config = FundingConfig::new("http://localhost:8545")
            .with_gas_multiplier(150)
            .with_token(Address::ZERO, Some("USDC".to_string()));

        assert_eq!(config.rpc_url, "http://localhost:8545");
        assert_eq!(config.gas_price_multiplier, 150);
        assert_eq!(config.token_address, Some(Address::ZERO));
        assert_eq!(config.token_symbol, Some("USDC".to_string()));
    }
}
