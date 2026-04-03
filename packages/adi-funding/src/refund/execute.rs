//! Refund execution — drains each wallet to the receiver.

use std::sync::Arc;

use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::eth::TransactionRequest;

use super::types::{RefundPlan, RefundResult};
use crate::error::{FundingError, Result};
use crate::events::{FundingEvent, FundingEventHandler};
use crate::provider::FundingProvider;
use crate::signer::create_signer;
use crate::transfer::{build_token_transfer_calldata, Transfer, TransferType};

/// Default ERC20 token decimals when not specified.
const DEFAULT_TOKEN_DECIMALS: u8 = 18;

/// Shared context for the execution loop.
struct ExecCtx {
    chain_id: u64,
    gas_price: u128,
    total_transfers: usize,
    event_handler: Arc<dyn FundingEventHandler>,
}

/// Execute a refund plan, draining each wallet to the receiver.
///
/// Uses continue-on-error: if one wallet fails, the rest still proceed.
///
/// # Errors
///
/// Returns error only for fatal issues (e.g., cannot connect to RPC).
/// Individual wallet failures are captured in [`RefundResult::errors`].
pub async fn execute_refund(
    rpc_url: &str,
    plan: &RefundPlan,
    event_handler: &Arc<dyn FundingEventHandler>,
    logger: &dyn adi_types::Logger,
) -> Result<RefundResult> {
    let provider = FundingProvider::new(rpc_url)?;
    let chain_id = provider.get_chain_id().await?;
    let parsed_url: alloy_transport_http::reqwest::Url =
        rpc_url
            .parse()
            .map_err(|e| FundingError::ProviderConnection {
                url: rpc_url.to_string(),
                reason: format!("Invalid URL: {e}"),
            })?;

    let ctx = ExecCtx {
        chain_id,
        gas_price: plan.gas_price,
        total_transfers: count_transfers(plan),
        event_handler: Arc::clone(event_handler),
    };

    ctx.event_handler
        .on_event(FundingEvent::ExecutingTransfers {
            total: ctx.total_transfers,
        })
        .await;

    let mut result = RefundResult {
        successful: 0,
        failed: 0,
        total_eth_refunded: U256::ZERO,
        total_token_refunded: U256::ZERO,
        total_gas_used: 0,
        tx_hashes: Vec::new(),
        errors: Vec::new(),
    };

    let mut idx = 0;

    for target in &plan.targets {
        logger.debug(&format!(
            "Refunding from {} ({})",
            target.label(),
            target.address
        ));

        let signer = match create_signer(&target.private_key) {
            Ok(s) => s,
            Err(e) => {
                record_error(&mut result, &target.label(), "signer creation", &e, logger);
                continue;
            }
        };

        let wallet = EthereumWallet::from(signer);
        let signing_provider = ProviderBuilder::new()
            .wallet(wallet)
            .connect_http(parsed_url.clone());

        let mut nonce = provider.get_nonce(target.address).await?;
        let label = target.label();

        // Token transfer first (needs ETH for gas)
        if let (Some(token_addr), false) = (plan.token_address, target.sendable_token.is_zero()) {
            let transfer = Transfer::token(
                target.role,
                target.address,
                plan.receiver,
                TransferType::Token {
                    token_address: token_addr,
                    amount: target.sendable_token,
                    symbol: plan.token_symbol.clone().unwrap_or_default(),
                    decimals: plan.token_decimals.unwrap_or(DEFAULT_TOKEN_DECIMALS),
                },
                target.token_gas_estimate,
            );

            let success = ctx
                .send_and_record(
                    &signing_provider,
                    &transfer,
                    nonce,
                    idx,
                    &mut result,
                    &format!("{label}: token transfer"),
                )
                .await;
            idx += 1;
            if success {
                nonce += 1;
            }
        }

        // ETH transfer (drain remaining balance minus gas)
        if !target.sendable_eth.is_zero() {
            let transfer = Transfer::eth(
                target.role,
                target.address,
                plan.receiver,
                target.sendable_eth,
                target.eth_gas_estimate,
            );

            ctx.send_and_record(
                &signing_provider,
                &transfer,
                nonce,
                idx,
                &mut result,
                &format!("{label}: ETH transfer"),
            )
            .await;
            idx += 1;
        }
    }

    ctx.event_handler
        .on_event(FundingEvent::Complete {
            successful: result.successful,
            total_gas_used: result.total_gas_used,
        })
        .await;

    Ok(result)
}

impl ExecCtx {
    /// Send a transfer, record the outcome in result, and emit events.
    ///
    /// Returns `true` if the transfer succeeded.
    async fn send_and_record<P: Provider>(
        &self,
        provider: &P,
        transfer: &Transfer,
        nonce: u64,
        index: usize,
        result: &mut RefundResult,
        description: &str,
    ) -> bool {
        self.event_handler
            .on_event(FundingEvent::TransferStarted {
                index,
                total: self.total_transfers,
                transfer: transfer.clone(),
            })
            .await;

        match send_transfer(provider, transfer, self.chain_id, nonce, self.gas_price).await {
            Ok((tx_hash, gas_used)) => {
                self.event_handler
                    .on_event(FundingEvent::TransferConfirmed {
                        index,
                        tx_hash,
                        gas_used,
                    })
                    .await;
                result.successful += 1;
                result.total_gas_used += gas_used;
                result.tx_hashes.push(tx_hash);

                match &transfer.transfer_type {
                    TransferType::Eth { amount } => result.total_eth_refunded += *amount,
                    TransferType::Token { amount, .. } => result.total_token_refunded += *amount,
                }
                true
            }
            Err(e) => {
                let msg = format!("{description} failed: {e}");
                result.errors.push(msg);
                result.failed += 1;
                false
            }
        }
    }
}

/// Record an error into the result and log it.
fn record_error(
    result: &mut RefundResult,
    label: &str,
    kind: &str,
    error: &dyn std::fmt::Display,
    logger: &dyn adi_types::Logger,
) {
    let msg = format!("{label}: {kind} failed: {error}");
    logger.debug(&msg);
    result.errors.push(msg);
    result.failed += 1;
}

/// Count total individual transfers in the plan.
fn count_transfers(plan: &RefundPlan) -> usize {
    plan.targets
        .iter()
        .map(|t| {
            let has_token = !t.sendable_token.is_zero() && plan.token_address.is_some();
            let has_eth = !t.sendable_eth.is_zero();
            usize::from(has_token) + usize::from(has_eth)
        })
        .sum()
}

/// Send a single transfer transaction and wait for receipt.
async fn send_transfer<P: Provider>(
    provider: &P,
    transfer: &Transfer,
    chain_id: u64,
    nonce: u64,
    gas_price: u128,
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

    let pending =
        provider
            .send_transaction(tx)
            .await
            .map_err(|e| FundingError::TransactionFailed {
                to: transfer.to,
                reason: e.to_string(),
            })?;

    let tx_hash = *pending.tx_hash();

    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| FundingError::TransactionFailed {
            to: transfer.to,
            reason: e.to_string(),
        })?;

    if !receipt.status() {
        return Err(FundingError::TransactionReverted(format!(
            "Transaction {tx_hash} reverted"
        )));
    }

    Ok((tx_hash, receipt.gas_used))
}
