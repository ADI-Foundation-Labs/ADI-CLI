//! Transaction filterer wiring for Nox (Prividium-style) private chains.
//!
//! Registers a deployed `NoxTransactionFilterer` on the chain's Diamond via
//! `setTransactionFilterer`, called through the ChainAdmin multicall — the
//! same pattern used for `setDAValidatorPair` in [`crate::da`].

use crate::error::{EcosystemError, Result};
use crate::signer::create_signer;
use adi_types::normalize_rpc_url;
use adi_types::Logger;
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};
use console::Style;
use secrecy::SecretString;
use tokio::time::{timeout, Duration};

use adi_types::TX_TIMEOUT_SECONDS;

sol! {
    /// Register the transaction filterer on the Diamond proxy.
    /// Called through ChainAdmin multicall.
    #[allow(missing_docs)]
    function setTransactionFilterer(address transactionFilterer) external;

    /// ChainAdmin multicall interface.
    #[allow(missing_docs)]
    function multicall(
        (address, uint256, bytes)[] calls,
        bool requireSuccess
    ) external;
}

/// Build calldata for `setTransactionFilterer` via ChainAdmin multicall.
#[must_use]
pub fn build_set_transaction_filterer_multicall_calldata(
    diamond_proxy: Address,
    transaction_filterer: Address,
) -> Bytes {
    let inner_call = setTransactionFiltererCall {
        transactionFilterer: transaction_filterer,
    };
    let inner_calldata = Bytes::from(inner_call.abi_encode());

    let multicall_call = multicallCall {
        calls: vec![(diamond_proxy, U256::ZERO, inner_calldata)],
        requireSuccess: true,
    };

    Bytes::from(multicall_call.abi_encode())
}

/// Arguments for configuring the transaction filterer.
pub struct TransactionFiltererConfig<'a> {
    /// Settlement layer RPC endpoint URL.
    pub rpc_url: &'a str,
    /// ChainAdmin contract address.
    pub chain_admin: Address,
    /// Diamond proxy contract address.
    pub diamond_proxy: Address,
    /// Deployed `NoxTransactionFilterer` proxy address.
    pub transaction_filterer: Address,
    /// Governor private key for signing transactions.
    pub governor_key: &'a SecretString,
    /// Gas price multiplier percentage.
    pub gas_multiplier: Option<u64>,
}

/// Register the transaction filterer on the chain's Diamond.
///
/// # Errors
///
/// Returns error if the transaction fails or reverts.
pub async fn configure_transaction_filterer(
    config: TransactionFiltererConfig<'_>,
    logger: &dyn Logger,
) -> Result<B256> {
    logger.debug(&format!(
        "Setting transaction filterer via chain_admin: {}",
        config.chain_admin
    ));

    let signer = create_signer(config.governor_key)?;
    let governor_address = signer.address();

    let wallet = EthereumWallet::from(signer);
    let normalized_rpc = normalize_rpc_url(config.rpc_url);
    let url: url::Url = normalized_rpc.parse().map_err(|e| {
        EcosystemError::InvalidConfig(format!("Invalid RPC URL '{}': {}", config.rpc_url, e))
    })?;
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

    let chain_id =
        provider
            .get_chain_id()
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to get chain ID: {}", e),
            })?;

    let nonce = provider
        .get_transaction_count(governor_address)
        .await
        .map_err(|e| EcosystemError::TransactionFailed {
            reason: format!("Failed to get nonce: {}", e),
        })?;

    let estimated =
        provider
            .get_gas_price()
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to get gas price: {}", e),
            })?;
    let gas_price = config
        .gas_multiplier
        .map_or(estimated, |m| estimated * u128::from(m) / 100);

    let calldata = build_set_transaction_filterer_multicall_calldata(
        config.diamond_proxy,
        config.transaction_filterer,
    );

    let green = Style::new().green();
    let spinner = cliclack::spinner();
    spinner.start(format!(
        "Setting transaction filterer to {}",
        green.apply_to(config.transaction_filterer)
    ));

    let tx = TransactionRequest::default()
        .with_from(governor_address)
        .with_to(config.chain_admin)
        .with_input(calldata)
        .with_nonce(nonce)
        .with_gas_limit(100_000)
        .with_gas_price(gas_price)
        .with_chain_id(chain_id);

    let pending = provider.send_transaction(tx).await.map_err(|e| {
        spinner.error(format!("Failed to send tx: {}", e));
        EcosystemError::TransactionFailed {
            reason: format!("Failed to send setTransactionFilterer tx: {}", e),
        }
    })?;

    let tx_hash = *pending.tx_hash();

    let receipt = timeout(Duration::from_secs(TX_TIMEOUT_SECONDS), pending.get_receipt())
        .await
        .map_err(|_| {
            spinner.error("Transaction not mined within timeout window");
            EcosystemError::TransactionFailed {
                reason: "Transaction not mined within timeout window: setTransactionFilterer".to_string(),
            }
        })?
        .map_err(|e| {
            spinner.error(format!("Confirmation failed: {}", e));
            EcosystemError::TransactionFailed {
                reason: format!("Failed to confirm setTransactionFilterer tx: {}", e),
            }
        })?;

    if !receipt.status() {
        spinner.error("Transaction reverted");
        return Err(EcosystemError::TransactionFailed {
            reason: format!("Transaction {} reverted", tx_hash),
        });
    }

    spinner.stop(format!(
        "Transaction filterer set -> Confirmed in block {} (gas: {})",
        green.apply_to(receipt.block_number.unwrap_or_default()),
        receipt.gas_used
    ));

    Ok(tx_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn test_build_calldata_not_empty() {
        let calldata =
            build_set_transaction_filterer_multicall_calldata(Address::ZERO, Address::ZERO);
        assert!(!calldata.is_empty());
        assert!(calldata.len() >= 4);
    }

    #[test]
    fn test_build_calldata_starts_with_multicall_selector() {
        let calldata =
            build_set_transaction_filterer_multicall_calldata(Address::ZERO, Address::ZERO);
        assert!(calldata.starts_with(&multicallCall::SELECTOR));
    }

    #[test]
    fn test_build_calldata_differs_by_filterer_address() {
        let diamond_proxy = address!("0000000000000000000000000000000000000001");

        let calldata_a = build_set_transaction_filterer_multicall_calldata(
            diamond_proxy,
            address!("00000000000000000000000000000000000002aa"),
        );
        let calldata_b = build_set_transaction_filterer_multicall_calldata(
            diamond_proxy,
            address!("00000000000000000000000000000000000002bb"),
        );

        assert_ne!(calldata_a, calldata_b);
    }

    #[test]
    fn test_build_calldata_encodes_diamond_proxy_target() {
        let diamond_proxy = address!("00000000000000000000000000000000000000aa");
        let transaction_filterer = address!("00000000000000000000000000000000000000bb");

        let calldata =
            build_set_transaction_filterer_multicall_calldata(diamond_proxy, transaction_filterer);

        // The outer multicall ABI-encodes the diamond proxy address (left-padded to
        // 32 bytes) as the target of the single call tuple.
        let mut padded_target = [0u8; 32];
        padded_target[12..].copy_from_slice(diamond_proxy.as_slice());
        assert!(calldata.windows(32).any(|w| w == padded_target));
    }
}
