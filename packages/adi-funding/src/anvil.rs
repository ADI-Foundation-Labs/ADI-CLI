//! Anvil (local development) wallet funding.
//!
//! Provides simplified funding for local Anvil networks using the
//! well-known default private key. Checks current balances and only
//! funds wallets that need more ETH.

use crate::balance::get_eth_balance;
use crate::config::{DefaultAmounts, WalletRole};
use crate::error::{FundingError, Result};
use crate::events::{FundingEvent, FundingEventHandler, NoOpEventHandler};
use crate::provider::FundingProvider;
use crate::signer::create_signer;
use adi_types::Wallets;
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::eth::TransactionRequest;
use secrecy::SecretString;
use std::sync::Arc;

/// Anvil default private key (account 0).
/// Address: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
pub const ANVIL_DEFAULT_KEY: &str =
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

/// Result of Anvil funding execution.
#[derive(Clone, Debug)]
pub struct AnvilFundingResult {
    /// Number of successful transfers.
    pub successful: usize,
    /// Number of wallets skipped (already funded).
    pub skipped: usize,
    /// Transaction hashes.
    pub tx_hashes: Vec<B256>,
    /// Total gas used.
    pub total_gas_used: u64,
}

/// A wallet address with its role and balance status for Anvil funding.
#[derive(Clone, Debug)]
pub struct AnvilFundingTarget {
    /// Wallet role (deployer, governor, operator, etc.).
    pub role: WalletRole,
    /// Wallet address.
    pub address: Address,
    /// Required funding amount.
    pub amount: U256,
    /// Current balance (for display purposes).
    pub current_balance: U256,
    /// Whether this wallet needs funding.
    pub needs_funding: bool,
}

/// Anvil-specific wallet funder.
///
/// Uses the well-known Anvil default account to fund wallets quickly
/// without complex balance checking.
pub struct AnvilFunder {
    provider: FundingProvider,
    funder_key: SecretString,
    amounts: DefaultAmounts,
    event_handler: Arc<dyn FundingEventHandler>,
}

impl AnvilFunder {
    /// Create a new Anvil funder with default settings.
    ///
    /// Uses localhost:8545 and the Anvil default private key.
    pub fn new() -> Result<Self> {
        Self::with_rpc("http://localhost:8545")
    }

    /// Create an Anvil funder with a custom RPC URL.
    pub fn with_rpc(rpc_url: &str) -> Result<Self> {
        let provider = FundingProvider::new(rpc_url)?;
        Ok(Self {
            provider,
            funder_key: SecretString::from(ANVIL_DEFAULT_KEY.to_string()),
            amounts: DefaultAmounts::default(),
            event_handler: Arc::new(NoOpEventHandler),
        })
    }

    /// Set custom funding amounts.
    pub fn with_amounts(mut self, amounts: DefaultAmounts) -> Self {
        self.amounts = amounts;
        self
    }

    /// Set event handler for progress reporting.
    pub fn with_event_handler(mut self, handler: Arc<dyn FundingEventHandler>) -> Self {
        self.event_handler = handler;
        self
    }

    /// Fund all wallets from ecosystem and chain wallet collections.
    ///
    /// Checks current balances and only funds wallets that need more ETH.
    /// Returns information about both funded and skipped wallets.
    pub async fn fund_wallets(
        &self,
        ecosystem_wallets: &Wallets,
        chain_wallets: &Wallets,
    ) -> Result<AnvilFundingResult> {
        let all_targets = self
            .build_targets_with_balances(ecosystem_wallets, chain_wallets)
            .await?;

        // Separate targets that need funding from those already funded
        let targets_to_fund: Vec<_> = all_targets
            .iter()
            .filter(|t| t.needs_funding)
            .cloned()
            .collect();
        let skipped = all_targets.len() - targets_to_fund.len();

        if targets_to_fund.is_empty() {
            return Ok(AnvilFundingResult {
                successful: 0,
                skipped,
                tx_hashes: Vec::new(),
                total_gas_used: 0,
            });
        }

        let mut result = self.execute_transfers(&targets_to_fund).await?;
        result.skipped = skipped;
        Ok(result)
    }

    /// Get all funding targets with their current balances.
    ///
    /// Returns targets for display purposes, including those already funded.
    pub async fn get_funding_targets(
        &self,
        ecosystem_wallets: &Wallets,
        chain_wallets: &Wallets,
    ) -> Result<Vec<AnvilFundingTarget>> {
        self.build_targets_with_balances(ecosystem_wallets, chain_wallets)
            .await
    }

    /// Build funding targets from wallet collections, checking current balances.
    async fn build_targets_with_balances(
        &self,
        ecosystem: &Wallets,
        chain: &Wallets,
    ) -> Result<Vec<AnvilFundingTarget>> {
        let mut targets = Vec::new();

        // Helper to create target with balance check
        let check_and_add = |targets: &mut Vec<AnvilFundingTarget>,
                             role: WalletRole,
                             address: Address,
                             amount: U256,
                             balance: U256| {
            targets.push(AnvilFundingTarget {
                role,
                address,
                amount,
                current_balance: balance,
                needs_funding: balance < amount,
            });
        };

        // Ecosystem wallets: deployer, governor
        if let Some(w) = &ecosystem.deployer {
            let balance = get_eth_balance(&self.provider, w.address).await?;
            check_and_add(
                &mut targets,
                WalletRole::Deployer,
                w.address,
                self.amounts.deployer_eth,
                balance,
            );
        }
        if let Some(w) = &ecosystem.governor {
            let balance = get_eth_balance(&self.provider, w.address).await?;
            check_and_add(
                &mut targets,
                WalletRole::Governor,
                w.address,
                self.amounts.governor_eth,
                balance,
            );
        }

        // Chain wallets: governor, operator, prove_operator, execute_operator
        // Note: blob_operator, fee_account, token_multiplier_setter are NOT funded
        // during ecosystem deployment (same as production funding logic)
        if let Some(w) = &chain.governor {
            let balance = get_eth_balance(&self.provider, w.address).await?;
            check_and_add(
                &mut targets,
                WalletRole::Governor,
                w.address,
                self.amounts.governor_eth,
                balance,
            );
        }
        if let Some(w) = &chain.operator {
            let balance = get_eth_balance(&self.provider, w.address).await?;
            check_and_add(
                &mut targets,
                WalletRole::Operator,
                w.address,
                self.amounts.operator_eth,
                balance,
            );
        }
        if let Some(w) = &chain.prove_operator {
            let balance = get_eth_balance(&self.provider, w.address).await?;
            check_and_add(
                &mut targets,
                WalletRole::ProveOperator,
                w.address,
                self.amounts.prove_operator_eth,
                balance,
            );
        }
        if let Some(w) = &chain.execute_operator {
            let balance = get_eth_balance(&self.provider, w.address).await?;
            check_and_add(
                &mut targets,
                WalletRole::ExecuteOperator,
                w.address,
                self.amounts.execute_operator_eth,
                balance,
            );
        }

        Ok(targets)
    }

    /// Execute transfers to all targets.
    async fn execute_transfers(
        &self,
        targets: &[AnvilFundingTarget],
    ) -> Result<AnvilFundingResult> {
        let signer = create_signer(&self.funder_key)?;
        let wallet = EthereumWallet::from(signer.clone());

        let url = self
            .provider
            .url()
            .parse()
            .map_err(|e| FundingError::ProviderConnection {
                url: self.provider.url().to_string(),
                reason: format!("{e}"),
            })?;

        let signing_provider = ProviderBuilder::new().wallet(wallet).connect_http(url);
        let chain_id = self.provider.get_chain_id().await?;
        let gas_price = self.provider.get_gas_price().await?;
        let mut nonce = self.provider.get_nonce(signer.address()).await?;

        let total = targets.len();
        self.event_handler
            .on_event(FundingEvent::ExecutingTransfers { total })
            .await;

        let mut tx_hashes = Vec::with_capacity(total);
        let mut total_gas_used = 0u64;

        for (index, target) in targets.iter().enumerate() {
            // Build transaction for gas estimation (without gas limit)
            let estimate_tx = TransactionRequest::default()
                .with_from(signer.address())
                .with_to(target.address)
                .with_value(target.amount);

            // Estimate gas - ZkSync Era requires more than 21k for account abstraction
            let gas_estimate = self.provider.estimate_gas(&estimate_tx).await?;

            // Build final transaction with estimated gas
            let tx = TransactionRequest::default()
                .with_from(signer.address())
                .with_to(target.address)
                .with_value(target.amount)
                .with_nonce(nonce)
                .with_gas_limit(gas_estimate)
                .with_gas_price(gas_price)
                .with_chain_id(chain_id);

            let pending = signing_provider.send_transaction(tx).await.map_err(|e| {
                FundingError::TransactionFailed {
                    to: target.address,
                    reason: e.to_string(),
                }
            })?;

            let tx_hash = *pending.tx_hash();

            let receipt =
                pending
                    .get_receipt()
                    .await
                    .map_err(|e| FundingError::TransactionFailed {
                        to: target.address,
                        reason: e.to_string(),
                    })?;

            if !receipt.status() {
                return Err(FundingError::TransactionReverted(format!(
                    "Transfer to {} ({}) reverted",
                    target.role, target.address
                )));
            }

            tx_hashes.push(tx_hash);
            total_gas_used += receipt.gas_used;
            nonce += 1;

            self.event_handler
                .on_event(FundingEvent::TransferConfirmed {
                    index,
                    tx_hash,
                    gas_used: receipt.gas_used,
                })
                .await;
        }

        self.event_handler
            .on_event(FundingEvent::Complete {
                successful: tx_hashes.len(),
                total_gas_used,
            })
            .await;

        Ok(AnvilFundingResult {
            successful: tx_hashes.len(),
            skipped: 0, // Will be updated by caller
            tx_hashes,
            total_gas_used,
        })
    }
}

/// Check if an RPC URL appears to be a local Anvil instance.
pub fn is_localhost_rpc(rpc_url: &str) -> bool {
    let lower = rpc_url.to_lowercase();
    lower.contains("localhost") || lower.contains("127.0.0.1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_localhost_rpc_localhost() {
        assert!(is_localhost_rpc("http://localhost:8545"));
        assert!(is_localhost_rpc("http://LOCALHOST:8545"));
        assert!(is_localhost_rpc("https://localhost:8545"));
    }

    #[test]
    fn test_is_localhost_rpc_127() {
        assert!(is_localhost_rpc("http://127.0.0.1:8545"));
        assert!(is_localhost_rpc("https://127.0.0.1:8545"));
    }

    #[test]
    fn test_is_localhost_rpc_remote() {
        assert!(!is_localhost_rpc("https://sepolia.infura.io/v3/key"));
        assert!(!is_localhost_rpc("https://mainnet.infura.io/v3/key"));
        assert!(!is_localhost_rpc("https://rpc.zksync.io"));
    }
}
