# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/ADI-Foundation-Labs/ADI-CLI/releases/tag/0.1.0
