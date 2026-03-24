//! Main upgrade orchestration logic.
//!
//! Coordinates all upgrade phases in sequence:
//! 1. Prepare config (generate chain.toml)
//! 2. Simulate (forge script dry run)
//! 3. Broadcast (forge script with --broadcast)
//! 4. Generate upgrade YAML (yarn upgrade-yaml-output-generator)
//! 5. Execute governance (scheduleTransparent + execute)
//! 6. Chain upgrades (per-chain via zkstack)
//! 7. Save upgrade YAML

use std::path::{Path, PathBuf};

use alloy_provider::Provider;

use crate::broadcast::{run_broadcast, BroadcastResult};
use crate::chain_toml::{generate_chain_toml, write_chain_toml, PreviousUpgradeValues};
use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::governance::{
    encode_governance_calls, execute_governance, extract_stage1_calls,
    resolve_governance_contracts, GovernanceCalldata, GovernanceResult,
};
use crate::simulation::{run_simulation, SimulationResult, ToolkitRunnerTrait};
use crate::upgrade_yaml;
use crate::versions::VersionHandler;

/// Upgrade orchestrator that coordinates all phases.
pub struct UpgradeOrchestrator<'a, R, P> {
    handler: &'a dyn VersionHandler,
    config: &'a UpgradeConfig,
    state_dir: &'a Path,
    runner: &'a R,
    provider: &'a P,
    protocol_version: semver::Version,
}

impl<'a, R, P> UpgradeOrchestrator<'a, R, P>
where
    R: ToolkitRunnerTrait,
    P: Provider + Clone,
{
    /// Create a new upgrade orchestrator.
    pub fn new(
        handler: &'a dyn VersionHandler,
        config: &'a UpgradeConfig,
        state_dir: &'a Path,
        runner: &'a R,
        provider: &'a P,
        protocol_version: semver::Version,
    ) -> Self {
        Self {
            handler,
            config,
            state_dir,
            runner,
            provider,
            protocol_version,
        }
    }

    /// Phase 1: Prepare upgrade config (generate chain.toml).
    pub async fn prepare_config(
        &self,
        chain_id: u64,
        previous_values: &PreviousUpgradeValues,
    ) -> Result<()> {
        log::info!("Preparing upgrade configuration");

        let content = generate_chain_toml(
            self.handler,
            self.config,
            self.provider,
            chain_id,
            previous_values,
        )
        .await?;

        write_chain_toml(&content, self.state_dir, self.handler.upgrade_env_dir())?;

        log::info!("chain.toml generated successfully");
        Ok(())
    }

    /// Phase 2: Run simulation (forge script dry run).
    pub async fn simulate(&self) -> Result<SimulationResult> {
        run_simulation(
            self.handler,
            self.config,
            self.state_dir,
            self.runner,
            &self.protocol_version,
        )
        .await
    }

    /// Phase 3: Run broadcast (forge script with --broadcast).
    pub async fn broadcast(&self) -> Result<BroadcastResult> {
        run_broadcast(
            self.handler,
            self.config,
            self.state_dir,
            self.runner,
            &self.protocol_version,
        )
        .await
    }

    /// Phase 4: Generate upgrade YAML from broadcast output.
    ///
    /// Reads the forge TOML output and broadcast JSON, extracts transaction
    /// hashes, and produces the ecosystem YAML needed for governance encoding.
    pub fn generate_upgrade_yaml(&self, l1_chain_id: u64) -> Result<()> {
        let l1_contracts = self.state_dir.join("l1-contracts");

        let toml_path = l1_contracts
            .join("script-out")
            .join(self.handler.upgrade_output_toml());
        let broadcast_path = l1_contracts.join(format!(
            "broadcast/{}/{}/run-latest.json",
            self.handler.upgrade_script(),
            l1_chain_id
        ));
        let yaml_path = l1_contracts
            .join("script-out")
            .join(self.handler.upgrade_output_yaml());

        crate::yaml_generator::generate_upgrade_yaml_from_files(
            &toml_path,
            &broadcast_path,
            &yaml_path,
        )
    }

    /// Phase 5: Extract stage1 calls and encode governance calldata.
    pub fn encode_governance(&self) -> Result<GovernanceCalldata> {
        let toml_path = self
            .state_dir
            .join("l1-contracts")
            .join("script-out")
            .join(self.handler.upgrade_output_toml());

        let toml_content = std::fs::read_to_string(&toml_path).map_err(|e| {
            UpgradeError::GovernanceFailed(format!(
                "Failed to read TOML output at {}: {e}",
                toml_path.display()
            ))
        })?;

        let stage1_calls = extract_stage1_calls(&toml_content)?;
        encode_governance_calls(&stage1_calls)
    }

    /// Phase 5: Resolve governance contracts and execute transactions.
    pub async fn execute_governance(&self) -> Result<GovernanceResult> {
        let calldata = self.encode_governance()?;

        let addresses =
            resolve_governance_contracts(self.provider, self.config.bridgehub_address).await?;

        execute_governance(
            self.provider,
            &self.config.governor_private_key,
            addresses.governance,
            calldata.schedule_transparent,
            calldata.execute,
        )
        .await
    }

    /// Phase 6: Save upgrade output YAML to state directory.
    pub fn save_upgrade_yaml(&self) -> Result<PathBuf> {
        let source = self
            .state_dir
            .join("l1-contracts")
            .join("script-out")
            .join(self.handler.upgrade_output_yaml());

        upgrade_yaml::save_upgrade_yaml(&source, self.state_dir, self.handler.upgrade_output_yaml())
    }
}
