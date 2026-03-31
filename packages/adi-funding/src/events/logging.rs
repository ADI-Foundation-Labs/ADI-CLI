//! Logger-based event handler for funding operations.

use super::{FundingEvent, FundingEventHandler};
use crate::config::WalletRole;
use adi_types::Logger;
use alloy_primitives::{Address, B256, U256};
use console::style;
use std::sync::Arc;

/// Event handler that logs events using the Logger trait.
pub struct LoggingEventHandler {
    logger: Arc<dyn Logger>,
}

impl LoggingEventHandler {
    /// Create a new LoggingEventHandler with the given logger.
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self { logger }
    }

    fn log_balance_checked(
        &self,
        role: WalletRole,
        address: Address,
        eth_balance: U256,
        token_balance: Option<U256>,
    ) {
        match token_balance {
            Some(tok) => self.logger.debug(&format!(
                "{role}: {address} has {eth_balance} wei ETH, {tok} token"
            )),
            None => self
                .logger
                .debug(&format!("{role}: {address} has {eth_balance} wei ETH")),
        }
    }

    fn log_plan_created(
        &self,
        transfer_count: usize,
        total_eth: U256,
        total_token: U256,
        gas_cost: U256,
    ) {
        self.logger.info(&format!(
            "Funding plan: {transfer_count} transfers, {total_eth} wei ETH total ({gas_cost} wei gas), {total_token} tokens"
        ));
    }

    fn log_transfer_confirmed(&self, index: usize, tx_hash: B256, gas_used: u64) {
        self.logger.info(&format!(
            "Transfer {} confirmed: {} (gas: {})",
            index + 1,
            style(tx_hash).green(),
            style(gas_used).green()
        ));
    }

    fn log_complete(&self, successful: usize, total_gas_used: u64) {
        self.logger.info(&format!(
            "Funding complete: {} transfers successful, {} gas used",
            style(successful).green(),
            style(total_gas_used).green()
        ));
    }
}

#[async_trait::async_trait]
impl FundingEventHandler for LoggingEventHandler {
    async fn on_event(&self, event: FundingEvent) {
        match event {
            FundingEvent::CheckingBalances { wallet_count } => {
                self.logger
                    .info(&format!("Checking balances for {wallet_count} wallets"));
            }
            FundingEvent::BalanceChecked {
                role,
                address,
                eth_balance,
                token_balance,
            } => self.log_balance_checked(role, address, eth_balance, token_balance),
            FundingEvent::PlanCreated {
                transfer_count,
                total_eth,
                total_token,
                gas_cost,
            } => self.log_plan_created(transfer_count, total_eth, total_token, gas_cost),
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
                    .debug(&format!("Transfer {} submitted: {tx_hash}", index + 1));
            }
            FundingEvent::TransferConfirmed {
                index,
                tx_hash,
                gas_used,
            } => self.log_transfer_confirmed(index, tx_hash, gas_used),
            FundingEvent::Complete {
                successful,
                total_gas_used,
            } => self.log_complete(successful, total_gas_used),
        }
    }
}
