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
            deployer_eth: eth(1),
            governor_eth: eth(1),
            governor_cgt_units: 5.0, // 5 tokens (converted using actual decimals at runtime)
            operator_eth: eth(5),
            blob_operator_eth: eth(5),
            prove_operator_eth: eth(5),
            execute_operator_eth: eth(5),
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

/// Target funding requirement for a single wallet.
#[derive(Clone, Debug)]
pub struct FundingTarget {
    /// Wallet role.
    pub role: WalletRole,
    /// Target wallet address.
    pub address: Address,
    /// Required ETH amount (in wei).
    pub eth_amount: U256,
    /// Required ERC20 token amount (in token units). None if no token funding needed.
    pub token_amount: Option<U256>,
}

impl FundingTarget {
    /// Create a new funding target.
    pub fn new(role: WalletRole, address: Address, eth_amount: U256) -> Self {
        Self {
            role,
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
