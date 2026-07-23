# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-07-23

### Added

- **Validium / custom DA mode support** — chains can use external data availability (e.g. Avail) via `pubdata_mode: custom_da`, created as validium chains with the mode-specific L1 DA validator wired automatically
- Automatic **`NoxTransactionFilterer` deployment** and registration on the chain's Diamond (via ChainAdmin multicall) for validium/custom-DA chains, with the address persisted to chain state, shown in `adi ecosystem` output, and contract sources prebuilt into the toolkit image
- **`settlement` ecosystem config key** (`l1` | `l2`, default `l2`) selecting the settlement layer, which drives the L1-sender fee tier (L1 ~25 gwei cap vs L2 ~1500 gwei cap) independently of the DA transport
- `--pubdata-mode` flag (`blobs` | `calldata` | `custom-da`) on `adi init`, `adi add`, and `adi deploy`, plus an interactive pubdata mode prompt when not provided
- Pubdata/settlement compatibility validation: `adi init` and `adi deploy` reject `pubdata_mode: blobs` with `settlement: l2` (blobs only exist on Ethereum L1) before touching state or the network

### Changed

- **BREAKING:** the `blobs: true/false` config key and the `--blobs` deploy flag are replaced by the `pubdata_mode` enum (`blobs` | `calldata` | `custom_da`, default `calldata`); the value parses case-insensitively and ignores `-`/`_`
- L1-sender fee tier selection is based on the settlement layer instead of the blobs flag; blob-gas vs pubdata-price overrides are chosen by DA transport
- Interactive prompt notes word-wrap properly for long messages (cliclack 0.5 / console 0.16 upgrade)

### Fixed

- Container creation on macOS is retried when Docker Desktop transiently reports "bind source path does not exist" for a just-created mount directory (VirtioFS race)

## [0.3.0] - 2026-06-17

### Added

- **snake_case name enforcement** for ecosystems and chains (`validate_snake_case_name`) — rejects names that would diverge from zkstack's stored form and suggests a corrected snake_case form
- **Unified config home** at `~/.adi_cli/.adi.yml` (preferred), with the legacy `~/.adi.yml` location kept as a deprecated fallback
- Startup warnings for misplaced or unrecognized top-level config keys, surfaced via a new `LoadedConfig` carrying non-fatal load warnings
- Ownership handoff extended to the **Verifier, Native Token Vault, and L1 Nullifier** across the deploy, accept, and transfer flows — the nullifier is reclaimed from the deployer to the governor before any transfer to a configured new owner
- Bridged Token Beacon and L1 Nullifier now shown in `adi owners` output
- Centralized `resolve_*` config helpers (gas multiplier, funder/private keys, explorer type/API key/URL) with full unit-test coverage

### Changed

- `adi accept --calldata` no longer requires a signing key — it resolves the expected pending owner from the configured `new_owner` (per ecosystem and per chain), falling back to the governor address
- `adi accept` execute path reads canonical ecosystem and per-chain owner keys instead of the deprecated top-level `ownership.private_key`; the chain signer no longer reuses the ecosystem key
- `adi ecosystem` groups contracts by semantic role, so CTM-derived chain contracts display under the selected chain instead of the ecosystem panel
- `adi owners` queries owners and pending owners concurrently (preserving display order), relabels the ecosystem Chain Admin, and drops the L2 ConsensusRegistry that has no L1 owner
- Per-contract accept/transfer logic consolidated into generic `Ownable2Step` helpers to remove duplication

### Removed

- **BREAKING:** top-level `ownership` and `funding.rpc_url` config keys are no longer read — use `ecosystem.ownership` (or `ecosystem.chains[].ownership`) and `ecosystem.rpc_url` instead. A stray top-level `ownership:` key now warns with a pointer to the replacement

### Fixed

- `adi accept --calldata` printed nothing after a transfer to a configured `new_owner`, because it checked pending transfers against a derived signing-key address
- Chain `ProxyAdmin` address persisted to `contracts.yaml` is now the actually-deployed address — zkstack's `RegisterZKChain` script computed a phantom address from a wrong CREATE2 init-code hash, which is now patched in-container to read back `Create2AndTransfer.deployedAddress()`
- Config values resolve consistently across every command, eliminating dead fields (e.g. `funding.rpc_url: null`) from `adi config`
- Note and prompt body content renders at full brightness instead of the upstream dimmed `Submit` style that washed out labels and values

## [0.2.5] - 2026-05-27

### Added

- **`FeeAdjusterConfig` L1 deployment** as a post-step of `adi deploy` — runs `forge script` inside the toolkit container, initializes the contract with the chain's `ChainAdmin` as owner, and persists the address to `chains/<chain>/configs/contracts.yaml` under `l1.fee_adjuster_config`
- `fee_adjuster.enabled` flag in `.adi.yml` (default `true`) to opt out of the post-deployment step
- `Fee Adjuster Config` line in the `adi ecosystem` chain L1 contracts display
- `fee_adjuster_config` field on `ChainL1Contracts` plus `ChainContracts::fee_adjuster_config()` accessor
- `ForgeScriptParams` + `ToolkitRunner::run_forge_script` in `adi-toolkit` for executing arbitrary `forge script` invocations against a mounted source tree with container-aware RPC URL rewriting and a `DEPLOYER_PRIVATE_KEY` env injection
- Idempotency in the fee-adjuster step: a chain that already has `fee_adjuster_config` set is skipped on re-run

### Changed

- Toolkit Docker image now clones `fee-adjuster-contracts` (pinned to `main`) and `forge-std` v1.9.4 into `/deps/fee-adjuster-contracts`, installs OpenZeppelin via `npm ci`, and pre-builds artefacts during image build. Credentials are accepted via BuildKit SSH agent forwarding **or** a `gitlab_token` secret (HTTPS+PAT, preferred in CI)
- Toolkit image installs Node.js 20 and `openssh-client` to support the above
- Build credential plumbing (`ssh = ["default"]`, `secret = [...]`) moved from CLI flags into `docker/docker-bake.hcl`, gated on the `ENABLE_SSH` / `GITLAB_TOKEN` env vars so the Taskfile can pick the right mode automatically

### Fixed

- Docker image builds in GitLab CI (dind) no longer abort with `invalid empty ssh agent socket` — credentials are now forwarded as a BuildKit secret sourced from `CI_JOB_READ_REPO_TOKEN` instead of requiring an SSH agent

## [0.2.4] - 2026-04-21

### Added

- **`adi_dev_3` ecosystem configuration template** (`configs/.adi.dev_3_ecosystem.yml`) with Sepolia RPC placeholder, `adi_devnet_3` chain (chain_id `99982`), CGT base token address, blob-mode enabled, and per-operator ETH funding allocations
- **Forced external price API parameters** in `adi server-params` output:
  - `external_price_api_client_source` set to `Forced`
  - `base_token_price_updater_enabled` set to `true`
  - `external_price_api_client_forced_prices__json` — JSON map containing the ETH placeholder price (3000.0) and, when the chain has a custom base token, the CGT token address mapped to `1.0`
- **Observability logging parameters** in server params: `observability_log_format` (`terminal`) and `observability_log_use_color` (`true`)
- `base_token_address` field on `ServerParamsInput`, resolved from the chain config's `base_token_address`, so the forced-prices map can include custom gas tokens
- Unit tests covering forced-prices JSON generation with and without a base token, and verifying the new observability / price-API parameters are emitted

### Changed

- L2 (blob) mode now explicitly emits `l1_sender_pubdata_mode = Blobs` as a server parameter (previously this key was omitted in L2 mode)

## [0.2.3] - 2026-04-03

### Added

- **`adi refund` command** to drain ETH and ERC20 tokens from ecosystem and chain wallets back to a receiver address, replacing the external `return_funds_l1_sepolia.sh` script
- `--receiver` flag with automatic fallback to funder address from config (`funding.funder_key` / `ADI_FUNDER_KEY`)
- `--chain` flag to refund a specific chain only (default: all chains)
- `--token-address` flag for explicit ERC20 token refund, with automatic detection of custom gas tokens from chain metadata (`base_token`)
- Token symbol and decimals queried dynamically from on-chain ERC20 contracts
- Gas estimation via RPC (`eth_estimateGas`) instead of hardcoded values, with conservative fallbacks
- Continue-on-error execution so partial refunds succeed even if individual wallets fail
- Per-wallet balance checking progress with cliclack spinner showing wallet role and address
- Refund plan displayed as a boxed `cliclack::note` with green-styled addresses and amounts
- `RefundConfig` struct grouping receiver, token address, and gas multiplier for clean API surface
- `format_eth` and `format_with_decimals` made public in `adi-funding` for reuse across CLI commands
- `RefundTransferFailed` error variant in `adi-funding` for continue-on-error wallet failures
- **`adi-funding/refund` submodule** with `types.rs`, `plan.rs`, and `execute.rs` following SDK-first architecture

## [0.2.2] - 2026-04-02

### Added

- **`adi-vault` package** for HashiCorp Vault HTTP client with KV v2 health checks and secret writing
- `--upload` flag on `adi server-params` to push generated parameters directly to HashiCorp Vault with interactive token prompt and path validation
- `vault` configuration section in `~/.adi.yml` with configurable `api_url` for Vault base URL
- `fee_collector_address` override in chain defaults, used as fee collector in server parameters when set
- Chain genesis file reading with JSON compaction and base64 encoding for server parameter output
- Prover mode-aware server parameters toggling fake SNARK/FRI provers based on `NoProofs` vs `Gpu`
- L2 blob-mode server parameters with dedicated fee, gas, and blob gas overrides
- Static server parameters for logging, RocksDB path, genesis input path, prover API, sequencer block settings, batcher config, and poll intervals
- Unit tests for server parameter extraction (L2/L3 modes, prover modes, numeric types, static fields)
- Unit tests for Vault path validation
- `rustfmt.toml` with project formatting rules

### Changed

- `server_params` module split from single file into sub-modules (`constants`, `params`, `mod`)
- Server parameter values use typed `serde_json::Value` (numbers and strings) instead of `Option<String>`
- Error handling in `server-params` command consolidated into shared `handle_missing` helper
- L3 calldata-mode parameters updated with explicit constants for gas, fee, and pubdata overrides

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

[0.4.0]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.2.5...0.3.0
[0.2.5]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.2.4...0.2.5
[0.2.4]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.2.3...0.2.4
[0.2.3]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.2.2...0.2.3
[0.2.2]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/ADI-Foundation-Labs/ADI-CLI/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/ADI-Foundation-Labs/ADI-CLI/releases/tag/0.1.0
