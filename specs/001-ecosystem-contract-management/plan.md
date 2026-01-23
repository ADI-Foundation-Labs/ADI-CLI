# Implementation Plan: Ecosystem Contract Management CLI

**Branch**: `001-ecosystem-contract-management` | **Date**: 2026-01-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-ecosystem-contract-management/spec.md`

## Summary

SDK-first Rust CLI (`adi-cli`) for managing ZkSync ecosystem smart contracts within Docker containers. The CLI automates ecosystem initialization, contract deployment, chain registration, and upgrade operations by wrapping zkstack CLI and foundry-zksync tooling. Uses abstract state backends (filesystem default) for persistent storage with Docker volume mounts.

## Technical Context

**Language/Version**: Rust (latest stable, edition 2021)
**Primary Dependencies**: Clap 4 (CLI), Tokio (async), eyre (errors), config (YAML), serde (serialization), colored (output)
**Storage**: Filesystem-based key-value state backend (abstract trait for extensibility)
**Testing**: cargo test (unit + integration), contract tests against local Anvil
**Target Platform**: Linux containers (Docker), also macOS for development
**Project Type**: Single project with SDK-first architecture (library crates + thin CLI wrapper)
**Performance Goals**: CLI responsiveness <1s for non-network operations; deployment bound by L1 transaction times
**Constraints**: Must operate within Docker; no dependency installation; strict Clippy lints (no panics, no unwrap, no indexing)
**Scale/Scope**: Single ecosystem with multiple chains; ~9 user stories across 3 priority levels

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### I. Safe Rust Patterns (NON-NEGOTIABLE) - PASS

- [x] No `unwrap()`, `expect()`, `panic!()` - using `eyre::Result` with `wrap_err()`
- [x] No unsafe indexing - using `.get()` and safe slice operations
- [x] No wildcard imports - explicit imports only
- [x] Error context via `wrap_err()` on all fallible operations

**Evidence**: Cargo.toml enforces via clippy lints: `panic = "deny"`, `unwrap_used = "deny"`, `expect_used = "deny"`, `indexing_slicing = "deny"`, `wildcard_imports = "deny"`

### II. Command Pattern Architecture - PASS

- [x] Commands in `src/commands/` as separate modules
- [x] Each command defines enum with `#[derive(Subcommand)]`
- [x] Each implements `async fn run(&self, context: &Context) -> Result<()>`
- [x] Commands registered in `commands/mod.rs` under `Commands` enum
- [x] Shared state via `Context` (config, logger)

**Evidence**: Existing `version.rs` command demonstrates pattern; all new commands will follow same structure.

### III. User Experience First - PASS

- [x] Structured output with colored, formatted console output via Logger
- [x] Error messages include context via `wrap_err()`
- [ ] Progress feedback for long-running operations - NEEDS IMPLEMENTATION
- [x] Exit codes: 0=success, 1=error in main.rs
- [ ] Help text for all commands - NEEDS IMPLEMENTATION per command

**Notes**: Progress indicators needed for deployment/upgrade commands. Help text via Clap derive macros.

### IV. Test-Driven Development - PASS (conditional)

- [ ] Unit tests - TO BE WRITTEN
- [ ] Integration tests - TO BE WRITTEN
- [ ] Contract tests against Anvil - TO BE WRITTEN

**Notes**: Tests will be implemented alongside features. TDD approach per constitution.

### V. ZK Stack Compatibility - PASS

- [x] Version pinning strategy via Docker Bake with commit hashes
- [ ] zkSync OS Server version tracking - TO BE DOCUMENTED
- [ ] era-contracts tag tracking - TO BE DOCUMENTED

**Notes**: FR-004 specifies commit pinning for smart contracts, os-integration, Zk-os, Genesis.json.

### VI. Configuration Hierarchy - PASS

- [x] CLI flags (highest priority) - via Clap
- [x] Environment variables with `ADI_` prefix - via config crate
- [x] Config file `~/.adi_cli/.adi.yml` - via config crate
- [x] Built-in defaults - via config crate

**Evidence**: Existing `config.rs` implements this hierarchy.

### Gate Status: PASS - Proceed to Phase 0

## Project Structure

### Documentation (this feature)

```text
specs/001-ecosystem-contract-management/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (CLI command contracts, not smart contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point, CLI parsing
├── context.rs           # Context struct (config + logger)
├── config.rs            # Configuration loading
├── error.rs             # Error handling (eyre re-exports)
├── log.rs               # Logger with MessageBuilder
├── commands/            # Command implementations
│   ├── mod.rs           # Commands enum registration
│   ├── version.rs       # Version command (existing)
│   ├── doctor.rs        # Dependency verification
│   ├── init/            # Init subcommands
│   │   ├── mod.rs
│   │   ├── ecosystem.rs # Initialize ecosystem
│   │   └── chain.rs     # Initialize chain
│   ├── deploy/          # Deploy subcommands
│   │   ├── mod.rs
│   │   └── ecosystem.rs # Deploy ecosystem contracts
│   ├── upgrade/         # Upgrade subcommands
│   │   ├── mod.rs
│   │   ├── ecosystem.rs # Upgrade ecosystem
│   │   └── chain.rs     # Upgrade chain
│   └── accept.rs        # Accept ownership
├── state/               # State backend abstraction
│   ├── mod.rs           # StateBackend trait
│   └── filesystem.rs    # Filesystem implementation
├── ecosystem/           # Ecosystem domain logic (SDK)
│   ├── mod.rs
│   ├── config.rs        # Ecosystem configuration types
│   ├── contracts.rs     # Contract addresses
│   └── wallets.rs       # Wallet management
├── chain/               # Chain domain logic (SDK)
│   ├── mod.rs
│   ├── config.rs        # Chain configuration types
│   └── contracts.rs     # Chain contract addresses
├── external/            # External tool wrappers
│   ├── mod.rs
│   ├── zkstack.rs       # zkstack CLI wrapper
│   ├── forge.rs         # forge wrapper
│   └── cast.rs          # cast wrapper
└── funding/             # Wallet funding logic
    ├── mod.rs
    └── transfer.rs      # ETH/token transfers

tests/
├── unit/                # Unit tests
├── integration/         # Integration tests
└── contract/            # Contract tests (Anvil)

docker/
├── Dockerfile.deps      # Dependencies image (zkstack + foundry-zksync)
├── Dockerfile           # CLI image (on top of deps)
└── docker-bake.hcl      # Docker Bake configuration

Taskfile.yml             # Development task automation
```

**Structure Decision**: Single project with modular organization. SDK logic in domain modules (`ecosystem/`, `chain/`, `state/`, `external/`, `funding/`) for reusability. Commands are thin wrappers calling SDK functions.

## Complexity Tracking

> No constitution violations requiring justification.

| Violation | Why Needed | Simpler Alternative Rejected Because |
| --------- | ---------- | ------------------------------------ |
| N/A       | N/A        | N/A                                  |
