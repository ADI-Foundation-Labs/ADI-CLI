<!--
SYNC IMPACT REPORT
==================
Version change: (new) → 1.0.0
Modified principles: N/A (initial constitution)
Added sections:
  - Core Principles (6 principles)
  - Technology Stack
  - Development Workflow
  - Governance
Removed sections: N/A (initial constitution)
Templates requiring updates:
  - .specify/templates/plan-template.md: ✅ compatible (Constitution Check section exists)
  - .specify/templates/spec-template.md: ✅ compatible (no changes needed)
  - .specify/templates/tasks-template.md: ✅ compatible (no changes needed)
  - .specify/templates/checklist-template.md: ✅ compatible (no changes needed)
  - .specify/templates/agent-file-template.md: ✅ compatible (no changes needed)
Follow-up TODOs: None
-->

# ADI Chain CLI Constitution

## Core Principles

### I. Safe Rust Patterns (NON-NEGOTIABLE)

All code MUST adhere to safe Rust practices enforced by strict Clippy lints:

- **No panics**: `unwrap()`, `expect()`, and `panic!()` are forbidden. Use `eyre::Result` with
  `wrap_err()` for error propagation.
- **No unsafe indexing**: Array/slice indexing (`[]`) is forbidden. Use `.get()` and handle
  `Option` appropriately.
- **No wildcard imports**: All imports MUST be explicit.
- **Error context**: Every fallible operation MUST include contextual error messages via
  `wrap_err()` or `wrap_err_with()`.

**Rationale**: The CLI interacts with blockchain infrastructure where runtime panics can cause
data inconsistency or loss. Defensive coding prevents unexpected failures in production.

### II. Command Pattern Architecture

Every feature MUST be implemented as a command following the established pattern:

- Commands live in `src/commands/` as separate modules.
- Each command defines an enum with `#[derive(Subcommand)]`.
- Each command implements `async fn run(&self, context: &Context) -> Result<()>`.
- Commands are registered in `commands/mod.rs` under the `Commands` enum.
- Commands receive shared state via `Context` (config, logger).

**Rationale**: Consistent architecture enables predictable behavior, easier testing, and
straightforward extension of CLI capabilities.

### III. User Experience First

CLI output MUST prioritize clarity and actionability:

- **Structured output**: Support both human-readable (colored, formatted) and machine-readable
  (JSON) output modes where applicable.
- **Error messages**: MUST include what failed, why it failed, and how to fix it when possible.
- **Progress feedback**: Long-running operations MUST provide progress indication.
- **Exit codes**: Use standard exit codes (0 = success, 1 = error) consistently.
- **Help text**: Every command MUST have descriptive help text and examples.

**Rationale**: Developers use CLIs in scripts and interactive sessions. Clear output reduces
debugging time and improves adoption.

### IV. Test-Driven Development

Tests MUST be written before implementation when requested:

- **Unit tests**: For pure functions and isolated logic.
- **Integration tests**: For command execution and external interactions.
- **Contract tests**: For API/RPC interactions with zkSync OS server.
- Tests MUST fail before implementation (Red-Green-Refactor).

**Rationale**: TDD ensures requirements are captured as executable specifications and prevents
regressions in blockchain interactions where bugs have financial consequences.

### V. ZK Stack Compatibility

The CLI MUST maintain compatibility with specified ZK Stack component versions:

- **zkSync OS Server**: Version 0.12.0 (update version here when upgraded).
- **era-contracts**: Tag zkos-v0.29.11 (update tag here when upgraded).
- Version changes MUST be documented and tested before adoption.
- Breaking changes in upstream dependencies MUST trigger constitution amendment.

**Rationale**: ZK rollup infrastructure has strict version dependencies. Mismatched versions
can cause transaction failures or security vulnerabilities.

### VI. Configuration Hierarchy

Configuration MUST follow a predictable precedence (highest to lowest):

1. CLI flags (explicit user intent)
2. Environment variables (prefixed with `ADI_`)
3. Config file (`~/.adi_cli/.adi.yml`)
4. Built-in defaults

**Rationale**: This hierarchy enables flexible deployment (containers, CI/CD, local dev) while
maintaining reproducible behavior.

## Technology Stack

- **Language**: Rust (latest stable)
- **CLI Framework**: Clap 4 with derive macros
- **Async Runtime**: Tokio (full features)
- **Error Handling**: eyre for Result types
- **Configuration**: config crate with YAML support
- **Output Formatting**: colored crate
- **Serialization**: serde with JSON/YAML support

## Development Workflow

### Build Commands

```bash
cargo build              # Development build
cargo build --release    # Optimized release build (LTO enabled, panic=abort)
cargo run -- <args>      # Run with arguments
cargo clippy             # Lint (strict rules enforced - MUST pass)
cargo test               # Run tests (MUST pass before merge)
```

### Code Review Requirements

- All changes MUST pass `cargo clippy` with zero warnings.
- All changes MUST pass `cargo test`.
- New commands MUST follow the pattern in `commands/version.rs`.
- Error handling MUST use `wrap_err()` with descriptive messages.

### Adding New Commands

1. Create module in `src/commands/<command>.rs`.
2. Define enum with `#[derive(Subcommand)]`.
3. Implement `async fn run(&self, context: &Context) -> Result<()>`.
4. Register in `commands/mod.rs` under `Commands` enum.
5. Add tests in `tests/` directory.

## Governance

This constitution supersedes all other development practices for the ADI Chain CLI.

### Amendment Process

1. Propose changes via pull request to this file.
2. Document rationale for each change.
3. Require approval from project maintainers.
4. Update `LAST_AMENDED_DATE` and increment version appropriately.

### Version Policy

- **MAJOR**: Breaking changes to principles or removal of non-negotiables.
- **MINOR**: New principles added or existing principles materially expanded.
- **PATCH**: Clarifications, typo fixes, non-semantic refinements.

### Compliance

- All PRs MUST verify compliance with these principles.
- Constitution Check in plan templates MUST reference these principles.
- Violations require explicit justification and complexity tracking.

**Version**: 1.0.0 | **Ratified**: 2026-01-22 | **Last Amended**: 2026-01-22
