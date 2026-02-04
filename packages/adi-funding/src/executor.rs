//! Funding executor for executing funding plans.

use crate::error::{FundingError, Result};
use crate::events::{FundingEvent, FundingEventHandler, NoOpEventHandler};
use crate::plan::FundingPlan;
use crate::provider::FundingProvider;
use crate::signer::create_signer;
use crate::transfer::{build_token_transfer_calldata, Transfer, TransferType};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, B256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::eth::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use secrecy::SecretString;
use std::sync::Arc;

/// Result of executing a funding plan.
#[derive(Clone, Debug)]
pub struct FundingResult {
    /// Number of successful transfers.
    pub successful: usize,
    /// Total gas used.
    pub total_gas_used: u64,
    /// Transaction hashes for successful transfers.
    pub tx_hashes: Vec<B256>,
}

impl FundingResult {
    /// Check if all transfers were successful.
    pub fn is_success(&self) -> bool {
        self.successful > 0
    }
}

/// Executor for funding operations.
pub struct FundingExecutor {
    provider: FundingProvider,
    signer: PrivateKeySigner,
    event_handler: Arc<dyn FundingEventHandler>,
}

impl FundingExecutor {
    /// Create a new funding executor.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - RPC endpoint URL.
    /// * `funder_key` - Funder wallet private key.
    ///
    /// # Errors
    ///
    /// Returns error if provider connection or signer creation fails.
    pub fn new(rpc_url: &str, funder_key: &SecretString) -> Result<Self> {
        let provider = FundingProvider::new(rpc_url)?;
        let signer = create_signer(funder_key)?;

        Ok(Self {
            provider,
            signer,
            event_handler: Arc::new(NoOpEventHandler),
        })
    }

    /// Set the event handler for progress reporting.
    pub fn with_event_handler(mut self, handler: Arc<dyn FundingEventHandler>) -> Self {
        self.event_handler = handler;
        self
    }

    /// Get the funder address.
    pub fn funder_address(&self) -> Address {
        self.signer.address()
    }

    /// Get a reference to the provider.
    pub fn provider(&self) -> &FundingProvider {
        &self.provider
    }

    /// Execute a funding plan.
    ///
    /// Executes transfers sequentially. Aborts on first failure.
    ///
    /// # Arguments
    ///
    /// * `plan` - The funding plan to execute.
    ///
    /// # Returns
    ///
    /// Result with execution statistics.
    ///
    /// # Errors
    ///
    /// Returns error if any transfer fails (abort-on-failure behavior).
    pub async fn execute(&self, plan: &FundingPlan) -> Result<FundingResult> {
        let total = plan.transfers.len();

        self.event_handler
            .on_event(FundingEvent::ExecutingTransfers { total })
            .await;

        let chain_id = self.provider.get_chain_id().await?;
        let mut nonce = self.provider.get_nonce(self.funder_address()).await?;

        let mut successful = 0;
        let mut total_gas_used = 0u64;
        let mut tx_hashes = Vec::with_capacity(total);

        // Create wallet for signing
        let wallet = EthereumWallet::from(self.signer.clone());

        // Create provider with signer
        let url = self
            .provider
            .url()
            .parse()
            .map_err(|e| FundingError::ProviderConnection {
                url: self.provider.url().to_string(),
                reason: format!("{e}"),
            })?;
        let signing_provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

        for (index, transfer) in plan.transfers.iter().enumerate() {
            self.event_handler
                .on_event(FundingEvent::TransferStarted {
                    index,
                    total,
                    transfer: transfer.clone(),
                })
                .await;

            // Execute the transfer (abort on failure)
            let (tx_hash, gas_used) = self
                .execute_transfer(
                    &signing_provider,
                    transfer,
                    chain_id,
                    nonce,
                    plan.gas_price,
                    index,
                )
                .await?;

            self.event_handler
                .on_event(FundingEvent::TransferConfirmed {
                    index,
                    tx_hash,
                    gas_used,
                })
                .await;

            successful += 1;
            total_gas_used += gas_used;
            tx_hashes.push(tx_hash);
            nonce += 1;
        }

        self.event_handler
            .on_event(FundingEvent::Complete {
                successful,
                total_gas_used,
            })
            .await;

        Ok(FundingResult {
            successful,
            total_gas_used,
            tx_hashes,
        })
    }

    /// Execute a single transfer.
    async fn execute_transfer<P: Provider>(
        &self,
        provider: &P,
        transfer: &Transfer,
        chain_id: u64,
        nonce: u64,
        gas_price: u128,
        index: usize,
    ) -> Result<(B256, u64)> {
        let tx = match &transfer.transfer_type {
            TransferType::Eth { amount } => TransactionRequest::default()
                .with_from(transfer.from)
                .with_to(transfer.to)
                .with_value(*amount)
                .with_nonce(nonce)
                .with_gas_limit(transfer.gas_estimate)
                .with_gas_price(gas_price)
                .with_chain_id(chain_id),
            TransferType::Token {
                token_address,
                amount,
                ..
            } => {
                let calldata = build_token_transfer_calldata(transfer.to, *amount);

                TransactionRequest::default()
                    .with_from(transfer.from)
                    .with_to(*token_address)
                    .with_input(calldata)
                    .with_nonce(nonce)
                    .with_gas_limit(transfer.gas_estimate)
                    .with_gas_price(gas_price)
                    .with_chain_id(chain_id)
            }
        };

        // Send transaction
        let pending =
            provider
                .send_transaction(tx)
                .await
                .map_err(|e| FundingError::TransactionFailed {
                    to: transfer.to,
                    reason: e.to_string(),
                })?;

        let tx_hash = *pending.tx_hash();

        self.event_handler
            .on_event(FundingEvent::TransferSubmitted { index, tx_hash })
            .await;

        // Wait for confirmation
        let receipt = pending
            .get_receipt()
            .await
            .map_err(|e| FundingError::TransactionFailed {
                to: transfer.to,
                reason: e.to_string(),
            })?;

        if !receipt.status() {
            return Err(FundingError::TransactionReverted(format!(
                "Transaction {} reverted",
                tx_hash
            )));
        }

        Ok((tx_hash, receipt.gas_used))
    }
}
