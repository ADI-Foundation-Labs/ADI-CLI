//! Refund types — entry, target, plan, result, and config.

use alloy_primitives::{Address, B256, U256};
use secrecy::SecretString;

use crate::config::{WalletRole, WalletSource};

/// Configuration for a refund operation.
#[derive(Clone, Debug)]
pub struct RefundConfig {
    /// Address to receive all refunded funds.
    pub receiver: Address,
    /// Optional ERC20 token to also refund.
    pub token_address: Option<Address>,
    /// Gas price safety margin percentage (e.g., 200 = 2x).
    pub gas_multiplier: u64,
}

/// Input entry describing a wallet to refund from.
#[derive(Clone, Debug)]
pub struct RefundEntry {
    /// Wallet role.
    pub role: WalletRole,
    /// Wallet source (ecosystem or chain).
    pub source: WalletSource,
    /// Chain name when source is Chain.
    pub chain_name: Option<String>,
    /// Wallet address.
    pub address: Address,
    /// Private key for signing.
    pub private_key: SecretString,
}

/// A wallet target included in the refund plan (has funds to send).
#[derive(Clone, Debug)]
pub struct RefundTarget {
    /// Wallet role.
    pub role: WalletRole,
    /// Wallet source.
    pub source: WalletSource,
    /// Chain name when source is Chain.
    pub chain_name: Option<String>,
    /// Wallet address.
    pub address: Address,
    /// Private key for signing.
    pub(crate) private_key: SecretString,
    /// Current ETH balance.
    pub eth_balance: U256,
    /// Current token balance (if token configured).
    pub token_balance: Option<U256>,
    /// ETH amount that will be sent (balance minus gas reserve).
    pub sendable_eth: U256,
    /// Token amount that will be sent (full token balance).
    pub sendable_token: U256,
    /// Estimated gas for ETH transfer.
    pub eth_gas_estimate: u64,
    /// Estimated gas for token transfer (0 if no token).
    pub token_gas_estimate: u64,
}

impl RefundTarget {
    /// Display label like "eco deployer" or "chain:mychain operator".
    pub fn label(&self) -> String {
        match (&self.source, &self.chain_name) {
            (WalletSource::Chain, Some(name)) => format!("chain:{} {}", name, self.role),
            _ => format!("{} {}", self.source.prefix(), self.role),
        }
    }
}

/// Complete refund plan ready for execution.
#[derive(Clone, Debug)]
pub struct RefundPlan {
    /// Receiver address.
    pub receiver: Address,
    /// Gas price used for calculations.
    pub gas_price: u128,
    /// Wallets to refund from.
    pub targets: Vec<RefundTarget>,
    /// Optional token address.
    pub token_address: Option<Address>,
    /// Token symbol for display.
    pub token_symbol: Option<String>,
    /// Token decimals for display.
    pub token_decimals: Option<u8>,
    /// Total ETH to be refunded.
    pub total_eth_to_refund: U256,
    /// Total tokens to be refunded.
    pub total_token_to_refund: U256,
}

impl RefundPlan {
    /// Number of targets with funds to refund.
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }

    /// True when there is nothing to refund.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

/// Result of executing a refund plan.
#[derive(Clone, Debug)]
pub struct RefundResult {
    /// Number of successful wallet refunds.
    pub successful: usize,
    /// Number of wallets that failed (continue-on-error).
    pub failed: usize,
    /// Total ETH actually refunded.
    pub total_eth_refunded: U256,
    /// Total tokens actually refunded.
    pub total_token_refunded: U256,
    /// Total gas used across all transfers.
    pub total_gas_used: u64,
    /// Transaction hashes for successful transfers.
    pub tx_hashes: Vec<B256>,
    /// Errors for failed wallets.
    pub errors: Vec<String>,
}
