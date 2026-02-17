//! Event system for funding progress reporting.

use crate::config::WalletRole;
use crate::transfer::Transfer;
use adi_types::Logger;
use alloy_primitives::{Address, B256, U256};
use cliclack::ProgressBar;
use console::style;
use std::sync::{Arc, Mutex};

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

/// Event handler that logs events using the Logger trait.
pub struct LoggingEventHandler {
    logger: Arc<dyn Logger>,
}

impl LoggingEventHandler {
    /// Create a new LoggingEventHandler with the given logger.
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self { logger }
    }
}

#[async_trait::async_trait]
impl FundingEventHandler for LoggingEventHandler {
    async fn on_event(&self, event: FundingEvent) {
        match event {
            FundingEvent::CheckingBalances { wallet_count } => {
                self.logger
                    .info(&format!("Checking balances for {} wallets", wallet_count));
            }
            FundingEvent::BalanceChecked {
                role,
                address,
                eth_balance,
                token_balance,
            } => {
                if let Some(tok) = token_balance {
                    self.logger.debug(&format!(
                        "{}: {} has {} wei ETH, {} token",
                        role, address, eth_balance, tok
                    ));
                } else {
                    self.logger.debug(&format!(
                        "{}: {} has {} wei ETH",
                        role, address, eth_balance
                    ));
                }
            }
            FundingEvent::PlanCreated {
                transfer_count,
                total_eth,
                total_token,
                gas_cost,
            } => {
                self.logger.info(&format!(
                    "Funding plan: {} transfers, {} wei ETH total ({} wei gas), {} tokens",
                    transfer_count, total_eth, gas_cost, total_token
                ));
            }
            FundingEvent::ExecutingTransfers { total } => {
                self.logger
                    .info(&format!("Executing {} transfers", style(total).green()));
            }
            FundingEvent::TransferStarted {
                index,
                total,
                transfer,
            } => {
                self.logger.info(&format!(
                    "[{}/{}] {}",
                    index + 1,
                    total,
                    transfer.description()
                ));
            }
            FundingEvent::TransferSubmitted { index, tx_hash } => {
                self.logger
                    .debug(&format!("Transfer {} submitted: {}", index + 1, tx_hash));
            }
            FundingEvent::TransferConfirmed {
                index,
                tx_hash,
                gas_used,
            } => {
                self.logger.info(&format!(
                    "Transfer {} confirmed: {} (gas: {})",
                    index + 1,
                    style(tx_hash).green(),
                    style(gas_used).green()
                ));
            }
            FundingEvent::Complete {
                successful,
                total_gas_used,
            } => {
                self.logger.info(&format!(
                    "Funding complete: {} transfers successful, {} gas used",
                    style(successful).green(),
                    style(total_gas_used).green()
                ));
            }
        }
    }
}

/// Event handler that shows transfer progress with cliclack spinners.
///
/// Each transfer gets its own spinner that shows progress and completion status.
pub struct SpinnerEventHandler {
    spinner: Mutex<Option<ProgressBar>>,
}

impl SpinnerEventHandler {
    /// Create a new SpinnerEventHandler.
    pub fn new() -> Self {
        Self {
            spinner: Mutex::new(None),
        }
    }
}

impl Default for SpinnerEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl FundingEventHandler for SpinnerEventHandler {
    async fn on_event(&self, event: FundingEvent) {
        match event {
            FundingEvent::TransferStarted {
                index,
                total,
                transfer,
            } => {
                let spinner = cliclack::spinner();
                spinner.start(format!(
                    "{} {} to {} ({})",
                    style(format!("[{}/{}]", index + 1, total)).magenta(),
                    style(transfer.amount_description()).green(),
                    style(transfer.to).green(),
                    style(transfer.role).cyan()
                ));
                if let Ok(mut guard) = self.spinner.lock() {
                    *guard = Some(spinner);
                }
            }
            FundingEvent::TransferConfirmed {
                index,
                tx_hash,
                gas_used,
            } => {
                if let Ok(mut guard) = self.spinner.lock() {
                    if let Some(spinner) = guard.take() {
                        spinner.stop(format!(
                            "{} Confirmed: {} (gas: {})",
                            style(format!("[{}]", index + 1)).magenta(),
                            style(tx_hash).green(),
                            style(gas_used).green()
                        ));
                    }
                }
            }
            // Ignore other events - they're handled by the UI layer
            _ => {}
        }
    }
}
