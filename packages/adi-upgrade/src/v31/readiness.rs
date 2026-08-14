//! Poll the L2 `finalized` block until it reaches a target height (readiness gate).

use std::time::{Duration, Instant};

use alloy_provider::Provider;
use alloy_rpc_types::BlockNumberOrTag;
use url::Url;

use crate::error::{Result, UpgradeError};
use crate::onchain::create_provider;

/// Poll the `finalized` block on `rpc_url` until it is at least `target`, or
/// `timeout` elapses. Returns the finalized block height reached. Transient RPC
/// errors are retried until the deadline rather than aborting the gate.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the deadline elapses before the target is
/// reached, or if an RPC call fails at or after the deadline.
pub async fn wait_for_finalized_block(
    rpc_url: &Url,
    target: u64,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<u64> {
    let provider = create_provider(rpc_url);
    let deadline = Instant::now() + timeout;
    loop {
        // A single transient RPC error must not abort a healthy 30-min drain; retry
        // until the deadline and only then surface the failure.
        let finalized = match finalized_block_number(&provider).await {
            Ok(n) => n,
            Err(e) => {
                if Instant::now() >= deadline {
                    return Err(e);
                }
                tokio::time::sleep(poll_interval).await;
                continue;
            }
        };
        if finalized >= target {
            return Ok(finalized);
        }
        if Instant::now() >= deadline {
            return Err(UpgradeError::Config(format!(
                "readiness gate timed out: finalized block {finalized} < target {target} after {timeout:?}"
            )));
        }
        tokio::time::sleep(poll_interval).await;
    }
}

/// Read the current `finalized` block number, or 0 if the tag has no block yet.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if the RPC call fails.
pub async fn finalized_block_number(provider: &(impl Provider + Clone)) -> Result<u64> {
    let block = provider
        .get_block_by_number(BlockNumberOrTag::Finalized)
        .await
        .map_err(|e| UpgradeError::Config(format!("get finalized block: {e}")))?;
    Ok(block.map(|b| b.header.number).unwrap_or(0))
}
