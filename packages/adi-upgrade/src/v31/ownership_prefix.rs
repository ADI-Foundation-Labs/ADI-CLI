//! Ownership pre-fix: transfer CAH / VT / NTV / NUL to governance so v31
//! stage 1's onlyOwner calls do not revert.

use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::onchain::query_owner;
use crate::signing::build_signing_provider;
use crate::v31::bundle::{BundleInfo, BundleTx};

sol! {
    #[sol(rpc)]
    interface IBridgehubCah {
        function chainAssetHandler() external view returns (address);
    }

    function transferOwnership(address newOwner) external;
    function acceptOwnership() external;

    struct GovCall {
        address target;
        uint256 value;
        bytes data;
    }
    struct Operation {
        GovCall[] calls;
        bytes32 predecessor;
        bytes32 salt;
    }
    function scheduleTransparent(Operation _operation, uint256 _delay) external;
    function execute(Operation _operation) external;
}

/// Contracts the v31 upgrade requires governance to own (besides CAH, which is
/// resolved on-chain from the bridgehub).
#[derive(Debug, Clone, Copy)]
pub struct OwnershipTargets {
    /// ValidatorTimelock.
    pub validator_timelock: Address,
    /// Native token vault.
    pub native_token_vault: Address,
    /// L1 nullifier.
    pub l1_nullifier: Address,
}

/// Ensure CAH + VT/NTV/NUL are owned by governance. Idempotent — targets already
/// governance-owned are skipped. `salt` makes the governance op id unique per run.
///
/// In `manual` mode nothing is sent: the transfer + governance-accept transactions
/// are returned as bundles for the operator to execute. Otherwise they are signed
/// and broadcast and an empty vec is returned.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] on RPC failure, a missing signer key (broadcast
/// only), or an ownerless target (CAH restore is mainnet-specific, not handled here).
pub async fn ensure_governance_owned<P: Provider + Clone>(
    provider: &P,
    config: &UpgradeConfig,
    targets: &OwnershipTargets,
    salt: B256,
    manual: bool,
    unlocked: bool,
) -> Result<Vec<BundleInfo>> {
    let bridgehub = config.bridgehub_address;
    let governance = query_owner(provider, bridgehub).await?;
    let cah = IBridgehubCah::new(bridgehub, provider)
        .chainAssetHandler()
        .call()
        .await
        .map_err(|e| UpgradeError::Config(format!("bridgehub.chainAssetHandler: {e}")))?;

    let all = [
        cah,
        targets.validator_timelock,
        targets.native_token_vault,
        targets.l1_nullifier,
    ];

    let mut bundles = Vec::new();
    let mut to_accept = Vec::new();
    for target in all {
        if target == Address::ZERO {
            continue;
        }
        let owner = query_owner(provider, target).await?;
        if owner == governance {
            continue;
        }
        if owner == Address::ZERO {
            return Err(UpgradeError::Config(format!(
                "target {target} is ownerless (owner == 0x0); restore it via its ProxyAdmin first \
                 (the mainnet cah-restorer flow), then re-run"
            )));
        }
        let data = transferOwnershipCall {
            newOwner: governance,
        }
        .abi_encode();
        if manual {
            bundles.push(BundleInfo {
                label: format!("ownership: transfer {target} to governance"),
                target: owner,
                txs: vec![BundleTx {
                    to: target,
                    value: U256::ZERO,
                    data: Bytes::from(data),
                }],
                safe_json_path: None,
            });
        } else if unlocked {
            send_tx_unlocked(provider, owner, target, data).await?;
        } else {
            send_tx(provider, owner_key(config, owner)?, target, data).await?;
        }
        to_accept.push(target);
    }

    if to_accept.is_empty() {
        return Ok(bundles);
    }

    let calls = to_accept
        .iter()
        .map(|t| GovCall {
            target: *t,
            value: U256::ZERO,
            data: Bytes::from(acceptOwnershipCall {}.abi_encode()),
        })
        .collect();
    let op = Operation {
        calls,
        predecessor: B256::ZERO,
        salt,
    };
    let schedule = scheduleTransparentCall {
        _operation: op.clone(),
        _delay: U256::ZERO,
    }
    .abi_encode();
    let exec = executeCall { _operation: op }.abi_encode();
    // schedule/execute are onlyOwner on the Governance contract, so the signer is
    // Governance.owner() (query it on-chain, not the wallets.yaml governor, which
    // can differ for a bridged or cloned state).
    let gov_owner = query_owner(provider, governance).await?;
    if manual {
        bundles.push(BundleInfo {
            label: "ownership: governance accept (schedule + execute)".to_string(),
            target: gov_owner,
            txs: vec![
                BundleTx {
                    to: governance,
                    value: U256::ZERO,
                    data: Bytes::from(schedule),
                },
                BundleTx {
                    to: governance,
                    value: U256::ZERO,
                    data: Bytes::from(exec),
                },
            ],
            safe_json_path: None,
        });
    } else if unlocked {
        send_tx_unlocked(provider, gov_owner, governance, schedule).await?;
        send_tx_unlocked(provider, gov_owner, governance, exec).await?;
    } else {
        let key = owner_key(config, gov_owner)?;
        send_tx(provider, key, governance, schedule).await?;
        send_tx(provider, key, governance, exec).await?;
    }
    Ok(bundles)
}

/// Send a tx as `from` via node impersonation (`anvil_impersonateAccount`), for
/// fork rehearsals where no signing key is held. Asserts the receipt succeeded.
async fn send_tx_unlocked<P: Provider + Clone>(
    provider: &P,
    from: Address,
    to: Address,
    data: Vec<u8>,
) -> Result<()> {
    let _: serde_json::Value = provider
        .raw_request("anvil_impersonateAccount".into(), (from,))
        .await
        .map_err(|e| UpgradeError::Config(format!("impersonate {from}: {e}")))?;
    // Impersonated mainnet owners usually hold no ETH on the fork; fund gas.
    let _: serde_json::Value = provider
        .raw_request("anvil_setBalance".into(), (from, "0x21e19e0c9bab2400000"))
        .await
        .map_err(|e| UpgradeError::Config(format!("fund {from}: {e}")))?;
    let tx = TransactionRequest::default()
        .from(from)
        .to(to)
        .input(Bytes::from(data).into())
        .value(U256::ZERO);
    let receipt = provider
        .send_transaction(tx)
        .await
        .map_err(|e| UpgradeError::Config(format!("send (unlocked) to {to} from {from}: {e}")))?
        .get_receipt()
        .await
        .map_err(|e| UpgradeError::Config(format!("receipt from {to}: {e}")))?;
    if !receipt.status() {
        return Err(UpgradeError::Config(format!(
            "unlocked tx to {to} from {from} reverted (status 0)"
        )));
    }
    Ok(())
}

async fn send_tx<P: Provider + Clone>(
    provider: &P,
    key: &secrecy::SecretString,
    to: Address,
    data: Vec<u8>,
) -> Result<()> {
    let signed = build_signing_provider(provider, key)?;
    let tx = TransactionRequest::default()
        .to(to)
        .input(Bytes::from(data).into())
        .value(U256::ZERO);
    let receipt = signed
        .send_transaction(tx)
        .await
        .map_err(|e| UpgradeError::Config(format!("send to {to}: {e}")))?
        .get_receipt()
        .await
        .map_err(|e| UpgradeError::Config(format!("receipt from {to}: {e}")))?;
    if !receipt.status() {
        return Err(UpgradeError::Config(format!(
            "tx to {to} reverted (status 0)"
        )));
    }
    Ok(())
}

fn owner_key(config: &UpgradeConfig, owner: Address) -> Result<&secrecy::SecretString> {
    if owner == config.deployer_address {
        return Ok(&config.deployer_private_key);
    }
    if owner == config.governor_address {
        return Ok(&config.governor_private_key);
    }
    Err(UpgradeError::Config(format!(
        "no signer key for owner {owner} (not deployer or governor)"
    )))
}
