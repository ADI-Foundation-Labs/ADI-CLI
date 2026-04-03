//! Refund plan building — checks balances and computes sendable amounts.

use std::collections::HashSet;
use std::sync::Arc;

use alloy_primitives::{Address, U256};

use super::types::{RefundConfig, RefundEntry, RefundPlan, RefundTarget};
use crate::balance::{get_eth_balance, get_token_balance};
use crate::error::Result;
use crate::events::{FundingEvent, FundingEventHandler};
use crate::provider::FundingProvider;
use crate::transfer::{estimate_eth_transfer_gas, estimate_token_transfer_gas};

/// Estimated gas costs for a single wallet (ETH + optional token transfer).
struct GasEstimates {
    eth_gas: u64,
    token_gas: u64,
    eth_cost: U256,
    token_cost: U256,
}

/// Estimate gas costs using the first available wallet and the receiver.
///
/// Gas is estimated once with a dummy amount (1 wei / 1 token unit) since
/// transfer gas doesn't depend on the amount. Falls back to conservative
/// defaults if estimation fails (e.g., wallet has zero balance).
async fn estimate_gas_costs(
    provider: &FundingProvider,
    first_entry: &RefundEntry,
    receiver: Address,
    token_address: Option<Address>,
    gas_price: u128,
    logger: &dyn adi_types::Logger,
) -> GasEstimates {
    let eth_gas = estimate_eth_transfer_gas(provider, first_entry.address, receiver, U256::from(1))
        .await
        .unwrap_or_else(|e| {
            logger.debug(&format!("ETH gas estimation failed, using fallback: {e}"));
            21_000
        });

    let token_gas = match token_address {
        Some(token_addr) => estimate_token_transfer_gas(
            provider,
            first_entry.address,
            receiver,
            token_addr,
            U256::from(1),
        )
        .await
        .unwrap_or_else(|e| {
            logger.debug(&format!("Token gas estimation failed, using fallback: {e}"));
            65_000
        }),
        None => 0,
    };

    let eth_cost = U256::from(eth_gas) * U256::from(gas_price);
    let token_cost = U256::from(token_gas) * U256::from(gas_price);

    logger.debug(&format!(
        "Estimated gas: ETH={eth_gas} ({eth_cost} wei), token={token_gas} ({token_cost} wei)"
    ));

    GasEstimates {
        eth_gas,
        token_gas,
        eth_cost,
        token_cost,
    }
}

/// Build a refund plan by checking balances and computing sendable amounts.
///
/// Deduplicates wallets by address (first occurrence wins).
/// Gas is estimated via RPC rather than hardcoded.
///
/// # Errors
///
/// Returns error if RPC calls fail.
pub async fn build_refund_plan(
    provider: &FundingProvider,
    entries: &[RefundEntry],
    config: &RefundConfig,
    logger: &dyn adi_types::Logger,
    event_handler: &Arc<dyn FundingEventHandler>,
) -> Result<RefundPlan> {
    let raw_gas_price = provider.get_gas_price().await?;
    let gas_price = raw_gas_price * u128::from(config.gas_multiplier) / 100;
    logger.debug(&format!(
        "Gas price: {raw_gas_price} (raw) -> {gas_price} (adjusted {}%)",
        config.gas_multiplier
    ));

    // Fetch token metadata once if needed
    let (token_symbol, token_decimals) = match config.token_address {
        Some(addr) => {
            let symbol = crate::balance::get_token_symbol(provider, addr).await?;
            let decimals = crate::balance::get_token_decimals(provider, addr).await?;
            (Some(symbol), Some(decimals))
        }
        None => (None, None),
    };

    // Estimate gas using the first entry as a representative wallet
    let Some(first_entry) = entries.first() else {
        return Ok(empty_plan(config, gas_price, token_symbol, token_decimals));
    };

    let gas = estimate_gas_costs(
        provider,
        first_entry,
        config.receiver,
        config.token_address,
        gas_price,
        logger,
    )
    .await;

    let mut seen_addresses = HashSet::new();
    let mut targets = Vec::new();
    let mut total_eth = U256::ZERO;
    let mut total_token = U256::ZERO;

    let unique_count = entries
        .iter()
        .map(|e| e.address)
        .collect::<HashSet<_>>()
        .len();

    event_handler
        .on_event(FundingEvent::CheckingBalances {
            wallet_count: unique_count,
        })
        .await;

    for entry in entries {
        if !seen_addresses.insert(entry.address) {
            logger.debug(&format!(
                "Skipping duplicate address {} ({})",
                entry.address, entry.role
            ));
            continue;
        }

        let eth_balance = get_eth_balance(provider, entry.address).await?;
        let token_bal = match config.token_address {
            Some(addr) => Some(get_token_balance(provider, addr, entry.address).await?),
            None => None,
        };

        event_handler
            .on_event(FundingEvent::BalanceChecked {
                role: entry.role,
                address: entry.address,
                eth_balance,
                token_balance: token_bal,
            })
            .await;

        logger.debug(&format!(
            "{} {} ({}) — ETH: {eth_balance} wei{}",
            entry.source.prefix(),
            entry.role,
            entry.address,
            token_bal
                .map(|b| format!(", token: {b}"))
                .unwrap_or_default()
        ));

        let total_gas_reserve = gas.eth_cost + gas.token_cost;
        let sendable_eth = eth_balance.saturating_sub(total_gas_reserve);
        let sendable_token = token_bal.unwrap_or(U256::ZERO);

        if sendable_eth.is_zero() && sendable_token.is_zero() {
            continue;
        }

        total_eth += sendable_eth;
        total_token += sendable_token;

        targets.push(RefundTarget {
            role: entry.role,
            source: entry.source,
            chain_name: entry.chain_name.clone(),
            address: entry.address,
            private_key: entry.private_key.clone(),
            eth_balance,
            token_balance: token_bal,
            sendable_eth,
            sendable_token,
            eth_gas_estimate: gas.eth_gas,
            token_gas_estimate: gas.token_gas,
        });
    }

    event_handler
        .on_event(FundingEvent::PlanCreated {
            transfer_count: targets.len(),
            total_eth,
            total_token,
            gas_cost: gas.eth_cost + gas.token_cost,
        })
        .await;

    Ok(RefundPlan {
        receiver: config.receiver,
        gas_price,
        targets,
        token_address: config.token_address,
        token_symbol,
        token_decimals,
        total_eth_to_refund: total_eth,
        total_token_to_refund: total_token,
    })
}

/// Create an empty plan when there are no entries.
fn empty_plan(
    config: &RefundConfig,
    gas_price: u128,
    token_symbol: Option<String>,
    token_decimals: Option<u8>,
) -> RefundPlan {
    RefundPlan {
        receiver: config.receiver,
        gas_price,
        targets: Vec::new(),
        token_address: config.token_address,
        token_symbol,
        token_decimals,
        total_eth_to_refund: U256::ZERO,
        total_token_to_refund: U256::ZERO,
    }
}
