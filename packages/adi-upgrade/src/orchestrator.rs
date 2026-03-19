//! Main upgrade orchestration logic.
//!
//! Coordinates all upgrade phases in sequence.

use std::path::Path;

use crate::broadcast::{run_broadcast, BroadcastResult};
use crate::config::UpgradeConfig;
use crate::error::Result;
use crate::simulation::{run_simulation, SimulationResult, ToolkitRunnerTrait};
use crate::versions::VersionHandler;

/// Upgrade orchestrator that coordinates all phases.
pub struct UpgradeOrchestrator<'a, R> {
    handler: &'a dyn VersionHandler,
    config: &'a UpgradeConfig,
    state_dir: &'a Path,
    runner: &'a R,
    protocol_version: semver::Version,
}

impl<'a, R: ToolkitRunnerTrait> UpgradeOrchestrator<'a, R> {
    /// Create a new upgrade orchestrator.
    pub fn new(
        handler: &'a dyn VersionHandler,
        config: &'a UpgradeConfig,
        state_dir: &'a Path,
        runner: &'a R,
        protocol_version: semver::Version,
    ) -> Self {
        Self {
            handler,
            config,
            state_dir,
            runner,
            protocol_version,
        }
    }

    /// Run simulation phase.
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

    /// Run broadcast phase.
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
}
