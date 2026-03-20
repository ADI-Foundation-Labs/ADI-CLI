//! Resolve governance contract addresses on-chain.

use alloy_primitives::Address;
use alloy_provider::Provider;

use crate::error::Result;
use crate::onchain;

/// Resolved governance contract addresses.
#[derive(Debug, Clone)]
pub struct GovernanceAddresses {
    /// Governance contract address (owner of BridgeHub).
    pub governance: Address,
    /// Ecosystem governor address (owner of Governance contract).
    pub governor: Address,
}

/// Resolve governance contracts from bridgehub.
///
/// Queries: bridgehub.owner() -> governance, governance.owner() -> governor.
pub async fn resolve_governance_contracts<P: Provider + Clone>(
    provider: &P,
    bridgehub: Address,
) -> Result<GovernanceAddresses> {
    log::info!(
        "Resolving governance contracts from bridgehub {}",
        bridgehub
    );

    let governance = onchain::query_owner(provider, bridgehub).await?;
    log::info!("Governance contract: {}", governance);

    let governor = onchain::query_owner(provider, governance).await?;
    log::info!("Ecosystem governor: {}", governor);

    Ok(GovernanceAddresses {
        governance,
        governor,
    })
}
