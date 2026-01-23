## Overview

SDK-first Rust CLI (`adi-cli`) for managing ZkSync ecosystem smart contracts within Docker containers.

## Build Commands

```bash
cargo build              # Development build
cargo build --release    # Optimized release build (LTO enabled, panic=abort)
cargo run -- <args>      # Run with arguments
cargo clippy             # Lint (strict rules enforced)
cargo test               # Run tests
```

## CLI Commands

- `adi init ecosystem` - Initialize new ecosystem configuration
- `adi deploy ecosystem` - Deploy ecosystem contracts to L1
- `adi init chain` - Initialize and register a chain within an ecosystem
- `adi doctor` - Verify external dependency availability (zkstack, forge, cast)
- `adi upgrade ecosystem` - Upgrade ecosystem contracts to new protocol version
- `adi upgrade chain` - Upgrade chain contracts to match ecosystem version
- `adi accept ownership` - Accept pending ownership transfers post-deployment
- `adi register verifier` - Register verifier for specific execution version

## Architecture

This is a Rust CLI application (`adi-cli`) using the command pattern with SDK-first design (core logic in reusable library crates).

### Core Components

- **main.rs**: Entry point. Parses CLI args via Clap, creates Context, dispatches to command handlers
- **context.rs**: `Context` struct carries config and logger through command execution
- **config.rs**: Loads config from `~/.adi_cli/.adi.yml` (YAML) with `ADI_` environment variable overrides
- **error.rs**: Re-exports `eyre::Result` and `WrapErr` for consistent error handling

### Logging

- Uses `env_logger` crate for logging interface
- Colored output via `env_logger`'s built-in support (uses `anstyle` internally)
- Default log level: `info`
- Debug logs available (set `RUST_LOG=debug`)

### Adding Commands

Commands live in `src/commands/`. Each command module:
1. Defines an enum with `#[derive(Subcommand)]`
2. Implements `async fn run(&self, context: &Context) -> Result<()>`
3. Gets registered in `commands/mod.rs` under the `Commands` enum

See `commands/version.rs` for the pattern.

## Docker

Two-layer image structure:
1. **Dependencies image**: zkstack CLI + foundry-zksync
2. **CLI image**: This tool built on top of dependencies

Uses Docker Bake for parameterized builds. State directories mounted from host for persistence.

## State Backend

Abstract key-value storage interface with filesystem-based default implementation. Designed for extension to database backends.

## Ecosystem Directory Structure

```
ecosystem/
├── ZkStack.yaml              # Ecosystem metadata
├── configs/                  # Ecosystem-level configs
│   ├── wallets.yaml
│   ├── contracts.yaml
│   └── initial_deployments.yaml
└── chains/<chain-name>/      # Per-chain directories
    └── configs/
        ├── contracts.yaml
        ├── wallets.yaml
        ├── genesis.yaml
        └── general.yaml
```

## Code Style

Strict Clippy lints are enforced (see `Cargo.toml`):
- No `unwrap()`, `expect()`, `panic!()` - use `eyre::Result` with `wrap_err()`
- No indexing/slicing - use safe alternatives like `.get()`
- No wildcard imports

## EVM Types and Dependencies

Use `alloy_*` crates for all EVM-related types - do not create custom types:

| Crate | Purpose |
|-------|---------|
| `alloy-primitives` | Address, B256, U256, Bytes |
| `alloy-signer` | Local wallet signing, address derivation |
| `alloy-provider` | JSON-RPC provider for L1 interactions |
| `alloy-contract` | Contract interaction helpers |
| `alloy-sol-types` | Solidity type encoding/decoding |

Additional dependencies:
- `semver` - Protocol version handling (v29.0.11 → `Version::new(29, 0, 11)`)
- `secrecy` - Secret string handling for private keys

## Dependency Management

All dependencies MUST be specified in the root `Cargo.toml`:
- Disable `default-features` by default
- Enable only required features explicitly
- Sub-packages use workspace dependencies via `{ workspace = true }`

Example root Cargo.toml:
```toml
[workspace.dependencies]
alloy-primitives = { version = "0.8", default-features = false }
alloy-signer = { version = "0.8", default-features = false }
tokio = { version = "1", default-features = false, features = ["rt-multi-thread", "macros"] }
```

Sub-package Cargo.toml:
```toml
[dependencies]
alloy-primitives = { workspace = true }
tokio = { workspace = true }
```
