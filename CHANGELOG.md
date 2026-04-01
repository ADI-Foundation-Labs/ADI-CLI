# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-04-01

### Changed

- Chain upgrade parameters grouped into `ChainUpgradeContext` struct, replacing long parameter lists in `run_chain_upgrades`
- S3 event handler switched from `std::sync::Mutex` to `tokio::sync::Mutex` for proper async locking without potential deadlocks
- File copy during chain upgrades uses async `tokio::fs::copy` instead of blocking `std::fs::copy`
- Main function return type simplified from `Box<dyn std::error::Error>` to `eyre::Result<()>`
- Large modules split into focused sub-modules: `chain_prompts`, `ecosystem`, `helpers`, `owners`, `transfer/ownership`, `config`, and `state`
- Removed `#[allow(dead_code)]` and `#[allow(unused_variables)]` annotations from actively used code

### Added

- Unit tests for `lerp`, `center`, and `build_subtitle` in version command
- Unit tests for `normalize_path` in state paths module
- Unit tests for `indent_as_array_item`, `find_chains_insertion_point` in config writer
- Shared S3 helpers module (`state/helpers.rs`) extracting `get_tenant_id`, `get_access_key_id`, and `get_secret_access_key`

## [0.2.0] - 2026-04-01

### Added

- **Chain contract types** (`ChainL1Contracts`, `ChainL2Contracts`, `BridgeContracts`, `BridgesConfig`) in `adi-types` for structured chain-level contract address management
- **Funding event system** with `FundingEventHandler` trait, `LoggingEventHandler`, and `SpinnerEventHandler` for real-time progress reporting during wallet funding
- **`ToolkitRunnerTrait`** abstraction in `adi-upgrade` to enable testing of toolkit container operations
- **Signing provider builder** helper (`build_signing_provider`) in `adi-upgrade` for constructing wallet-backed RPC providers
- **Funding config tests** covering CGT amount calculations, wallet role display names, and config builder patterns
- **Invalid backend configuration** error variant in `adi-state`
- **Transfer context** in `adi-funding` for improved balance checks and transfer execution

### Changed

- Docker image pulls now always fetch the latest version instead of skipping when a local copy exists
- Image pull progress tracking extracted into composable helper functions with `cliclack` progress bars
- Filesystem state backend uses atomic file operations (`OpenOptions`) instead of `exists()` checks followed by read/write, eliminating TOCTOU race conditions
- State backend trait simplified by removing serialize/deserialize helpers from `FilesystemBackend`
- Validator role transaction parameters grouped into `ValidatorRoleTxParams` struct, replacing long parameter lists
- Ecosystem deployment enhanced with zkstack initialization and validator role configuration
- S3 client uses improved object existence checking
- Ecosystem contract counting simplified with `count_some` helper
- Ownership transfer functions streamlined with improved context management
- Verification command split into focused modules (`check`, `config`, `contracts`, `submit`)
- Accept ownership command split into modules (`config`, `execute`)
- Toolkit runner split into modules (`commands`, `params`)
- Implementation address reader split into modules (`apply`, `contracts`, `readers`, `slots`, `types`)
- Funding events split into modules (`logging`, `spinner`)
- Contract types reorganized into module hierarchy (`bridge`, `chain`, `ecosystem`)
- Verification registry builders refactored with extracted `ecosystem_targets` module

### Fixed

- `adi init` now respects user confirmation flag when saving chain configuration

### Removed

- Unused `eyre` dependency from `adi-toolkit`
- Unused error variants (`CommandFailed`, `InvalidVersion`) from toolkit error type
- Local image existence check from Docker image manager (always pull for freshness)

## [0.1.0] - 2026-03-30

### Added

- **CLI framework** with Clap-based command parsing, YAML configuration (`~/.adi.yml`), and `ADI__` environment variable overrides
- **`adi init`** command to initialize new ZkSync ecosystem configurations with interactive prompts for settlement layer, DA layer, base token, and chain parameters
- **`adi add`** command to add new chains to an existing ecosystem with support for L2/L3, custom gas tokens, and DA configuration (Ethereum blobs, Avail, Celestia)
- **`adi deploy`** command to deploy ecosystem smart contracts to the settlement layer, including ERC20 tokens, bridge contracts, and chain registration
- **`adi accept`** command to accept pending ownership transfers for deployed L1 contracts
- **`adi transfer`** command to accept ownership and transfer all ecosystem contracts to a new owner address
- **`adi owners`** command to display current owners, pending owners, and admin roles for all deployed contracts
- **`adi ecosystem`** command to display ecosystem and chain information with deployed contract addresses
- **`adi verify`** command to check and submit contract verification to block explorers (Etherscan-compatible), with diamond proxy facet support
- **`adi upgrade`** command to upgrade ecosystem and chain contracts to a new protocol version, with orchestrated phases (validation, simulation, confirmation, broadcast, governance)
- **`adi config`** command to display current configuration
- **`adi version`** command with build metadata (commit hash, build date, Rust version)
- **`adi state`** subcommands for state synchronization and restoration with S3
- **`adi server-params`** command to output Docker Compose configuration parameters with optional JSON output
- **`adi completions`** command for shell completion script generation
- **Docker orchestration** via Bollard SDK with ephemeral container lifecycle, registry authentication, automatic image pulling, and real-time log streaming with sliding window
- **`adi-docker` package** for low-level Docker client management, container creation, image operations, and stream handling
- **`adi-toolkit` package** for high-level toolkit container orchestration with pre-built Docker images tagged by protocol version
- **`adi-ecosystem` package** for domain logic including deployment configuration, ownership management (collect, accept, transfer), validator role assignment, contract verification registry, and signer utilities
- **`adi-state` package** with abstract `StateBackend` trait, filesystem backend with YAML serialization, and typed `StateManager` API for ecosystem/chain state
- **`adi-funding` package** with plan-execute pattern for wallet funding, Anvil auto-funding detection, balance checking, and event-driven progress reporting
- **`adi-types` package** for shared domain types (wallets, contracts, metadata, protocol versions)
- **`adi-upgrade` package** for protocol upgrade orchestration with version handlers, bytecode validation, YAML config generation, on-chain state queries, simulation, and governance encoding
- **Wallet funding** with automatic plan calculation, minimum balance thresholds, and support for both testnet (Anvil) and live networks
- **Ownership management** with multi-contract collection, batched acceptance, full transfer flows, and detailed status reporting
- **Contract verification** with implementation address resolution via storage slots, diamond proxy facet enumeration, constructor argument encoding, and Etherscan API integration
- **Interactive UI** with themed prompts, confirmations, and multi-select pickers via `dialoguer` and `console`
- **Colored terminal output** and structured logging via `env_logger` with configurable log levels

[0.2.1]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/ADI-Foundation-Labs/ADI-CLI/releases/tag/0.1.0
