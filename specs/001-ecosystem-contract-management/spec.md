# Feature Specification: Ecosystem Contract Management CLI

**Feature Branch**: `001-ecosystem-contract-management`
**Created**: 2026-01-22
**Status**: Draft
**Input**: User description: SDK-first Rust CLI for managing ZkSync ecosystem smart contracts. The CLI runs on the host machine and orchestrates pre-built Docker toolkit images containing zkstack, foundry-zksync, and era-contracts. Features abstract state backends, automation of chain deployments, and contract upgrade operations.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Initialize New Ecosystem (Priority: P1)

As a chain operator, I want to initialize a new ZkSync ecosystem configuration from scratch so that I can deploy smart contracts to the settlement layer and create the foundation for my chain.

**Why this priority**: This is the foundational operation that must occur before any other ecosystem management. Without ecosystem initialization, no other operations are possible.

**Independent Test**: Can be fully tested by running the init command with Docker available, verifying that all configuration files (ZkStack.yaml, wallets.yaml, contracts.yaml) are generated in the state directory.


**Acceptance Scenarios**:

1. **Given** Docker is running with toolkit images available, **When** user runs `adi init ecosystem` with ecosystem name and settlement network configuration, **Then** the system creates the ecosystem directory structure with ZkStack.yaml, configs directory, and chains directory.

2. **Given** user provides a config file with private keys, **When** ecosystem init is executed, **Then** the system uses provided keys for deployer and governor wallets instead of generating random ones.

3. **Given** no existing state, **When** user runs `adi init ecosystem --chain-name mychain --chain-id 222`, **Then** a chain configuration is created with proper wallet files (wallets.yaml containing deployer, operator, governor keys).

4. **Given** ecosystem init completes successfully, **Then** all generated files are stored in the configured state directory on the host machine.

---

### User Story 2 - Deploy Ecosystem Contracts (Priority: P1)

As a chain operator, I want to deploy ecosystem smart contracts to the settlement layer so that my chain infrastructure is established.

**Why this priority**: Contract deployment is required immediately after initialization to make the ecosystem operational. This is core functionality.

**Independent Test**: Can be tested by running deploy command against a local Anvil node, verifying that bridgehub, state transition manager, and governance contracts are deployed and addresses stored in contracts.yaml.

**Acceptance Scenarios**:

1. **Given** an initialized ecosystem with funded wallets, **When** user runs `adi deploy ecosystem`, **Then** ecosystem contracts are deployed and contract addresses are persisted to state.

2. **Given** deployment is successful, **When** checking contracts.yaml, **Then** it contains bridgehub_proxy_addr, governance, chain_admin, validator_timelock_addr, and all other ecosystem contract addresses.

3. **Given** deployment fails mid-way, **When** user re-runs deploy command, **Then** the system resumes from the last successful step or provides clear instructions for recovery.

4. **Given** custom gas price requirements, **When** user provides `--gas-price` flag, **Then** deployment uses the specified gas price for all transactions.

5. **Given** a funder wallet is configured with sufficient ETH (and CGT if custom base token is configured), **When** user runs deployment, **Then** ecosystem wallets are automatically funded before contract deployment begins.

6. **Given** a funder wallet has insufficient balance, **When** user runs deployment, **Then** system reports required amounts (ETH, and CGT if custom base token) and halts before any transactions.

7. **Given** protocol version requires verifier registration (versions <0.30.2), **When** deployment completes, **Then** verifiers are automatically registered as part of the deployment process.

8. **Given** deployment creates contracts with pending ownership, **When** deployment completes, **Then** ownership is automatically accepted for all Ownable2Step contracts using `acceptOwnership()` and for governance-controlled contracts using `governanceAcceptOwner()`. The specific contracts requiring ownership acceptance vary by protocol version (see research.md Section 6 for version-specific lists).

---

### User Story 3 - Initialize Chain Configuration (Priority: P1)

As a chain operator, I want to initialize a new chain configuration within my ecosystem so that I can prepare for chain deployment.

**Why this priority**: Chain initialization is the first step before deployment. Configuration must be established before contracts can be deployed.

**Independent Test**: Can be tested by running `adi init chain` after ecosystem initialization, verifying chain directory structure and configuration files (wallets.yaml, genesis.yaml) are created.

**Acceptance Scenarios**:

1. **Given** an initialized ecosystem, **When** user runs `adi init chain --name adi --chain-id 222`, **Then** chain configuration directory is created with wallets.yaml and genesis.yaml.

2. **Given** chain initialization succeeds, **When** checking chain directory, **Then** it contains proper wallet files with deployer, operator, governor keys.

3. **Given** a base token configuration, **When** user specifies `--base-token-address <ADDRESS>`, **Then** the chain configuration includes the custom base token instead of ETH.

---

### User Story 3b - Deploy Chain Contracts (Priority: P1)

As a chain operator, I want to deploy chain contracts and register my chain with the ecosystem so that I can operate a ZkSync rollup.

**Why this priority**: Chain deployment is required to make the chain operational. Without deployed contracts, the chain cannot process transactions.

**Independent Test**: Can be tested by running `adi deploy chain` after ecosystem deployment, verifying chain contracts are deployed and chain is registered with bridgehub.

**Acceptance Scenarios**:

1. **Given** an initialized chain and deployed ecosystem, **When** user runs `adi deploy chain --name adi`, **Then** chain-specific contracts are deployed and chain is registered with the bridgehub.

2. **Given** chain deployment succeeds, **When** checking chain contracts.yaml, **Then** it contains diamond_proxy_addr, governance_addr, chain_admin_addr, and all settlement layer/L2 contract addresses.

3. **Given** custom gas price requirements, **When** user provides `--gas-price` flag, **Then** deployment uses the specified gas price for all transactions.

4. **Given** a funder wallet is configured, **When** user runs deployment with `--auto-fund`, **Then** chain wallets are automatically funded before contract deployment begins.

5. **Given** chain deployment creates contracts with pending ownership, **When** deployment completes, **Then** ownership is automatically accepted for all Ownable2Step contracts using `acceptOwnership()`. The specific contracts vary by protocol version (see research.md Section 6 for version-specific lists).

---

### User Story 4 - Upgrade Ecosystem Contracts (Priority: P2)

As a chain operator, I want to upgrade ecosystem contracts to a new protocol version so that I can benefit from new features and security fixes.

**Why this priority**: Upgrades are critical for long-term operation but not required for initial deployment.

**Independent Test**: Can be tested by deploying v29 ecosystem, then running upgrade command to v30, verifying protocol version changes and new contracts are deployed.

**Acceptance Scenarios**:

1. **Given** an ecosystem running protocol version v29, **When** user runs `adi upgrade ecosystem --to v30`, **Then** upgrade preparation is performed and calldata for governance execution is generated.

2. **Given** upgrade calldata is generated, **When** system outputs the scheduleTransparent and execute calldata, **Then** operator can execute these via governance contract.

3. **Given** upgrade is executed successfully, **When** checking protocol version, **Then** ecosystem reports new protocol version.

---

### User Story 5 - Upgrade Chain Contracts (Priority: P2)

As a chain operator, I want to upgrade my chain's contracts to match the ecosystem protocol version so that my chain can use new features.

**Why this priority**: Chain upgrades must follow ecosystem upgrades and are essential for continued operation.

**Independent Test**: Can be tested by upgrading chain after ecosystem upgrade, verifying diamond proxy reports new protocol version.

**Acceptance Scenarios**:

1. **Given** ecosystem is upgraded to v30, **When** user runs `adi upgrade chain --to v30`, **Then** chain upgrade calldata is generated for chain admin execution.

2. **Given** chain upgrade calldata is executed, **When** checking diamond proxy protocol version, **Then** it matches the target version.

3. **Given** upgrade requires DA validator pair update, **When** upgrade completes, **Then** system provides instructions or performs setDAValidatorPair call.

---

### User Story 6 - Manage State Backend (Priority: P3)

As a chain operator, I want to use different state backends so that I can persist ecosystem state in various storage systems.

**Why this priority**: Flexible state backends enable future extensibility but filesystem is sufficient for initial release.

**Independent Test**: Can be tested by configuring filesystem backend and verifying all operations read/write state correctly.

**Acceptance Scenarios**:

1. **Given** filesystem state backend is configured, **When** user performs any operation, **Then** state is persisted to the configured directory path.

2. **Given** state directory is configured, **When** CLI operations complete, **Then** all state is preserved in the configured directory.

3. **Given** state exists from previous session, **When** CLI is run again, **Then** operations can continue from previous state.

---

### Edge Cases

- What happens when settlement layer RPC is unreachable during deployment? System should provide clear error with retry guidance.
- What happens when funder wallet has insufficient funds? System should check ETH (and CGT if custom base token) balances before deploying and report required amounts for each.
- What happens when gas price changes significantly during multi-transaction deployment? System should handle transaction failures gracefully.
- What happens when user tries to upgrade to an unsupported version? System should validate target version before proceeding.
- What happens when state directory has corrupted files? System should validate state integrity on startup.
- What happens when Docker daemon is not running? System should detect and report clearly with instructions to start Docker.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST be architected with SDK-first approach where core logic resides in reusable library crates, with CLI being a thin wrapper.

- **FR-002**: System MUST run on the host machine and orchestrate pre-built Docker toolkit images. The CLI does NOT run inside Docker containers.

- **FR-003**: System MUST use pre-built toolkit Docker images containing zkstack CLI, foundry-zksync, and era-contracts. Images are tagged by protocol version (e.g., `v29`, `v30`).

- **FR-004**: System MUST use Docker Bake for building parameterized toolkit container images with:
  - zksync-era commit (smart contracts)
  - era-contracts tag
  - foundry-zksync version
  - Optional: os-integration commit, zk-os commit

- **FR-005**: System MUST implement abstract state backend with key-value storage interface.

- **FR-006**: System MUST provide filesystem-based state backend implementation as default.

- **FR-007**: State backend MUST be designed for easy extension to other implementations (e.g., database backends).

- **FR-008**: System MUST use Taskfile for development operations (build binary, build docker images, run tests, etc.).

- **FR-009**: System MUST ONLY manage smart contracts - no server or prover deployment functionality.

- **FR-010**: System MUST work with ecosystem directory structure matching the reference structure:
  - `ZkStack.yaml` - ecosystem metadata
  - `configs/` - ecosystem-level configs (wallets.yaml, contracts.yaml, initial_deployments.yaml)
  - `chains/<chain-name>/` - per-chain directories
  - `chains/<chain-name>/configs/` - chain configs (contracts.yaml, wallets.yaml, genesis.yaml, genesis.json, general.yaml, secrets.yaml)
  - `volumes/` - runtime data directories

- **FR-011**: System MUST allow users to provide private keys in configuration for deployment and wallet funding.

- **FR-012**: System MUST mount state directories into toolkit containers for persistence.

- **FR-013**: System MUST support creating new ecosystem state from scratch in the configured state directory.

- **FR-014**: System MUST automate the manual processes documented in deployment and upgrade guides.

- **FR-015**: System MUST auto-detect protocol version from ecosystem configuration and select the appropriate toolkit image, with CLI flag override (`--protocol-version`) available.

- **FR-016**: System MUST support configuring settlement layer RPC URL for contract interactions.

- **FR-017**: System MUST follow config-file-first approach where configuration values (settlement layer RPC URL, state directory paths, private keys, CGT address, chain name/ID, gas price) are read from config file by default, with optional CLI flags for overrides. Action-specific parameters like `--to <version>` for upgrades remain required flags.

- **FR-018**: System MUST persist all contract addresses and deployment state to state backend.

- **FR-019**: System MUST generate correct calldata for governance operations (scheduleTransparent, execute).

- **FR-020**: System MUST support both Sepolia testnet and local Anvil deployments.

- **FR-021**: System MUST automatically accept pending ownership transfers during deployment for contracts using Ownable2Step pattern (`acceptOwnership()`) and governance-controlled contracts (`governanceAcceptOwner()`). The list of affected contracts varies by protocol version and MUST be determined through research for each supported version (see research.md Section 6 for version-specific lists).

- **FR-022**: System MUST support automatic wallet funding where user provides a funded "funder" wallet private key, and the system automatically funds ecosystem wallets (deployer, governor, operator) with required ETH (and CGT when custom base token is configured) before operations.

- **FR-023**: System MUST verify sufficient balance (ETH, and CGT when custom base token is configured) in funder wallet before starting deployment operations and report required amounts if insufficient.

- **FR-024**: System MUST allow configuration of CGT (Custom Gas Token) contract address for automatic funding operations when chain uses custom base token (base token address != `0x0000000000000000000000000000000000000001`).

- **FR-025**: System MUST require Docker to be installed and running on the host machine.

- **FR-026**: System MUST automatically pull toolkit images from the configured registry when they are not available locally.

- **FR-027**: System MUST use ephemeral containers (create, run, remove) for each operation to ensure clean state.

- **FR-028**: System MUST stream container output to the terminal in real-time during operations.

### Key Entities

- **Ecosystem**: Top-level entity containing multiple chains, identified by name. Stores bridgehub, governance, and shared infrastructure contract addresses.

- **Chain**: A ZkSync rollup within an ecosystem, identified by chain name and chain ID. Contains diamond proxy, chain admin, and chain-specific contract addresses.

- **Wallet**: A keypair (address + private key) used for deployment operations. Types include: deployer, governor, operator, prove_operator, execute_operator, funder (for automatic funding of other wallets).

- **Contract Deployment**: A record of deployed contract including address, transaction hash, and deployment parameters.

- **State Backend**: Abstract storage interface for persisting ecosystem, chain, and contract data with get/set/delete operations on string keys and byte values.

- **Upgrade**: A protocol version transition containing source version, target version, calldata for execution, and verification requirements.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Operators can initialize and deploy a complete ecosystem with one chain to local Anvil in under 10 minutes.

- **SC-002**: Operators can upgrade ecosystem and chain contracts between supported versions without manual calldata encoding.

- **SC-003**: All operations documented in deployment guide (v29_ecosystem_deployment.md) can be performed through CLI commands.

- **SC-004**: All operations documented in upgrade guides (v29_v30, v30_v30.1) can be performed through CLI commands.

- **SC-005**: State persisted to filesystem survives Docker container restarts and can be shared between container instances.

- **SC-006**: SDK library crates can be imported and used independently of CLI for programmatic ecosystem management.

- **SC-007**: Docker images build successfully via Docker Bake with configurable parameters for different deployment scenarios.

- **SC-008**: CLI provides clear error messages with actionable remediation steps for common failure scenarios (insufficient funds, missing dependencies, invalid configuration).

## Assumptions

- Users have Docker installed and running on their host machine (Docker Desktop or Docker Engine).
- Settlement layer RPC endpoints are available and have appropriate rate limits for deployment operations.
- Users understand basic ZkSync ecosystem concepts (bridgehub, chain admin, governance).
- The reference ecosystem directory structure from dry_run_ecosystem is the canonical format.
- zkstack CLI and foundry-zksync tooling may have breaking changes; specific commits/versions are pinned to ensure reproducibility.
- Deployment and upgrade processes follow the documented guides without significant deviation.

## Out of Scope

- Server/sequencer deployment and management
- Prover deployment and management
- Block explorer setup
- Monitoring and observability configuration
- Multi-signature wallet integration for governance
- Cross-ecosystem operations
