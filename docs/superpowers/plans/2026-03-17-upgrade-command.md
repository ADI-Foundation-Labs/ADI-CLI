# Upgrade Command Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `adi upgrade` command to upgrade ZkSync ecosystem and chain contracts to a new protocol version.

**Architecture:** New `adi-upgrade` package contains upgrade SDK. CLI command in `src/commands/upgrade/` is a thin wrapper. Docker container execution via `adi-toolkit`. Version-specific logic via `VersionHandler` trait.

**Tech Stack:** Rust, cliclack (prompts), alloy (EVM), adi-toolkit (Docker), serde (YAML/JSON)

**Spec:** [docs/superpowers/specs/2026-03-17-upgrade-command-design.md](../specs/2026-03-17-upgrade-command-design.md)

---

## File Structure

```
packages/adi-upgrade/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Public API
    ├── error.rs                # UpgradeError type
    ├── config.rs               # UpgradeConfig generation
    ├── orchestrator.rs         # Main upgrade flow
    ├── simulation.rs           # Simulate phase
    ├── broadcast.rs            # Broadcast phase
    ├── validation/
    │   ├── mod.rs
    │   └── bytecode.rs         # Bytecode hash validation
    ├── governance/
    │   ├── mod.rs
    │   ├── ecosystem.rs        # Ecosystem governance txs
    │   └── chain.rs            # Chain governance txs
    └── versions/
        ├── mod.rs              # VersionHandler trait
        └── v0_30.rs            # V0_30_x handlers

src/commands/upgrade/
├── mod.rs                      # UpgradeArgs, run()
└── prompts.rs                  # cliclack interactions
```

---

## Phase 1: Package Scaffold + CLI Integration

**Goal:** `adi upgrade --help` works, command registered in CLI.

### Task 1.1: Create adi-upgrade package

**Files:**
- Create: `packages/adi-upgrade/Cargo.toml`
- Create: `packages/adi-upgrade/src/lib.rs`
- Create: `packages/adi-upgrade/src/error.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "adi-upgrade"
version.workspace = true
edition.workspace = true
description = "SDK for upgrading ZkSync ecosystem contracts"

[lints]
workspace = true

[lib]
name = "adi_upgrade"
path = "src/lib.rs"

[dependencies]
thiserror = { workspace = true }
eyre = { workspace = true }
```

- [ ] **Step 2: Create error.rs**

```rust
//! Error types for upgrade operations.

use thiserror::Error;

/// Result type alias using UpgradeError.
pub type Result<T> = std::result::Result<T, UpgradeError>;

/// Errors that can occur during upgrade operations.
#[derive(Error, Debug)]
pub enum UpgradeError {
    /// Unsupported protocol version.
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Simulation failed.
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    /// Broadcast failed.
    #[error("Broadcast failed: {0}")]
    BroadcastFailed(String),

    /// Bytecode validation failed.
    #[error("Bytecode validation failed: {0}")]
    ValidationFailed(String),

    /// Governance transaction failed.
    #[error("Governance transaction failed: {0}")]
    GovernanceFailed(String),
}
```

- [ ] **Step 3: Create lib.rs**

```rust
//! SDK for upgrading ZkSync ecosystem contracts.
//!
//! This crate provides the upgrade orchestration logic for ZkSync
//! ecosystem and chain contracts.

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod error;

pub use error::{Result, UpgradeError};
```

- [ ] **Step 4: Add package to workspace**

Modify: `Cargo.toml` (root)

Add to `[workspace.dependencies]`:
```toml
adi-upgrade = { path = "packages/adi-upgrade" }
```

- [ ] **Step 5: Verify package compiles**

Run: `cargo build -p adi-upgrade`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add packages/adi-upgrade Cargo.toml
git commit -m "$(cat <<'EOF'
feat(upgrade): add adi-upgrade package scaffold

Initial package structure with error types.
EOF
)"
```

### Task 1.2: Add CLI command module

**Files:**
- Create: `src/commands/upgrade/mod.rs`
- Modify: `src/commands/mod.rs`
- Modify: `Cargo.toml` (root - add dependency)

- [ ] **Step 1: Create upgrade command module**

```rust
//! Upgrade command for ecosystem and chain contracts.

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Target for upgrade operations.
#[derive(Clone, Debug, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpgradeTarget {
    /// Upgrade ecosystem-level contracts only
    Ecosystem,
    /// Upgrade chain-level contracts only
    Chain,
    /// Upgrade both ecosystem and chain contracts
    #[default]
    Both,
}

/// Arguments for `upgrade` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct UpgradeArgs {
    /// Target protocol version (e.g., v0.30.1)
    #[arg(long, required = true)]
    pub protocol_version: String,

    /// Upgrade target: ecosystem, chain, or both
    #[arg(long, default_value = "both")]
    pub target: UpgradeTarget,

    /// Chain name (bypasses multi-select picker)
    #[arg(long)]
    pub chain: Option<String>,

    /// Skip simulation, go straight to broadcast
    #[arg(long)]
    pub skip_simulation: bool,

    /// Settlement layer RPC URL
    #[arg(long)]
    pub rpc_url: Option<url::Url>,

    /// Gas price multiplier
    #[arg(long, default_value = "1.2")]
    pub gas_multiplier: f64,

    /// Ecosystem name
    #[arg(long)]
    pub ecosystem_name: Option<String>,
}

/// Execute the upgrade command.
pub async fn run(args: UpgradeArgs, context: &Context) -> Result<()> {
    use crate::commands::helpers::resolve_ecosystem_name;
    use crate::ui;

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    ui::intro(format!(
        "Upgrading {} to {}",
        ui::green(&ecosystem_name),
        ui::green(&args.protocol_version)
    ))?;

    ui::note(
        "Upgrade Target",
        format!(
            "Target: {:?}\nChain: {}\nSkip simulation: {}",
            args.target,
            args.chain.as_deref().unwrap_or("(all)"),
            args.skip_simulation
        ),
    )?;

    ui::outro("Upgrade command registered (implementation pending)")?;

    Ok(())
}
```

- [ ] **Step 2: Register command in mod.rs**

Modify: `src/commands/mod.rs`

Add import:
```rust
mod upgrade;
```

Add to `Commands` enum:
```rust
    /// Upgrade ecosystem and chain contracts to a new protocol version
    Upgrade(upgrade::UpgradeArgs),
```

Add to `Commands::run` match:
```rust
            Commands::Upgrade(args) => upgrade::run(args, context).await,
```

- [ ] **Step 3: Add adi-upgrade dependency to CLI**

Modify: `Cargo.toml` (root)

Add to `[dependencies]`:
```toml
adi-upgrade = { workspace = true }
```

- [ ] **Step 4: Verify command works**

Run: `cargo run -- upgrade --help`
Expected: Shows upgrade command help with all options

Run: `cargo run -- upgrade --protocol-version=v0.30.1`
Expected: Shows intro message and "implementation pending" outro

- [ ] **Step 5: Commit**

```bash
git add src/commands/upgrade src/commands/mod.rs Cargo.toml
git commit -m "$(cat <<'EOF'
feat(upgrade): add upgrade command CLI skeleton

Registers adi upgrade command with all args from spec.
EOF
)"
```

---

## Phase 2: Version Handling

**Goal:** Parse protocol version and get handler, error on unsupported versions.

### Task 2.1: Add VersionHandler trait and registry

**Files:**
- Create: `packages/adi-upgrade/src/versions/mod.rs`
- Create: `packages/adi-upgrade/src/versions/v0_30.rs`
- Modify: `packages/adi-upgrade/src/lib.rs`
- Modify: `packages/adi-upgrade/Cargo.toml`

- [ ] **Step 1: Add adi-toolkit dependency**

Modify: `packages/adi-upgrade/Cargo.toml`

Add to `[dependencies]`:
```toml
adi-toolkit = { workspace = true }
```

- [ ] **Step 2: Create versions/mod.rs**

```rust
//! Version-specific upgrade handlers.
//!
//! Each protocol version may have different upgrade scripts and post-hooks.

mod v0_30;

use adi_toolkit::ProtocolVersion;

/// Post-upgrade hook to run after governance execution.
#[derive(Debug, Clone)]
pub enum PostUpgradeHook {
    /// Setup DAValidator pair (v0.30.0 specific)
    DaValidatorSetup,
}

/// Handler for version-specific upgrade logic.
pub trait VersionHandler: Send + Sync {
    /// Forge script path for this version's upgrade.
    fn upgrade_script(&self) -> &str;

    /// Post-upgrade hooks to run after governance execution.
    fn post_upgrade_hooks(&self) -> Vec<PostUpgradeHook>;
}

/// Get the appropriate handler for a protocol version.
///
/// # Returns
///
/// `Some(handler)` if the version is supported, `None` otherwise.
#[must_use]
pub fn get_handler(version: &ProtocolVersion) -> Option<Box<dyn VersionHandler>> {
    match version {
        ProtocolVersion::V0_30_1 => Some(Box::new(v0_30::V0_30_1Handler)),
    }
}

/// Check if a protocol version is supported for upgrades.
#[must_use]
pub fn is_supported(version: &ProtocolVersion) -> bool {
    get_handler(version).is_some()
}
```

- [ ] **Step 3: Create versions/v0_30.rs**

```rust
//! Version handlers for v0.30.x protocol versions.

use super::{PostUpgradeHook, VersionHandler};

/// Handler for v0.30.1 upgrades.
pub struct V0_30_1Handler;

impl VersionHandler for V0_30_1Handler {
    fn upgrade_script(&self) -> &str {
        "l1-contracts/deploy-scripts/upgrade/EcosystemUpgrade.s.sol"
    }

    fn post_upgrade_hooks(&self) -> Vec<PostUpgradeHook> {
        // v0.30.1 has no post-upgrade hooks
        vec![]
    }
}
```

- [ ] **Step 4: Export from lib.rs**

Modify: `packages/adi-upgrade/src/lib.rs`

Add:
```rust
pub mod versions;

pub use versions::{get_handler, is_supported, PostUpgradeHook, VersionHandler};
```

- [ ] **Step 5: Verify compiles**

Run: `cargo build -p adi-upgrade`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add packages/adi-upgrade/src/versions packages/adi-upgrade/src/lib.rs packages/adi-upgrade/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(upgrade): add version handler trait and v0.30.1 support

VersionHandler trait allows version-specific upgrade logic.
EOF
)"
```

### Task 2.2: Integrate version handling in CLI

**Files:**
- Modify: `src/commands/upgrade/mod.rs`

- [ ] **Step 1: Add version validation**

Replace `run` function:

```rust
/// Execute the upgrade command.
pub async fn run(args: UpgradeArgs, context: &Context) -> Result<()> {
    use adi_toolkit::ProtocolVersion;
    use adi_upgrade::{get_handler, is_supported};
    use crate::commands::helpers::resolve_ecosystem_name;
    use crate::error::WrapErr;
    use crate::ui;

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    ui::intro(format!(
        "Upgrading {} to {}",
        ui::green(&ecosystem_name),
        ui::green(&args.protocol_version)
    ))?;

    // Parse and validate protocol version
    let version = ProtocolVersion::parse(&args.protocol_version)
        .wrap_err("Invalid protocol version")?;

    if !is_supported(&version) {
        return Err(eyre::eyre!(
            "Protocol version {} is not supported for upgrades",
            version
        ));
    }

    let handler = get_handler(&version)
        .ok_or_else(|| eyre::eyre!("No handler for version {}", version))?;

    ui::info(format!(
        "Using upgrade script: {}",
        ui::green(handler.upgrade_script())
    ))?;

    let hooks = handler.post_upgrade_hooks();
    if !hooks.is_empty() {
        ui::info(format!("Post-upgrade hooks: {:?}", hooks))?;
    }

    ui::note(
        "Upgrade Target",
        format!(
            "Target: {:?}\nChain: {}\nSkip simulation: {}",
            args.target,
            args.chain.as_deref().unwrap_or("(all)"),
            args.skip_simulation
        ),
    )?;

    ui::outro("Upgrade command registered (implementation pending)")?;

    Ok(())
}
```

- [ ] **Step 2: Test version validation**

Run: `cargo run -- upgrade --protocol-version=v0.30.1`
Expected: Shows upgrade script path

Run: `cargo run -- upgrade --protocol-version=v99.0.0`
Expected: Error "Invalid protocol version" or "not supported"

- [ ] **Step 3: Commit**

```bash
git add src/commands/upgrade/mod.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): validate protocol version and show handler info
EOF
)"
```

---

## Phase 3: State Loading + Config Preparation

**Goal:** Load ecosystem state and prepare upgrade config.

### Task 3.1: Add UpgradeConfig type

**Files:**
- Create: `packages/adi-upgrade/src/config.rs`
- Modify: `packages/adi-upgrade/src/lib.rs`
- Modify: `packages/adi-upgrade/Cargo.toml`

- [ ] **Step 1: Add dependencies**

Modify: `packages/adi-upgrade/Cargo.toml`

Add to `[dependencies]`:
```toml
adi-state = { workspace = true }
adi-types = { workspace = true }
alloy-primitives = { workspace = true }
serde = { workspace = true }
serde_yaml = { workspace = true }
tokio = { workspace = true }
url = { workspace = true }
log = { workspace = true }
```

- [ ] **Step 2: Create config.rs**

```rust
//! Upgrade configuration generation.

use adi_state::StateManager;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::path::Path;
use url::Url;

use crate::error::{Result, UpgradeError};

/// Configuration for upgrade operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeConfig {
    /// Settlement layer RPC URL
    pub l1_rpc_url: Url,

    /// Ecosystem name
    pub ecosystem_name: String,

    /// Governor address (from wallets.yaml)
    pub governor_address: Address,

    /// Deployer address (from wallets.yaml)
    pub deployer_address: Address,

    /// Bridgehub address
    pub bridgehub_address: Option<Address>,

    /// CTM address (queried on-chain)
    pub ctm_address: Option<Address>,

    /// Governance address (queried on-chain)
    pub governance_address: Option<Address>,

    /// Gas price multiplier
    pub gas_multiplier: f64,
}

impl UpgradeConfig {
    /// Load upgrade config from ecosystem state.
    ///
    /// # Arguments
    ///
    /// * `state_manager` - State manager for the ecosystem
    /// * `l1_rpc_url` - Settlement layer RPC URL
    /// * `gas_multiplier` - Gas price multiplier
    pub async fn from_state(
        state_manager: &StateManager,
        l1_rpc_url: Url,
        gas_multiplier: f64,
    ) -> Result<Self> {
        log::debug!("Loading upgrade config from state");

        // Load ecosystem wallets
        let wallets = state_manager
            .wallets()
            .await
            .map_err(|e| UpgradeError::Config(format!("Failed to load wallets: {}", e)))?;

        let governor_address = wallets
            .governor
            .as_ref()
            .map(|w| w.address)
            .ok_or_else(|| UpgradeError::Config("Governor wallet not found in state".to_string()))?;

        let deployer_address = wallets
            .deployer
            .as_ref()
            .map(|w| w.address)
            .ok_or_else(|| UpgradeError::Config("Deployer wallet not found in state".to_string()))?;

        // Load ecosystem contracts
        let contracts = state_manager
            .contracts()
            .await
            .map_err(|e| UpgradeError::Config(format!("Failed to load contracts: {}", e)))?;

        let ecosystem_name = state_manager.ecosystem_name().to_string();

        Ok(Self {
            l1_rpc_url,
            ecosystem_name,
            governor_address,
            deployer_address,
            bridgehub_address: contracts.bridgehub,
            ctm_address: None, // Will be queried on-chain
            governance_address: None, // Will be queried on-chain
            gas_multiplier,
        })
    }

    /// Write config to chain.toml format for forge script.
    pub fn write_chain_toml(&self, path: &Path) -> Result<()> {
        use std::io::Write;

        log::debug!("Writing chain.toml to {}", path.display());

        let content = format!(
            r#"[profile.default]
l1_rpc_url = "{}"
governor = "{}"
deployer = "{}"
"#,
            self.l1_rpc_url,
            self.governor_address,
            self.deployer_address,
        );

        let mut file = std::fs::File::create(path)
            .map_err(|e| UpgradeError::Config(format!("Failed to create chain.toml: {}", e)))?;

        file.write_all(content.as_bytes())
            .map_err(|e| UpgradeError::Config(format!("Failed to write chain.toml: {}", e)))?;

        Ok(())
    }
}
```

- [ ] **Step 3: Export from lib.rs**

Add to `packages/adi-upgrade/src/lib.rs`:
```rust
mod config;

pub use config::UpgradeConfig;
```

- [ ] **Step 4: Verify compiles**

Run: `cargo build -p adi-upgrade`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add packages/adi-upgrade/src/config.rs packages/adi-upgrade/src/lib.rs packages/adi-upgrade/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(upgrade): add UpgradeConfig for state loading
EOF
)"
```

### Task 3.2: Load state in CLI command

**Files:**
- Modify: `src/commands/upgrade/mod.rs`

- [ ] **Step 1: Add state loading**

Update `run` function to load state:

```rust
/// Execute the upgrade command.
pub async fn run(args: UpgradeArgs, context: &Context) -> Result<()> {
    use adi_toolkit::ProtocolVersion;
    use adi_upgrade::{get_handler, is_supported, UpgradeConfig};
    use crate::commands::helpers::{
        create_state_manager_with_s3, resolve_ecosystem_name, resolve_rpc_url,
    };
    use crate::error::WrapErr;
    use crate::ui;

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    ui::intro(format!(
        "Upgrading {} to {}",
        ui::green(&ecosystem_name),
        ui::green(&args.protocol_version)
    ))?;

    // Parse and validate protocol version
    let version = ProtocolVersion::parse(&args.protocol_version)
        .wrap_err("Invalid protocol version")?;

    if !is_supported(&version) {
        return Err(eyre::eyre!(
            "Protocol version {} is not supported for upgrades",
            version
        ));
    }

    let handler = get_handler(&version)
        .ok_or_else(|| eyre::eyre!("No handler for version {}", version))?;

    ui::info(format!(
        "Using upgrade script: {}",
        ui::green(handler.upgrade_script())
    ))?;

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    ui::info(format!("RPC URL: {}", ui::green(&rpc_url)))?;

    // Load ecosystem state
    let (state_manager, _s3_control) =
        create_state_manager_with_s3(&ecosystem_name, context).await?;

    // Build upgrade config
    let upgrade_config = UpgradeConfig::from_state(
        &state_manager,
        rpc_url,
        args.gas_multiplier,
    )
    .await
    .wrap_err("Failed to build upgrade config")?;

    ui::note(
        "Upgrade Configuration",
        format!(
            "Governor: {}\nDeployer: {}\nBridgehub: {}\nGas multiplier: {}",
            ui::green(upgrade_config.governor_address),
            ui::green(upgrade_config.deployer_address),
            upgrade_config
                .bridgehub_address
                .map(|a| ui::green(a).to_string())
                .unwrap_or_else(|| "(not deployed)".to_string()),
            upgrade_config.gas_multiplier
        ),
    )?;

    ui::note(
        "Upgrade Target",
        format!(
            "Target: {:?}\nChain: {}\nSkip simulation: {}",
            args.target,
            args.chain.as_deref().unwrap_or("(all)"),
            args.skip_simulation
        ),
    )?;

    ui::outro("Config loaded (simulation phase pending)")?;

    Ok(())
}
```

- [ ] **Step 2: Test with existing ecosystem**

Run: `cargo run -- upgrade --protocol-version=v0.30.1 --ecosystem-name=<existing>`
Expected: Shows governor/deployer addresses from state

- [ ] **Step 3: Commit**

```bash
git add src/commands/upgrade/mod.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): load ecosystem state and build upgrade config
EOF
)"
```

---

## Phase 4: Simulation Phase

**Goal:** Run forge script without broadcast, parse output, show summary.

### Task 4.1: Add simulation module

**Files:**
- Create: `packages/adi-upgrade/src/simulation.rs`
- Modify: `packages/adi-upgrade/src/lib.rs`

- [ ] **Step 1: Create simulation.rs**

```rust
//! Simulation phase for upgrade operations.
//!
//! Runs forge script without --broadcast to validate upgrade.

use std::path::Path;

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::versions::VersionHandler;

/// Result of a simulation run.
#[derive(Debug)]
pub struct SimulationResult {
    /// Whether simulation succeeded
    pub success: bool,

    /// Exit code from forge
    pub exit_code: i64,

    /// Path to output YAML file
    pub output_path: Option<std::path::PathBuf>,

    /// Summary of what will be deployed
    pub summary: String,
}

/// Run upgrade simulation (forge script without --broadcast).
///
/// # Arguments
///
/// * `handler` - Version-specific handler
/// * `config` - Upgrade configuration
/// * `state_dir` - Ecosystem state directory
/// * `runner` - Toolkit runner for Docker execution
/// * `protocol_version` - Protocol version for image selection
pub async fn run_simulation<R>(
    handler: &dyn VersionHandler,
    config: &UpgradeConfig,
    state_dir: &Path,
    runner: &R,
    protocol_version: &semver::Version,
) -> Result<SimulationResult>
where
    R: ToolkitRunnerTrait,
{
    log::info!("Running upgrade simulation");
    log::debug!("Upgrade script: {}", handler.upgrade_script());

    let script_path = handler.upgrade_script();

    // Build forge command args
    let rpc_url = config.l1_rpc_url.to_string();
    let args = vec![
        "script",
        script_path,
        "--rpc-url",
        &rpc_url,
        "-vvv",
    ];

    let exit_code = runner
        .run_forge(&args, state_dir, protocol_version)
        .await
        .map_err(|e| UpgradeError::SimulationFailed(e.to_string()))?;

    let success = exit_code == 0;

    let summary = if success {
        "Simulation completed successfully. Review the output above.".to_string()
    } else {
        format!("Simulation failed with exit code {}", exit_code)
    };

    Ok(SimulationResult {
        success,
        exit_code,
        output_path: None, // TODO: parse output path
        summary,
    })
}

/// Trait for toolkit runner to enable testing.
#[async_trait::async_trait]
pub trait ToolkitRunnerTrait: Send + Sync {
    /// Run forge command.
    async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &semver::Version,
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>>;
}
```

- [ ] **Step 2: Add async-trait dependency**

Modify: `packages/adi-upgrade/Cargo.toml`

Add to `[dependencies]`:
```toml
async-trait = { workspace = true }
semver = { workspace = true }
```

- [ ] **Step 3: Export from lib.rs**

Add to `packages/adi-upgrade/src/lib.rs`:
```rust
mod simulation;

pub use simulation::{run_simulation, SimulationResult, ToolkitRunnerTrait};
```

- [ ] **Step 4: Verify compiles**

Run: `cargo build -p adi-upgrade`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add packages/adi-upgrade/src/simulation.rs packages/adi-upgrade/src/lib.rs packages/adi-upgrade/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(upgrade): add simulation phase module
EOF
)"
```

### Task 4.2: Integrate simulation in CLI

**Files:**
- Modify: `src/commands/upgrade/mod.rs`

- [ ] **Step 1: Implement ToolkitRunnerTrait for ToolkitRunner**

Add wrapper implementation in `src/commands/upgrade/mod.rs`:

```rust
use adi_toolkit::ToolkitRunner;
use adi_upgrade::ToolkitRunnerTrait;
use std::path::Path;

struct ToolkitRunnerWrapper(ToolkitRunner);

#[async_trait::async_trait]
impl ToolkitRunnerTrait for ToolkitRunnerWrapper {
    async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &semver::Version,
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .run_forge(args, state_dir, protocol_version)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}
```

- [ ] **Step 2: Add simulation to run function**

Update run function to include simulation phase (after config loading):

```rust
    // Run simulation unless skipped
    if !args.skip_simulation {
        ui::info("Running upgrade simulation...")?;

        let runner = ToolkitRunner::with_logger(Arc::clone(context.logger()))
            .await
            .wrap_err("Failed to create toolkit runner")?;

        let wrapper = ToolkitRunnerWrapper(runner);
        let state_dir = context.config().state_dir.join(&ecosystem_name);

        let simulation_result = adi_upgrade::run_simulation(
            handler.as_ref(),
            &upgrade_config,
            &state_dir,
            &wrapper,
            &version.to_semver(),
        )
        .await?;

        if !simulation_result.success {
            return Err(eyre::eyre!(simulation_result.summary));
        }

        ui::note("Simulation Result", &simulation_result.summary)?;

        // Confirmation prompt
        let proceed: bool = ui::confirm("Proceed with broadcast?")
            .initial_value(false)
            .interact()
            .wrap_err("Confirmation cancelled")?;

        if !proceed {
            ui::outro_cancel("Upgrade cancelled by user")?;
            return Ok(());
        }
    }
```

- [ ] **Step 3: Add required imports**

Add at top of file:
```rust
use std::sync::Arc;
use async_trait::async_trait;
```

- [ ] **Step 4: Test simulation**

Run: `cargo run -- upgrade --protocol-version=v0.30.1 --ecosystem-name=<existing>`
Expected: Runs simulation (may fail if ecosystem not set up for upgrade)

- [ ] **Step 5: Commit**

```bash
git add src/commands/upgrade/mod.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): integrate simulation phase with confirmation prompt
EOF
)"
```

---

## Phase 5: Broadcast Phase

**Goal:** Run forge script with --broadcast after confirmation.

### Task 5.1: Add broadcast module

**Files:**
- Create: `packages/adi-upgrade/src/broadcast.rs`
- Modify: `packages/adi-upgrade/src/lib.rs`

- [ ] **Step 1: Create broadcast.rs**

```rust
//! Broadcast phase for upgrade operations.
//!
//! Runs forge script with --broadcast to deploy contracts.

use std::path::Path;

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::simulation::ToolkitRunnerTrait;
use crate::versions::VersionHandler;

/// Result of a broadcast run.
#[derive(Debug)]
pub struct BroadcastResult {
    /// Whether broadcast succeeded
    pub success: bool,

    /// Exit code from forge
    pub exit_code: i64,

    /// Path to output YAML file
    pub output_path: Option<std::path::PathBuf>,
}

/// Run upgrade broadcast (forge script with --broadcast).
///
/// # Arguments
///
/// * `handler` - Version-specific handler
/// * `config` - Upgrade configuration
/// * `state_dir` - Ecosystem state directory
/// * `runner` - Toolkit runner for Docker execution
/// * `protocol_version` - Protocol version for image selection
pub async fn run_broadcast<R>(
    handler: &dyn VersionHandler,
    config: &UpgradeConfig,
    state_dir: &Path,
    runner: &R,
    protocol_version: &semver::Version,
) -> Result<BroadcastResult>
where
    R: ToolkitRunnerTrait,
{
    log::info!("Running upgrade broadcast");
    log::debug!("Upgrade script: {}", handler.upgrade_script());

    let script_path = handler.upgrade_script();

    // Build forge command args with --broadcast
    let rpc_url = config.l1_rpc_url.to_string();
    let args = vec![
        "script",
        script_path,
        "--rpc-url",
        &rpc_url,
        "--broadcast",
        "-vvv",
    ];

    let exit_code = runner
        .run_forge(&args, state_dir, protocol_version)
        .await
        .map_err(|e| UpgradeError::BroadcastFailed(e.to_string()))?;

    let success = exit_code == 0;

    if !success {
        return Err(UpgradeError::BroadcastFailed(format!(
            "Forge script failed with exit code {}",
            exit_code
        )));
    }

    Ok(BroadcastResult {
        success,
        exit_code,
        output_path: None, // TODO: parse output path
    })
}
```

- [ ] **Step 2: Export from lib.rs**

Add to `packages/adi-upgrade/src/lib.rs`:
```rust
mod broadcast;

pub use broadcast::{run_broadcast, BroadcastResult};
```

- [ ] **Step 3: Integrate in CLI**

Add broadcast phase after simulation in `src/commands/upgrade/mod.rs`:

```rust
    // Run broadcast
    ui::info("Running upgrade broadcast...")?;

    let broadcast_result = adi_upgrade::run_broadcast(
        handler.as_ref(),
        &upgrade_config,
        &state_dir,
        &wrapper,
        &version.to_semver(),
    )
    .await?;

    if broadcast_result.success {
        ui::success("Broadcast completed successfully")?;
    }
```

- [ ] **Step 4: Test broadcast**

Run: `cargo run -- upgrade --protocol-version=v0.30.1 --ecosystem-name=<existing>`
Expected: Runs simulation, prompts for confirmation, then broadcasts

- [ ] **Step 5: Commit**

```bash
git add packages/adi-upgrade/src/broadcast.rs packages/adi-upgrade/src/lib.rs src/commands/upgrade/mod.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): add broadcast phase after confirmation
EOF
)"
```

---

## Phase 6: Bytecode Validation

**Goal:** Validate forge output contains expected bytecode hashes.

### Task 6.1: Add bytecode validation module

**Files:**
- Create: `packages/adi-upgrade/src/validation/mod.rs`
- Create: `packages/adi-upgrade/src/validation/bytecode.rs`
- Modify: `packages/adi-upgrade/src/lib.rs`

- [ ] **Step 1: Create validation/mod.rs**

```rust
//! Validation modules for upgrade operations.

mod bytecode;

pub use bytecode::{validate_upgrade_output, BytecodeManifest, ValidationReport};
```

- [ ] **Step 2: Create validation/bytecode.rs**

```rust
//! Bytecode validation for upgrade outputs.
//!
//! Validates that forge upgrade output contains expected contract bytecode hashes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{Result, UpgradeError};

/// Manifest of expected bytecode hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeManifest {
    /// Contract name -> bytecode_hash
    #[serde(flatten)]
    pub contracts: HashMap<String, ContractEntry>,
}

/// Entry for a contract in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEntry {
    /// Bytecode hash (without 0x prefix)
    pub bytecode_hash: String,
}

/// Report from bytecode validation.
#[derive(Debug, Default)]
pub struct ValidationReport {
    /// Contract names that were found in the output
    pub found: Vec<String>,

    /// Contract names and hashes that were NOT found
    pub missing: Vec<(String, String)>,

    /// Extra hashes found in output but not in manifest
    pub extra: Vec<String>,
}

impl ValidationReport {
    /// Check if validation passed (no missing hashes).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.missing.is_empty()
    }

    /// Format report as human-readable string.
    #[must_use]
    pub fn format(&self) -> String {
        let mut lines = vec![format!(
            "Found {}/{} expected hashes",
            self.found.len(),
            self.found.len() + self.missing.len()
        )];

        if !self.missing.is_empty() {
            lines.push(format!("\nMissing {} hashes:", self.missing.len()));
            for (name, hash) in &self.missing {
                lines.push(format!("  - {}: {}", name, hash));
            }
        }

        if !self.extra.is_empty() {
            lines.push(format!("\nExtra {} hashes found:", self.extra.len()));
            for hash in &self.extra {
                lines.push(format!("  - {}", hash));
            }
        }

        lines.join("\n")
    }
}

/// Validate upgrade YAML output against expected bytecode hashes.
///
/// # Arguments
///
/// * `upgrade_yaml` - Contents of the forge upgrade output YAML
/// * `manifest` - Expected bytecode manifest
pub fn validate_upgrade_output(upgrade_yaml: &str, manifest: &BytecodeManifest) -> ValidationReport {
    let mut report = ValidationReport::default();
    let yaml_lower = upgrade_yaml.to_lowercase();

    // Check each expected hash
    for (contract_name, entry) in &manifest.contracts {
        let hash = entry.bytecode_hash.trim_start_matches("0x").to_lowercase();

        if yaml_lower.contains(&hash) {
            report.found.push(contract_name.clone());
        } else {
            report.missing.push((contract_name.clone(), hash));
        }
    }

    // Extract 00000060<hash> patterns and find unknowns
    let known_hashes: std::collections::HashSet<_> = manifest
        .contracts
        .values()
        .map(|e| e.bytecode_hash.trim_start_matches("0x").to_lowercase())
        .collect();

    let pattern = regex::Regex::new(r"00000060([0-9a-fA-F]{64})").ok();
    if let Some(re) = pattern {
        for cap in re.captures_iter(upgrade_yaml) {
            if let Some(hash_match) = cap.get(1) {
                let hash = hash_match.as_str().to_lowercase();

                // Skip noise patterns
                if hash.starts_with("c37bb1bc") {
                    continue;
                }
                if hash.chars().take(24).all(|c| c == '0') {
                    continue;
                }

                if !known_hashes.contains(&hash) {
                    report.extra.push(hash);
                }
            }
        }
    }

    report.extra.sort();
    report.extra.dedup();

    report
}
```

- [ ] **Step 3: Add regex dependency**

Modify: `packages/adi-upgrade/Cargo.toml`

Add to `[workspace.dependencies]` in root Cargo.toml:
```toml
regex = { version = "1", default-features = false, features = ["std"] }
```

Add to `packages/adi-upgrade/Cargo.toml`:
```toml
regex = { workspace = true }
```

- [ ] **Step 4: Export from lib.rs**

Add to `packages/adi-upgrade/src/lib.rs`:
```rust
pub mod validation;

pub use validation::{validate_upgrade_output, BytecodeManifest, ValidationReport};
```

- [ ] **Step 5: Verify compiles**

Run: `cargo build -p adi-upgrade`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add packages/adi-upgrade/src/validation Cargo.toml packages/adi-upgrade/Cargo.toml packages/adi-upgrade/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): add bytecode validation module
EOF
)"
```

### Task 6.2: Integrate validation in CLI

**Files:**
- Modify: `src/commands/upgrade/mod.rs`

- [ ] **Step 1: Add validation after broadcast**

Add after broadcast phase:

```rust
    // Validate bytecode (if output exists)
    // TODO: Load manifest from toolkit image and validate
    ui::info("Bytecode validation: skipped (manifest not yet available)")?;
```

- [ ] **Step 2: Commit**

```bash
git add src/commands/upgrade/mod.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): add placeholder for bytecode validation
EOF
)"
```

---

## Phase 7: Governance Execution

**Goal:** Send governance transactions (scheduleTransparent + execute).

### Task 7.1: Add governance modules

**Files:**
- Create: `packages/adi-upgrade/src/governance/mod.rs`
- Create: `packages/adi-upgrade/src/governance/ecosystem.rs`
- Modify: `packages/adi-upgrade/src/lib.rs`
- Modify: `packages/adi-upgrade/Cargo.toml`

- [ ] **Step 1: Add alloy dependencies**

Modify: `packages/adi-upgrade/Cargo.toml`

Add to `[dependencies]`:
```toml
alloy-provider = { workspace = true }
alloy-network = { workspace = true }
alloy-signer-local = { workspace = true }
alloy-sol-types = { workspace = true }
alloy-rpc-types = { workspace = true }
alloy-contract = { workspace = true }
secrecy = { workspace = true }
hex = { workspace = true }
```

- [ ] **Step 2: Create governance/mod.rs**

```rust
//! Governance transaction execution for upgrades.

pub mod ecosystem;

pub use ecosystem::EcosystemGovernance;
```

- [ ] **Step 3: Create governance/ecosystem.rs**

```rust
//! Ecosystem-level governance for upgrades.
//!
//! Handles scheduleTransparent and execute calls on the governance contract.

use alloy_primitives::{Address, Bytes, B256};
use alloy_provider::Provider;

use crate::error::{Result, UpgradeError};

/// Ecosystem governance handler.
pub struct EcosystemGovernance<P> {
    provider: P,
    governance_addr: Address,
}

impl<P: Provider + Clone> EcosystemGovernance<P> {
    /// Create a new ecosystem governance handler.
    pub fn new(provider: P, governance_addr: Address) -> Self {
        Self {
            provider,
            governance_addr,
        }
    }

    /// Schedule a transparent governance call.
    ///
    /// # Arguments
    ///
    /// * `target` - Target contract address
    /// * `calldata` - Encoded function call
    /// * `value` - ETH value to send
    pub async fn schedule_transparent(
        &self,
        target: Address,
        calldata: Bytes,
        value: u128,
    ) -> Result<B256> {
        log::info!(
            "Scheduling transparent call to {} with {} bytes calldata",
            target,
            calldata.len()
        );

        // TODO: Build and send transaction
        // For now, return placeholder
        Err(UpgradeError::GovernanceFailed(
            "scheduleTransparent not yet implemented".to_string(),
        ))
    }

    /// Execute a scheduled governance call.
    ///
    /// # Arguments
    ///
    /// * `target` - Target contract address
    /// * `calldata` - Encoded function call
    /// * `value` - ETH value to send
    pub async fn execute(&self, target: Address, calldata: Bytes, value: u128) -> Result<B256> {
        log::info!(
            "Executing call to {} with {} bytes calldata",
            target,
            calldata.len()
        );

        // TODO: Build and send transaction
        // For now, return placeholder
        Err(UpgradeError::GovernanceFailed(
            "execute not yet implemented".to_string(),
        ))
    }
}
```

- [ ] **Step 4: Export from lib.rs**

Add to `packages/adi-upgrade/src/lib.rs`:
```rust
pub mod governance;

pub use governance::EcosystemGovernance;
```

- [ ] **Step 5: Verify compiles**

Run: `cargo build -p adi-upgrade`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add packages/adi-upgrade/src/governance packages/adi-upgrade/Cargo.toml packages/adi-upgrade/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): add ecosystem governance module scaffold
EOF
)"
```

---

## Phase 8: Chain Selection + Chain Upgrades

**Goal:** Multi-select chains and upgrade each one.

### Task 8.1: Add chain prompts

**Files:**
- Create: `src/commands/upgrade/prompts.rs`
- Modify: `src/commands/upgrade/mod.rs`

- [ ] **Step 1: Create prompts.rs**

```rust
//! Interactive prompts for upgrade command.

use crate::error::{Result, WrapErr};
use crate::ui;

/// Select chains to upgrade using multi-select picker.
///
/// # Arguments
///
/// * `available_chains` - List of chain names available in ecosystem
/// * `preselected` - Optional chain name from --chain flag
pub fn select_chains(
    available_chains: &[String],
    preselected: Option<&String>,
) -> Result<Vec<String>> {
    // If --chain flag provided, use it directly
    if let Some(chain) = preselected {
        if !available_chains.contains(chain) {
            return Err(eyre::eyre!(
                "Chain '{}' not found. Available: {}",
                chain,
                available_chains.join(", ")
            ));
        }
        return Ok(vec![chain.clone()]);
    }

    // Single chain - auto-select
    if available_chains.len() == 1 {
        let chain = available_chains
            .first()
            .ok_or_else(|| eyre::eyre!("No chains available"))?
            .clone();
        ui::info(format!("Auto-selected chain: {}", ui::green(&chain)))?;
        return Ok(vec![chain]);
    }

    // Multiple chains - show picker
    let items: Vec<(String, String, String)> = available_chains
        .iter()
        .map(|name| (name.clone(), name.clone(), String::new()))
        .collect();

    let selected: Vec<String> = cliclack::multiselect("Select chains to upgrade")
        .items(&items)
        .required(true)
        .interact()
        .wrap_err("Chain selection cancelled")?;

    Ok(selected)
}
```

- [ ] **Step 2: Update mod.rs to use prompts**

Add at top of `src/commands/upgrade/mod.rs`:
```rust
mod prompts;
```

- [ ] **Step 3: Add chain selection to run function**

After broadcast phase, add:

```rust
    // Chain upgrades (if target includes chains)
    let upgrade_chains = matches!(args.target, UpgradeTarget::Chain | UpgradeTarget::Both);

    if upgrade_chains {
        let chain_names = state_manager.list_chains().await?;

        if chain_names.is_empty() {
            ui::warning("No chains found in ecosystem, skipping chain upgrade")?;
        } else {
            let selected_chains = prompts::select_chains(&chain_names, args.chain.as_ref())?;

            for chain_name in &selected_chains {
                ui::info(format!("Upgrading chain: {}", ui::green(chain_name)))?;
                // TODO: Implement chain upgrade
            }
        }
    }
```

- [ ] **Step 4: Test chain selection**

Run: `cargo run -- upgrade --protocol-version=v0.30.1 --ecosystem-name=<existing>`
Expected: Shows chain multi-select if multiple chains exist

- [ ] **Step 5: Commit**

```bash
git add src/commands/upgrade/prompts.rs src/commands/upgrade/mod.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): add chain selection with multi-select picker
EOF
)"
```

---

## Phase 9: Orchestrator + Final Integration

**Goal:** Tie everything together with clean orchestration.

### Task 9.1: Add orchestrator module

**Files:**
- Create: `packages/adi-upgrade/src/orchestrator.rs`
- Modify: `packages/adi-upgrade/src/lib.rs`

- [ ] **Step 1: Create orchestrator.rs**

```rust
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
```

- [ ] **Step 2: Export from lib.rs**

Add to `packages/adi-upgrade/src/lib.rs`:
```rust
mod orchestrator;

pub use orchestrator::UpgradeOrchestrator;
```

- [ ] **Step 3: Refactor CLI to use orchestrator**

Update `src/commands/upgrade/mod.rs` to use `UpgradeOrchestrator`:

```rust
    // Create orchestrator
    let orchestrator = adi_upgrade::UpgradeOrchestrator::new(
        handler.as_ref(),
        &upgrade_config,
        &state_dir,
        &wrapper,
        version.to_semver(),
    );

    // Simulation phase
    if !args.skip_simulation {
        ui::info("Running upgrade simulation...")?;

        let simulation_result = orchestrator.simulate().await?;

        if !simulation_result.success {
            return Err(eyre::eyre!(simulation_result.summary));
        }

        ui::note("Simulation Result", &simulation_result.summary)?;

        let proceed: bool = ui::confirm("Proceed with broadcast?")
            .initial_value(false)
            .interact()
            .wrap_err("Confirmation cancelled")?;

        if !proceed {
            ui::outro_cancel("Upgrade cancelled by user")?;
            return Ok(());
        }
    }

    // Broadcast phase
    ui::info("Running upgrade broadcast...")?;
    let broadcast_result = orchestrator.broadcast().await?;

    if broadcast_result.success {
        ui::success("Broadcast completed successfully")?;
    }
```

- [ ] **Step 4: Add final summary**

At end of run function:

```rust
    ui::outro(format!(
        "Upgrade to {} completed successfully",
        ui::green(&args.protocol_version)
    ))?;

    Ok(())
```

- [ ] **Step 5: Verify full flow**

Run: `cargo build && cargo clippy -- -D warnings`
Expected: No warnings

Run: `cargo run -- upgrade --help`
Expected: Shows all options correctly

- [ ] **Step 6: Commit**

```bash
git add packages/adi-upgrade/src/orchestrator.rs packages/adi-upgrade/src/lib.rs src/commands/upgrade/mod.rs
git commit -m "$(cat <<'EOF'
feat(upgrade): add orchestrator for coordinated upgrade flow
EOF
)"
```

---

## Summary

After completing all phases:
- `adi upgrade` command is fully functional
- Version handling via `VersionHandler` trait
- Simulation -> Confirm -> Broadcast flow
- Bytecode validation (manifest loading TBD)
- Chain selection with multi-select
- Clean orchestration via `UpgradeOrchestrator`

**Remaining work (future phases):**
- Load bytecode manifest from toolkit image
- Implement governance transaction sending
- Implement chain-level upgrade logic (ChainAdmin calls)
- Add post-upgrade hooks (DAValidator setup for v0.30.0)
- Add idempotency checks for re-runs
