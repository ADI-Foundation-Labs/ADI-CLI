# Upgrade Command Design

**Date**: 2026-03-17
**Status**: Draft
**Author**: Claude + User

## Overview

Add an `adi upgrade` command to upgrade ZkSync ecosystem and chain contracts to a new protocol version. This replaces the existing bash scripts in `ecosystem-deployment/upgrade/` with a Rust implementation following existing CLI patterns.

## Command Interface

```
adi upgrade [OPTIONS]

OPTIONS:
  --protocol-version <VERSION>   Target version (e.g., v30.0.2) [required]
  --target <TARGET>              Upgrade target: ecosystem, chain, or both [default: both]
  --chain <NAME>                 Chain name (bypasses multi-select picker)
  --skip-simulation              Skip simulation, go straight to broadcast
  --rpc-url <URL>                Settlement layer RPC URL override
  --gas-multiplier <FLOAT>       Gas price multiplier [default: 1.2]
```

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Execution model | Docker container | Consistent with existing CLI (`adi deploy`, `adi init`) |
| Command structure | Single command with flags | `--target` and `--skip-simulation` provide control without separate subcommands |
| Version-specific logic | Rust code with runtime detection | `VersionHandler` trait with match on `ProtocolVersion` variants |
| Private keys | Reuse from state | Governor/Deployer wallets already in `wallets.yaml` |
| Upgrade target | `--target=ecosystem\|chain\|both` | Default is both, flexible for partial upgrades |
| Simulation flow | Two-phase with confirmation | Simulate -> cliclack confirm -> broadcast |
| Bytecode validation | Always, in Rust | Validates forge output contains expected bytecode hashes |
| Error handling | Stateless, on-chain idempotency | Re-run checks on-chain state to skip completed steps |
| Multi-chain | cliclack multi-select | `--chain=<name>` flag bypasses picker |
| Gas handling | Existing pattern | Auto-estimate with `--gas-multiplier` |
| Logging | Existing CLI style | Debug everywhere, info for checkpoints, cliclack for summaries |

## Package Architecture

### New Package: `adi-upgrade`

```
packages/adi-upgrade/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Public API exports
    ├── error.rs                # UpgradeError type
    ├── config.rs               # UpgradeConfig generation from state
    ├── orchestrator.rs         # Main upgrade orchestration logic
    ├── simulation.rs           # Simulate phase, parse forge output
    ├── broadcast.rs            # Broadcast phase execution
    ├── validation/
    │   ├── mod.rs
    │   └── bytecode.rs         # Bytecode validation (hash matching)
    ├── governance/
    │   ├── mod.rs
    │   ├── ecosystem.rs        # scheduleTransparent + execute
    │   └── chain.rs            # ChainAdmin calls, post-hooks
    └── versions/
        ├── mod.rs              # VersionHandler trait + dispatch
        ├── handlers/
        │   ├── mod.rs
        │   ├── v0_30_0.rs
        │   ├── v0_30_x.rs
        │   └── v0_31.rs
        └── hooks.rs            # PostUpgradeHook enum
```

### Dependencies

```toml
[dependencies]
adi-toolkit = { workspace = true }      # Docker container execution
adi-ecosystem = { workspace = true }    # Ownership/verification modules
adi-state = { workspace = true }        # Read ecosystem/chain state
adi-types = { workspace = true }        # Shared types
alloy-primitives = { workspace = true }
alloy-provider = { workspace = true }
alloy-sol-types = { workspace = true }
```

### CLI Module

```
src/commands/upgrade/
├── mod.rs          # UpgradeArgs, run() entry point
└── prompts.rs      # cliclack interactions
```

## Upgrade Workflow

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. LOAD STATE                                                   │
│    - Read ecosystem config from ~/.adi_cli/state/<ecosystem>/   │
│    - Load wallets (Governor, Deployer)                          │
│    - Get version handler for target ProtocolVersion             │
└─────────────────────────────┬───────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. PREPARE CONFIG                                               │
│    - Query on-chain: CTM, Governance, current verifier          │
│    - Extract values from state (contracts.yaml, genesis.json)   │
│    - Generate chain.toml for forge script                       │
└─────────────────────────────┬───────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. SIMULATE (unless --skip-simulation)                          │
│    - Run forge script via toolkit container (no --broadcast)    │
│    - Parse output: deployed addresses, calldata                 │
│    - Display summary via cliclack::note                         │
│    - Prompt: "Proceed with broadcast?" (cliclack::confirm)      │
└─────────────────────────────┬───────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. BROADCAST                                                    │
│    - Run forge script with --broadcast                          │
│    - Parse deployed contract addresses                          │
└─────────────────────────────┬───────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. VALIDATE BYTECODE                                            │
│    - Load bytecode manifest from toolkit image                  │
│    - Search for expected hashes in forge YAML output            │
│    - Report found/missing/extra hashes                          │
│    - Abort if critical hashes missing                           │
└─────────────────────────────┬───────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 6. ECOSYSTEM GOVERNANCE (if --target includes ecosystem)        │
│    - Build scheduleTransparent calldata                         │
│    - Send tx with Governor wallet                               │
│    - Build execute calldata                                     │
│    - Send tx with Governor wallet                               │
└─────────────────────────────┬───────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 7. CHAIN UPGRADE (if --target includes chain)                   │
│    - Show multi-select picker (or use --chain flag)             │
│    - For each chain:                                            │
│      - Generate chain upgrade calldata (via zkstack)            │
│      - Send schedule + execute txs via ChainAdmin               │
│      - Set upgrade timestamp                                    │
│      - Run version-specific post-hooks (e.g., DAValidator)      │
└─────────────────────────────┬───────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 8. VERIFY & SUMMARY                                             │
│    - Query on-chain protocol versions (CTM vs Diamond)          │
│    - Display success summary with cliclack::outro               │
│    - Save upgrade artifacts to state directory                  │
└─────────────────────────────────────────────────────────────────┘
```

## Version Handling

```rust
pub trait VersionHandler: Send + Sync {
    /// Forge script path for this version
    fn upgrade_script(&self) -> &str;

    /// Post-upgrade hooks (e.g., DAValidator setup)
    fn post_upgrade_hooks(&self) -> Vec<PostUpgradeHook>;

    /// Any version-specific config adjustments
    fn adjust_config(&self, config: &mut UpgradeConfig);
}

pub fn get_handler(version: &ProtocolVersion) -> Option<Box<dyn VersionHandler>> {
    match version {
        ProtocolVersion::V0_30_0 => Some(Box::new(V0_30_0Handler)),
        ProtocolVersion::V0_30_1 => Some(Box::new(V0_30_xHandler)),
        ProtocolVersion::V0_30_2 => Some(Box::new(V0_30_xHandler)),
        ProtocolVersion::V0_31_0 => Some(Box::new(V0_31_Handler)),
        _ => None,
    }
}
```

## Bytecode Validation

Validates that forge upgrade output contains expected contract bytecode hashes before governance execution.

```rust
pub struct BytecodeManifest {
    /// Contract name -> bytecode_hash
    pub contracts: HashMap<String, String>,
}

pub struct ValidationReport {
    pub found: Vec<String>,              // Contract names found
    pub missing: Vec<(String, String)>,  // (name, hash) not found
    pub extra: Vec<String>,              // Hashes in output but not in manifest
}

pub fn validate_upgrade_output(
    upgrade_yaml: &str,
    manifest: &BytecodeManifest,
) -> ValidationReport {
    // Search for each bytecode_hash in YAML content (case-insensitive)
    // Extract 00000060<hash> patterns and report unknowns
    // Filter noise (c37bb1bc prefix, many leading zeros)
}
```

**Manifest source**: Toolkit image includes `/contracts/bytecode-manifest.json`.

## Governance Execution

### Ecosystem Governance

```rust
pub struct EcosystemGovernance {
    provider: Provider,
    governance_addr: Address,  // From Bridgehub.owner()
    governor_wallet: Wallet,   // From state wallets.yaml
}

impl EcosystemGovernance {
    pub async fn schedule_transparent(&self, calls: &EncodedCalls) -> Result<TxHash>;
    pub async fn execute(&self, calls: &EncodedCalls) -> Result<TxHash>;
}
```

### Chain Governance

```rust
pub struct ChainGovernance {
    provider: Provider,
    chain_admin_addr: Address,  // From Diamond.getAdmin()
    governor_wallet: Wallet,
}

impl ChainGovernance {
    pub async fn schedule_upgrade(&self, calldata: Bytes) -> Result<TxHash>;
    pub async fn execute_upgrade(&self, calldata: Bytes) -> Result<TxHash>;
    pub async fn set_upgrade_timestamp(&self, version: &ProtocolVersion) -> Result<TxHash>;
    pub async fn run_post_hooks(&self, hooks: Vec<PostUpgradeHook>) -> Result<()>;
}
```

## CLI Integration

### Args

```rust
#[derive(Args, Debug, Serialize, Deserialize)]
pub struct UpgradeArgs {
    #[arg(long, required = true)]
    pub protocol_version: String,

    #[arg(long, default_value = "both")]
    pub target: UpgradeTarget,

    #[arg(long)]
    pub chain: Option<String>,

    #[arg(long)]
    pub skip_simulation: bool,

    #[arg(long)]
    pub rpc_url: Option<String>,

    #[arg(long, default_value = "1.2")]
    pub gas_multiplier: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
pub enum UpgradeTarget {
    Ecosystem,
    Chain,
    Both,
}
```

### cliclack Usage

| Function | Purpose |
|----------|---------|
| `cliclack::intro()` | "Upgrading ecosystem to vX.Y.Z" |
| `cliclack::spinner()` | During forge execution, on-chain queries |
| `cliclack::note()` | Simulation summary (addresses, calldata) |
| `cliclack::confirm()` | "Proceed with broadcast?" |
| `cliclack::multiselect()` | Chain picker |
| `cliclack::outro()` | Final success summary |

## Idempotency

On re-run after failure, the CLI checks on-chain state:

| Step | Check |
|------|-------|
| Broadcast | Contracts already deployed at expected addresses? |
| Ecosystem governance | Protocol version already matches target? |
| Chain upgrade | Each chain's Diamond protocol version already upgraded? |

## Out of Scope

- Rollback/downgrade functionality
- Parallel chain upgrades (sequential only)
- Custom forge script paths (derived from version handler)
