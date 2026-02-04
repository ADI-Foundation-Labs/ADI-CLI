//! Event system for funding progress reporting.

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

/// Event handler that logs events using the log crate.
pub struct LoggingEventHandler;

#[async_trait::async_trait]
impl FundingEventHandler for LoggingEventHandler {
    async fn on_event(&self, event: FundingEvent) {
        match event {
            FundingEvent::CheckingBalances { wallet_count } => {
                log::info!("Checking balances for {} wallets", wallet_count);
            }
            FundingEvent::BalanceChecked {
                role,
                address,
                eth_balance,
                token_balance,
            } => {
                if let Some(tok) = token_balance {
                    log::debug!(
                        "{}: {} has {} wei ETH, {} token",
                        role,
                        address,
                        eth_balance,
                        tok
                    );
                } else {
                    log::debug!("{}: {} has {} wei ETH", role, address, eth_balance);
                }
            }
            FundingEvent::PlanCreated {
                transfer_count,
                total_eth,
                total_token,
                gas_cost,
            } => {
                log::info!(
                    "Funding plan: {} transfers, {} wei ETH total ({} wei gas), {} tokens",
                    transfer_count,
                    total_eth,
                    gas_cost,
                    total_token
                );
            }
            FundingEvent::ExecutingTransfers { total } => {
                log::info!("Executing {} transfers", total);
            }
            FundingEvent::TransferStarted {
                index,
                total,
                transfer,
            } => {
                log::info!("[{}/{}] {}", index + 1, total, transfer.description());
            }
            FundingEvent::TransferSubmitted { index, tx_hash } => {
                log::debug!("Transfer {} submitted: {}", index + 1, tx_hash);
            }
            FundingEvent::TransferConfirmed {
                index,
                tx_hash,
                gas_used,
            } => {
                log::info!(
                    "Transfer {} confirmed: {} (gas: {})",
                    index + 1,
                    tx_hash,
                    gas_used
                );
            }
            FundingEvent::Complete {
                successful,
                total_gas_used,
            } => {
                log::info!(
                    "Funding complete: {} transfers successful, {} gas used",
                    successful,
                    total_gas_used
                );
            }
        }
    }
}
