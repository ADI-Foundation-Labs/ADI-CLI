//! Event system for funding progress reporting.

mod logging;
mod spinner;

pub use logging::LoggingEventHandler;
pub use spinner::SpinnerEventHandler;

use crate::config::WalletRole;
use crate::transfer::Transfer;
use alloy_primitives::{Address, B256, U256};

/// Events emitted during funding operations.
#[derive(Clone, Debug)]
pub enum FundingEvent {
    /// Starting balance check phase.
    CheckingBalances {
        /// Number of wallets to check.
        wallet_count: usize,
    },

    /// Balance checked for a wallet.
    BalanceChecked {
        /// Wallet role.
        role: WalletRole,
        /// Wallet address.
        address: Address,
        /// Current ETH balance.
        eth_balance: U256,
        /// Current token balance (if applicable).
        token_balance: Option<U256>,
    },

    /// Plan created with required transfers.
    PlanCreated {
        /// Number of transfers needed.
        transfer_count: usize,
        /// Total ETH required (including gas).
        total_eth: U256,
        /// Total token required.
        total_token: U256,
        /// Estimated gas cost.
        gas_cost: U256,
    },

    /// Starting transfer execution.
    ExecutingTransfers {
        /// Total number of transfers.
        total: usize,
    },

    /// Single transfer started.
    TransferStarted {
        /// Transfer index (0-based).
        index: usize,
        /// Total transfers.
        total: usize,
        /// Transfer details.
        transfer: Transfer,
    },

    /// Transfer transaction submitted.
    TransferSubmitted {
        /// Transfer index.
        index: usize,
        /// Transaction hash.
        tx_hash: B256,
    },

    /// Transfer confirmed.
    TransferConfirmed {
        /// Transfer index.
        index: usize,
        /// Transaction hash.
        tx_hash: B256,
        /// Gas used.
        gas_used: u64,
    },

    /// All transfers complete.
    Complete {
        /// Number of successful transfers.
        successful: usize,
        /// Total gas used.
        total_gas_used: u64,
    },
}

/// Trait for receiving funding events.
///
/// Implement this trait to receive progress updates without coupling to
/// a specific logging framework.
#[async_trait::async_trait]
pub trait FundingEventHandler: Send + Sync {
    /// Handle a funding event.
    async fn on_event(&self, event: FundingEvent);
}

/// No-op event handler that discards all events.
pub struct NoOpEventHandler;

#[async_trait::async_trait]
impl FundingEventHandler for NoOpEventHandler {
    async fn on_event(&self, _event: FundingEvent) {
        // Discard
    }
}
