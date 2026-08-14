//! v31 upgrade orchestration: one method per protocol_ops phase over a generic runner.

use std::path::Path;
use std::time::Duration;

use alloy_primitives::{address, Address, B256, U256};
use alloy_provider::Provider;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::onchain;
use crate::runner::ToolkitRunnerTrait;
use crate::v31::bundle::{self, BundleInfo};
use crate::v31::env_toml::{write_env_tomls, V31EnvInputs, V31_ENV_DIR};
use crate::v31::outputs;
use crate::v31::ownership_prefix::{ensure_governance_owned, OwnershipTargets};
use crate::v31::readiness::wait_for_finalized_block;
use crate::v31::script::{self, Signer, CHAIN_KEY_VAR, DEP_KEY_VAR, GOV_KEY_VAR};
use crate::versions::V0_31_0Handler;

/// L2 base-token system contract holding the tracked total supply.
const L2_BASE_TOKEN: Address = address!("000000000000000000000000000000000000800a");

/// Parameters for constructing a [`V31Orchestrator`].
pub struct V31OrchestratorParams<'a, R, P> {
    /// Version metadata handler.
    pub handler: &'a V0_31_0Handler,
    /// Upgrade configuration (bridgehub, governor, deployer, keys).
    pub config: &'a UpgradeConfig,
    /// Ecosystem state directory (bind-mounted to `/workspace`).
    pub state_dir: &'a Path,
    /// Toolkit runner for Docker execution.
    pub runner: &'a R,
    /// Alloy provider for on-chain queries.
    pub provider: &'a P,
    /// Target protocol version (image tag).
    pub protocol_version: semver::Version,
    /// Env preset name (e.g. `adi-local`).
    pub env: &'a str,
    /// Values rendered into the two protocol_ops env TOMLs.
    pub inputs: &'a V31EnvInputs,
    /// L1 RPC reachable from inside the container.
    pub l1_rpc_container: &'a str,
    /// L2 RPC (host) for the readiness gate.
    pub l2_rpc: &'a Url,
    /// L2 RPC reachable from inside the container (total-supply calc).
    pub l2_rpc_container: &'a str,
    /// Chain being upgraded.
    pub chain_id: u64,
    /// Chain-level ChainAdmin owner (signs set-ts / upgrade / total-supply bundles).
    pub chain_governor_address: Address,
    /// Chain governor private key (chain-level wallets.yaml).
    pub chain_governor_private_key: SecretString,
    /// Upgrade timestamp in unix seconds (`1` = immediately, used locally).
    pub upgrade_timestamp: u64,
    /// Output-only mode: surface bundles for the operator instead of broadcasting.
    pub manual: bool,
    /// Fork rehearsal: broadcast via node impersonation (`--unlocked`), no keys.
    pub unlocked: bool,
    /// Output-only hybrid: broadcast the deployer-signed phases (prepare, stage3)
    /// with the deployer key; surface governance/chain-owner phases as bundles.
    pub sign_deployer: bool,
}

/// Orchestrates the v31 upgrade phases.
pub struct V31Orchestrator<'a, R, P>(V31OrchestratorParams<'a, R, P>);

impl<'a, R, P> V31Orchestrator<'a, R, P>
where
    R: ToolkitRunnerTrait,
    P: Provider + Clone,
{
    /// Create a new v31 orchestrator.
    pub fn new(params: V31OrchestratorParams<'a, R, P>) -> Self {
        Self(params)
    }

    /// Phase 0: write the two protocol_ops env preset TOMLs into the state dir.
    pub fn prepare_env(&self) -> Result<()> {
        write_env_tomls(&self.0.state_dir.join("l1-contracts"), self.0.inputs)?;
        Ok(())
    }

    /// pre-fix: ensure CAH/VT/NTV/NUL are governance-owned before the upgrade.
    /// Returns transfer/accept bundles in manual mode; broadcasts and returns empty otherwise.
    pub async fn ownership_prefix(
        &self,
        targets: &OwnershipTargets,
        salt: B256,
    ) -> Result<Vec<BundleInfo>> {
        ensure_governance_owned(
            self.0.provider,
            self.0.config,
            targets,
            salt,
            self.0.manual,
            self.0.unlocked,
        )
        .await
    }

    /// Phase 1: deploy new ecosystem contracts on the fork; emit bundles.
    pub async fn prepare(&self) -> Result<Vec<BundleInfo>> {
        let body = script::prepare_all(
            self.0.env,
            self.0.config.bridgehub_address,
            self.0.config.deployer_address,
            self.0.l1_rpc_container,
        );
        self.run("prepare-all", &body, false).await?;
        self.finish("prepare", "prepare", true).await
    }

    /// Phase 2: governance stages 0+1+2.
    pub async fn governance(&self) -> Result<Vec<BundleInfo>> {
        let body = script::governance_prepare(self.0.env, self.0.l1_rpc_container);
        self.run("governance", &body, false).await?;
        self.finish("governance", "governance", false).await
    }

    /// Discover legacy bridged tokens and write the stage3 input list.
    /// Read-only; no bundle. Run before [`Self::stage3`].
    pub async fn discover_bridged_tokens(&self) -> Result<()> {
        // Start the scan at the bridgehub's deploy block, not genesis: the empty
        // pre-deployment history is the dominant cost of this phase. Fall back to 0
        // if detection fails so discovery can never silently miss a deposit.
        let from_block = onchain::deploy_block(self.0.provider, self.0.config.bridgehub_address)
            .await
            .unwrap_or(0);
        let body = script::discover_bridged_tokens(self.0.env, self.0.l1_rpc_container, from_block);
        self.run("discover-bridged-tokens", &body, false).await
    }

    /// Number of bridged tokens the discover phase found (0 = ETH-only stage3).
    pub fn bridged_tokens_count(&self) -> Result<usize> {
        outputs::bridged_tokens_count(self.0.state_dir, self.0.env)
    }

    /// Phase 3: legacy-token registration (stage3).
    pub async fn stage3(&self) -> Result<Vec<BundleInfo>> {
        let body = script::stage3_prepare(
            self.0.env,
            self.0.config.bridgehub_address,
            self.0.config.deployer_address,
            self.0.l1_rpc_container,
        );
        self.run("stage3", &body, false).await?;
        self.finish("stage3", "stage3", true).await
    }

    /// Set the upgrade timestamp so the server detects the pending upgrade.
    pub async fn set_upgrade_timestamp(&self) -> Result<Vec<BundleInfo>> {
        let body = script::set_upgrade_timestamp(&script::UpgradeTimestampParams {
            env: self.0.env,
            chain_id: self.0.chain_id,
            new_protocol_version: self.0.handler.new_protocol_version(),
            upgrade_timestamp: self.0.upgrade_timestamp,
            l1_rpc: self.0.l1_rpc_container,
        });
        self.run("set-upgrade-timestamp", &body, false).await?;
        let rel = format!("set-upgrade-timestamp/{}", self.0.chain_id);
        self.finish("set-upgrade-timestamp", &rel, false).await
    }

    /// Finalize the diamond cut; `da` optionally re-sets the DA validator pair.
    pub async fn chain_upgrade(&self, da: Option<(Address, &str)>) -> Result<Vec<BundleInfo>> {
        let body = script::chain_upgrade(self.0.env, self.0.chain_id, da, self.0.l1_rpc_container);
        self.run("chain-upgrade", &body, false).await?;
        let rel = format!("upgrade/{}", self.0.chain_id);
        self.finish("chain-upgrade", &rel, false).await
    }

    /// Compute + record the pre-v31 base-token total supply. `finalize_tx` is the
    /// chain-upgrade tx the calc scans chain history from.
    pub async fn set_total_supply(&self, finalize_tx: &str) -> Result<Vec<BundleInfo>> {
        let body = script::set_total_supply(&script::TotalSupplyParams {
            env: self.0.env,
            chain_id: self.0.chain_id,
            bridgehub: self.0.config.bridgehub_address,
            l1_rpc: self.0.l1_rpc_container,
            l2_rpc: self.0.l2_rpc_container,
            finalize_tx,
        });
        self.run("set-total-supply", &body, false).await?;
        let rel = format!("set-zkos-pre-v31-total-supply/{}", self.0.chain_id);
        self.finish("set-total-supply", &rel, false).await
    }

    /// After a phase's prepare emits its manifest: broadcast it (execute mode) and
    /// return empty, or (manual mode) read it back as bundles for the operator. An
    /// absent manifest (e.g. stage3 with no bridged tokens) is a no-op.
    async fn finish(
        &self,
        label: &str,
        rel: &str,
        deployer_phase: bool,
    ) -> Result<Vec<BundleInfo>> {
        let manifest = self
            .0
            .state_dir
            .join("l1-contracts/upgrade-envs")
            .join(V31_ENV_DIR)
            .join("output")
            .join(self.0.env)
            .join(rel)
            .join("manifest.json");
        if !manifest.exists() {
            return Ok(Vec::new());
        }
        // Output-only surfaces bundles for the operator — except that --sign-deployer
        // broadcasts the deployer-signed phases (prepare, stage3) directly, since
        // forcing ~40 deploys through calldata is pointless.
        let hybrid_deployer = self.0.manual && self.0.sign_deployer && deployer_phase;
        if self.0.manual && !hybrid_deployer {
            return bundle::read_bundles(&manifest);
        }
        let signers = if hybrid_deployer {
            vec![Signer {
                address: self.0.config.deployer_address,
                key_var: DEP_KEY_VAR,
            }]
        } else {
            self.chain_signers().to_vec()
        };
        let body = script::broadcast(&script::BroadcastParams {
            env: self.0.env,
            rel,
            l1_rpc: self.0.l1_rpc_container,
            signers: &signers,
            unlocked: self.0.unlocked,
            skip_unkeyed: hybrid_deployer,
        });
        // Unlocked (fork) broadcast impersonates on the node, so no keys are passed.
        self.run(&format!("{label}-broadcast"), &body, !self.0.unlocked)
            .await?;
        // Hybrid broadcast keyed (deployer) txs only; surface the rest (Safe/owner).
        if hybrid_deployer {
            let rest = bundle::read_bundles(&manifest)?
                .into_iter()
                .filter(|b| b.target != self.0.config.deployer_address)
                .collect();
            return Ok(rest);
        }
        Ok(Vec::new())
    }

    /// The finalize (`upgradeChainFromVersion`) tx hash from the chain-upgrade
    /// broadcast output; input to [`Self::set_total_supply`].
    pub fn finalize_tx(&self) -> Result<String> {
        outputs::finalize_tx_hash(self.0.state_dir, self.0.env, self.0.chain_id)
    }

    /// The new BytecodesSupplier prepare deployed.
    pub fn new_bytecodes_supplier(&self) -> Result<String> {
        outputs::new_bytecodes_supplier(self.0.state_dir, self.0.env)
    }

    /// The chain's current diamond protocol version on L1.
    pub async fn diamond_protocol_version(&self) -> Result<U256> {
        let diamond = onchain::query_zk_chain(
            self.0.provider,
            self.0.config.bridgehub_address,
            self.0.chain_id,
        )
        .await?;
        onchain::query_diamond_protocol_version(self.0.provider, diamond).await
    }

    /// The ecosystem CTM's current protocol version on L1.
    pub async fn ctm_protocol_version(&self) -> Result<U256> {
        let ctm = onchain::query_ctm(
            self.0.provider,
            self.0.config.bridgehub_address,
            self.0.chain_id,
        )
        .await?;
        onchain::query_ctm_protocol_version(self.0.provider, ctm).await
    }

    /// The L2 base-token total supply.
    pub async fn l2_base_token_total_supply(&self) -> Result<U256> {
        let provider = onchain::create_provider(self.0.l2_rpc);
        onchain::query_total_supply(&provider, L2_BASE_TOKEN).await
    }

    /// The v31 protocol version this upgrade targets.
    #[must_use]
    pub fn target_version(&self) -> U256 {
        U256::from(self.0.handler.new_protocol_version())
    }

    /// The protocol version being upgraded from (v30.1).
    #[must_use]
    pub fn source_version(&self) -> U256 {
        U256::from(self.0.handler.old_protocol_version())
    }

    /// Current L2 head block number.
    pub async fn l2_head(&self) -> Result<u64> {
        let provider = onchain::create_provider(self.0.l2_rpc);
        provider
            .get_block_number()
            .await
            .map_err(|e| UpgradeError::Config(format!("L2 head: {e}")))
    }

    /// Wait until the L2 finalized block reaches `target`.
    pub async fn readiness_gate(&self, target: u64) -> Result<u64> {
        wait_for_finalized_block(
            self.0.l2_rpc,
            target,
            Duration::from_secs(600),
            Duration::from_secs(3),
        )
        .await
    }

    /// Pre-finalize gate: block until every block up to `target` (the last
    /// pre-v31 block) is executed on L1, so the V6 to V7 verifier swap never
    /// strands an un-executed pre-v31 batch. Generous timeout; failure is fatal.
    pub async fn wait_pre_upgrade_drain(&self, target: u64) -> Result<u64> {
        wait_for_finalized_block(
            self.0.l2_rpc,
            target,
            Duration::from_secs(1800),
            Duration::from_secs(5),
        )
        .await
    }

    /// Best-effort drain before the supply read: wait until the L2 finalized
    /// block catches up to its current head. Callers treat failure as non-fatal.
    pub async fn wait_for_server_drain(&self) -> Result<u64> {
        let head = self.l2_head().await?;
        self.readiness_gate(head).await
    }

    /// Verify the chain reached the v31 protocol version on L1.
    pub async fn verify(&self) -> Result<bool> {
        Ok(self.diamond_protocol_version().await? == self.target_version())
    }

    async fn run(&self, label: &str, body: &str, with_key: bool) -> Result<()> {
        // Keys are exposed only for a broadcast; prepares and manual mode pass none.
        let env_vars: Vec<(&str, &str)> = if with_key {
            vec![
                (
                    GOV_KEY_VAR,
                    self.0.config.governor_private_key.expose_secret(),
                ),
                (
                    DEP_KEY_VAR,
                    self.0.config.deployer_private_key.expose_secret(),
                ),
                (
                    CHAIN_KEY_VAR,
                    self.0.chain_governor_private_key.expose_secret(),
                ),
            ]
        } else {
            Vec::new()
        };
        let code = self
            .0
            .runner
            .run_command(
                &["sh", "-c", body],
                self.0.state_dir,
                &self.0.protocol_version,
                &env_vars,
            )
            .await
            .map_err(|e| UpgradeError::Config(format!("protocol_ops {label}: {e}")))?;
        if code != 0 {
            return Err(UpgradeError::Config(format!(
                "protocol_ops {label} exited with code {code}"
            )));
        }
        Ok(())
    }

    /// Offer every signer key; protocol_ops matches whichever each bundle names
    /// (eco governor + deployer for ecosystem phases, chain ChainAdmin owner for
    /// chain phases).
    fn chain_signers(&self) -> [Signer<'static>; 3] {
        [
            Signer {
                address: self.0.config.governor_address,
                key_var: GOV_KEY_VAR,
            },
            Signer {
                address: self.0.config.deployer_address,
                key_var: DEP_KEY_VAR,
            },
            Signer {
                address: self.0.chain_governor_address,
                key_var: CHAIN_KEY_VAR,
            },
        ]
    }
}
