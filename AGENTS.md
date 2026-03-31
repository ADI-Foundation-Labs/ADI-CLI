## Overview

SDK-first Rust CLI (`adi-cli`) for managing ZkSync ecosystem smart contracts. The CLI runs on the host machine and orchestrates pre-built Docker toolkit images containing zkstack, foundry-zksync, and era-contracts.

**Requirement:** Docker must be installed and running on the host machine.

## Key Principles

- Write clear, concise, and idiomatic Rust code with accurate examples.
- Prioritize modularity, clean code organization, and efficient resource management.
- Use expressive variable names that convey intent (e.g., `is_ready`, `has_data`).
- Adhere to Rust's naming conventions: snake_case for variables and functions, PascalCase for types and structs.
- Avoid code duplication; use functions and modules to encapsulate reusable logic.
- Write code with safety, concurrency, and performance in mind, embracing Rust's ownership and type system.
- Ensure code is well-documented with inline comments and Rustdoc.
- Keep files small and focused (<200 lines)
- Test after every meaningful change
- Don't give out high level answers, your job is to give a specific solution applicable to the project.

## Error Handling and Safety

- Strict Clippy lints are enforced (see `Cargo.toml`)
- Run `cargo fmt` before commits
- Run `cargo clippy -- -D warnings` — treat warnings as errors
- No `unwrap()`, `expect()`, `panic!()` - use `eyre::Result` with `wrap_err()`
- No indexing/slicing - use safe alternatives like `.get()`
- Use `?` operator to propagate errors in functions.
- Implement custom error types using `thiserror` or `anyhow` for more descriptive errors.
- Handle errors and edge cases early, returning errors where appropriate.
- No wildcard imports
- Use exit codes: 0 = success, 1 = error, 2 = usage error
- Validate all input data

## Performance
- Use `&str` over `String` when possible
- Avoid unnecessary `.clone()` — prefer borrowing
- Use iterators over explicit loops

## Git Conventions

- Commit message format: feat|fix|refactor|docs|test|chore|ci|build|style: description
- `build` - Changes that affect the build system or external dependencies (dependencies update)
- `ci` - Changes to CI configuration files and scripts
- `docs` - Documentation only changes
- `feat` - A new feature
- `fix` - A bug fix
- `chore` - Changes which does not touch the code (ex. manual update of release notes). It will not generate release notes changes
- `refactor` - A code change that contains refactor
- `style` - Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
- `test` - Adding missing tests or correcting existing tests and also changes for our test app

## Documentation
- Document public functions with `///` doc comments
- Add examples in doc comments with ```` ```rust ```` blocks
- Keep `README.md` updated with usage examples
- Use `#![deny(missing_docs)]` in `lib.rs`

## Build Commands

```bash
cargo build              # Development build
cargo build --release    # Optimized release build (LTO enabled, panic=abort)
cargo run -- <args>      # Run with arguments
cargo clippy             # Lint (strict rules enforced)
cargo test               # Run tests
```

## CLI Commands

- `adi version` - Display CLI version and build information
- `adi config` - Display current configuration
- `adi ecosystem` - Display ecosystem and chain information with deployed contracts
- `adi owners` - Display owners of deployed L1 contracts
- `adi init` - Initialize new ecosystem configuration
- `adi add` - Add a new chain to an existing ecosystem
- `adi deploy` - Deploy ecosystem contracts to settlement layer
- `adi accept` - Accept pending ownership transfers for deployed contracts
- `adi transfer` - Accept ownership and transfer to a new owner address
- `adi verify` - Check and submit contract verification to block explorers

## Architecture

This is a Rust CLI application (`adi-cli`) using the command pattern with SDK-first design (core logic in reusable library crates).

### Core Components

- **main.rs**: Entry point. Parses CLI args via Clap, creates Context, dispatches to command handlers
- **context.rs**: `Context` struct carries config and logger through command execution
- **config.rs**: Loads config from `~/.adi.yml` (YAML) with `ADI__` environment variable overrides
- **error.rs**: Re-exports `eyre::Result` and `WrapErr` for consistent error handling

### Logging

- Uses `env_logger` crate for logging interface
- Colored output via `env_logger`'s built-in support (uses `anstyle` internally)
- Default log level: `info`
- Debug logs available (set `RUST_LOG=debug`)

### Adding Commands

Commands live in `src/commands/`. Each command module:
1. Defines args struct with `#[derive(Args)]` (if command has arguments)
2. Implements `pub async fn run(args, context: &Context) -> Result<()>`
3. Gets registered in `commands/mod.rs` under the `Commands` enum

See `commands/version.rs` (simple) or `commands/init/` (with args) for patterns.

### Workspace Packages

The CLI is built from 6 workspace packages with clear separation of concerns:

| Package         | Description                                        |
| --------------- | -------------------------------------------------- |
| `adi-docker`    | Pure Docker management SDK (bollard wrapper)       |
| `adi-toolkit`   | High-level toolkit container orchestration         |
| `adi-ecosystem` | Domain logic for ZkSync ecosystem management       |
| `adi-state`     | Abstract state management with filesystem backend  |
| `adi-funding`   | Wallet funding SDK with plan-execute pattern       |
| `adi-types`     | Shared domain types (wallets, contracts, metadata) |

Package dependency graph:
```
adi-cli (binary)
├── adi-toolkit ─────► adi-docker
├── adi-ecosystem ───► adi-types
├── adi-state ───────► adi-types
├── adi-funding ─────► adi-types
└── adi-types (no internal deps)
```

## Docker Architecture

The CLI orchestrates pre-built toolkit Docker images:

```
Host Machine
┌─────────────────────────────────────────────────────────┐
│  adi-cli (Rust binary)                                  │
│  ├── Commands (Clap)                                    │
│  ├── Docker Orchestrator (Bollard)                      │
│  └── Config (~/.adi.yml) / State (~/.adi_cli/state/)    │
└────────────────────┬────────────────────────────────────┘
                     │ Docker API
┌────────────────────▼────────────────────────────────────┐
│  Docker Daemon                                          │
│  └── adi-toolkit:v{VERSION} (ephemeral container)      │
│      ├── zkstack CLI                                    │
│      ├── foundry-zksync (forge, cast)                   │
│      └── era-contracts                                  │
└─────────────────────────────────────────────────────────┘
```

**Toolkit Images:**
- Pre-built images containing all dependencies
- Tagged by protocol version (e.g., `v30.0.2`)
- Default registry: `harbor.sde.adifoundation.ai/adi-public/cli`
- Image name: `adi-toolkit`
- Auto-pulled when missing

**Container Lifecycle:**
- Ephemeral: containers are created, run, and removed per operation
- State persists via host volume mounts to `~/.adi_cli/state/`
- Real-time output streaming to terminal

## State Backend

The `adi-state` crate provides abstract state storage:
- **StateBackend trait**: Low-level key-value operations (async)
- **FilesystemBackend**: Default implementation using tokio::fs with YAML serialization
- **StateManager**: High-level typed API for ecosystem/chain state operations
- **BackendType enum**: Extensible for future database backends

Key design: Write operations require existing files (prevents accidental creation).

## Ecosystem Directory Structure

```
~/.adi_cli/state/<ecosystem-name>/
├── ZkStack.yaml                    # Ecosystem metadata
├── configs/                        # Ecosystem-level configs
│   ├── wallets.yaml                # Ecosystem wallets
│   ├── contracts.yaml              # Deployed contract addresses
│   ├── initial_deployments.yaml    # Initial deployment settings
│   ├── erc20_deployments.yaml      # ERC20 token deployments
│   └── apps.yaml                   # Apps config (explorer, portal)
└── chains/<chain-name>/            # Per-chain directories
    ├── ZkStack.yaml                # Chain metadata
    └── configs/
        ├── wallets.yaml            # Chain-specific wallets
        └── contracts.yaml          # Chain contract addresses
```

## EVM Types and Dependencies

Use `alloy_*` crates (v1.5) for all EVM-related types - do not create custom types:

| Crate                  | Purpose                                             |
| ---------------------- | --------------------------------------------------- |
| `alloy-primitives`     | Address, B256, U256, Bytes                          |
| `alloy-signer`         | Signer trait definition                             |
| `alloy-signer-local`   | Local wallet signing with PrivateKeySigner          |
| `alloy-provider`       | JSON-RPC provider for settlement layer interactions |
| `alloy-network`        | EthereumWallet for signing providers                |
| `alloy-contract`       | Contract interaction helpers                        |
| `alloy-sol-types`      | Solidity type encoding/decoding                     |
| `alloy-rpc-types`      | RPC request/response types                          |
| `alloy-transport-http` | HTTP transport with reqwest                         |

Additional dependencies:
- `semver` - Protocol version handling (e.g., `Version::new(30, 0, 2)`)
- `secrecy` - Secret string handling for private keys
- `colored` - Terminal output styling
- `dialoguer` - Interactive CLI prompts

## Dependency Management

All dependencies MUST be specified in the root `Cargo.toml`:
- Disable `default-features` by default
- Enable only required features explicitly
- Sub-packages use workspace dependencies via `{ workspace = true }`

Example root Cargo.toml:
```toml
[workspace.dependencies]
alloy-primitives = { version = "1.5", default-features = false, features = ["serde"] }
alloy-signer = { version = "1.5", default-features = false }
alloy-provider = { version = "1.5", default-features = false, features = ["reqwest"] }
tokio = { version = "1", default-features = false, features = ["rt-multi-thread", "macros"] }
```

Sub-package Cargo.toml:
```toml
[dependencies]
alloy-primitives = { workspace = true }
tokio = { workspace = true }
```

