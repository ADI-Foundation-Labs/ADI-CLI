# Feature Specification: Ecosystem Contract Management CLI

**Feature Branch**: `001-ecosystem-contract-management`
**Created**: 2026-01-22
**Status**: Draft
**Input**: User description: SDK-first Rust CLI for managing ZkSync ecosystem smart contracts within Docker containers, featuring abstract state backends, automation of chain deployments, and contract upgrade operations.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Initialize New Ecosystem (Priority: P1)

As a chain operator, I want to initialize a new ZkSync ecosystem configuration from scratch so that I can deploy smart contracts to L1 and create the foundation for my chain.

**Why this priority**: This is the foundational operation that must occur before any other ecosystem management. Without ecosystem initialization, no other operations are possible.

**Independent Test**: Can be fully tested by running the init command in a Docker container with mounted volumes, verifying that all configuration files (ZkStack.yaml, wallets.yaml, contracts.yaml) are generated in the state directory.


**Acceptance Scenarios**:

1. **Given** a Docker container with required dependencies (zkstack CLI, foundry-zksync), **When** user runs `adi init ecosystem` with ecosystem name and L1 network configuration, **Then** the system creates the ecosystem directory structure with ZkStack.yaml, configs directory, and chains directory.

2. **Given** user provides a config file with private keys, **When** ecosystem init is executed, **Then** the system uses provided keys for deployer and governor wallets instead of generating random ones.

3. **Given** no existing state, **When** user runs `adi init ecosystem --chain-name mychain --chain-id 222`, **Then** a chain configuration is created with proper wallet files (wallets.yaml containing deployer, operator, governor keys).

4. **Given** ecosystem init completes successfully, **When** user mounts the output directory, **Then** all generated files are accessible from the host machine.

---

### User Story 2 - Deploy Ecosystem Contracts (Priority: P1)

As a chain operator, I want to deploy ecosystem smart contracts to L1 so that my chain infrastructure is established on the settlement layer.

**Why this priority**: Contract deployment is required immediately after initialization to make the ecosystem operational. This is core functionality.

**Independent Test**: Can be tested by running deploy command against a local Anvil node, verifying that bridgehub, state transition manager, and governance contracts are deployed and addresses stored in contracts.yaml.

**Acceptance Scenarios**:

1. **Given** an initialized ecosystem with funded wallets, **When** user runs `adi deploy ecosystem`, **Then** ecosystem contracts are deployed and contract addresses are persisted to state.

2. **Given** deployment is successful, **When** checking contracts.yaml, **Then** it contains bridgehub_proxy_addr, governance, chain_admin, validator_timelock_addr, and all other ecosystem contract addresses.

3. **Given** deployment fails mid-way, **When** user re-runs deploy command, **Then** the system resumes from the last successful step or provides clear instructions for recovery.

4. **Given** custom gas price requirements, **When** user provides `--gas-price` flag, **Then** deployment uses the specified gas price for all transactions.

5. **Given** a funder wallet is configured with sufficient ETH and ADI tokens, **When** user runs deployment, **Then** ecosystem wallets are automatically funded before contract deployment begins.

6. **Given** a funder wallet has insufficient balance, **When** user runs deployment, **Then** system reports required amounts (ETH and ADI tokens) and halts before any transactions.

---

### User Story 3 - Initialize and Register Chain (Priority: P1)

As a chain operator, I want to initialize and register a new chain within my ecosystem so that I can operate a ZkSync rollup.

**Why this priority**: Chain registration is essential for having an operational L2. Without a registered chain, the ecosystem cannot process transactions.

**Independent Test**: Can be tested by running chain init after ecosystem deployment, verifying chain contracts are deployed and chain is registered with bridgehub.

**Acceptance Scenarios**:

1. **Given** a deployed ecosystem, **When** user runs `adi init chain --name adi --chain-id 222`, **Then** chain-specific contracts are deployed and chain is registered with the bridgehub.

2. **Given** chain initialization succeeds, **When** checking chain contracts.yaml, **Then** it contains diamond_proxy_addr, governance_addr, chain_admin_addr, and all L1/L2 contract addresses.

3. **Given** a base token configuration, **When** user specifies `--base-token-address <ADDRESS>`, **Then** the chain is configured with the custom base token instead of ETH.

---

### User Story 4 - Verify Dependency Availability (Priority: P2)

As a chain operator running the CLI in Docker, I want the system to verify that required external tools are available so that I can troubleshoot missing dependencies before starting operations.

**Why this priority**: Dependency verification prevents cryptic failures during operations and improves user experience.

**Independent Test**: Can be tested by running `adi doctor` command and verifying it reports status of zkstack CLI, foundry (forge/cast), and required environment configurations.

**Acceptance Scenarios**:

1. **Given** CLI is executed in Docker, **When** user runs `adi doctor`, **Then** system checks for zkstack CLI, forge, cast, and reports availability status for each.

2. **Given** a dependency is missing, **When** user runs `adi doctor`, **Then** system reports which dependency is missing and does NOT attempt to install it.

3. **Given** all dependencies are available, **When** user runs any command, **Then** operations proceed without dependency-related failures.

---

### User Story 5 - Upgrade Ecosystem Contracts (Priority: P2)

As a chain operator, I want to upgrade ecosystem contracts to a new protocol version so that I can benefit from new features and security fixes.

**Why this priority**: Upgrades are critical for long-term operation but not required for initial deployment.

**Independent Test**: Can be tested by deploying v29 ecosystem, then running upgrade command to v30, verifying protocol version changes and new contracts are deployed.

**Acceptance Scenarios**:

1. **Given** an ecosystem running protocol version v29, **When** user runs `adi upgrade ecosystem --to v30`, **Then** upgrade preparation is performed and calldata for governance execution is generated.

2. **Given** upgrade calldata is generated, **When** system outputs the scheduleTransparent and execute calldata, **Then** operator can execute these via governance contract.

3. **Given** upgrade is executed successfully, **When** checking protocol version, **Then** ecosystem reports new protocol version.

---

### User Story 6 - Upgrade Chain Contracts (Priority: P2)

As a chain operator, I want to upgrade my chain's contracts to match the ecosystem protocol version so that my chain can use new features.

**Why this priority**: Chain upgrades must follow ecosystem upgrades and are essential for continued operation.

**Independent Test**: Can be tested by upgrading chain after ecosystem upgrade, verifying diamond proxy reports new protocol version.

**Acceptance Scenarios**:

1. **Given** ecosystem is upgraded to v30, **When** user runs `adi upgrade chain --to v30`, **Then** chain upgrade calldata is generated for chain admin execution.

2. **Given** chain upgrade calldata is executed, **When** checking diamond proxy protocol version, **Then** it matches the target version.

3. **Given** upgrade requires DA validator pair update, **When** upgrade completes, **Then** system provides instructions or performs setDAValidatorPair call.

---

### User Story 7 - Accept Pending Ownership (Priority: P3)

As a chain operator, I want to accept pending ownership transfers after deployment so that governance is properly established.

**Why this priority**: Ownership acceptance is a post-deployment cleanup task that can be done after core functionality works.

**Independent Test**: Can be tested by running ownership acceptance commands after deployment, verifying ownership is transferred.

**Acceptance Scenarios**:

1. **Given** deployment completed with pending ownership, **When** user runs `adi accept ownership`, **Then** ownership is accepted for server notifier, rollup DA manager, validator timelock, and verifier.

2. **Given** custom ownership transfer is needed, **When** user runs `adi transfer ownership --to <ADDRESS>`, **Then** ownership transfer is initiated to the specified address.

---

### User Story 8 - Register Verifier for Execution Version (Priority: P3)

As a chain operator, I want to register verifiers for specific execution versions so that proofs can be verified correctly.

**Why this priority**: Verifier registration is a specific post-deployment task for chains using real proofs.

**Independent Test**: Can be tested by registering verifier for execution version 4 or 5, verifying verifier is registered in DualVerifier.

**Acceptance Scenarios**:

1. **Given** deployed ecosystem with DualVerifier, **When** user runs `adi register verifier --version 4`, **Then** Plonk and Fflonk verifiers are registered for execution version 4.

---

### User Story 9 - Manage State Backend (Priority: P3)

As a chain operator, I want to use different state backends so that I can persist ecosystem state in various storage systems.

**Why this priority**: Flexible state backends enable future extensibility but filesystem is sufficient for initial release.

**Independent Test**: Can be tested by configuring filesystem backend and verifying all operations read/write state correctly.

**Acceptance Scenarios**:

1. **Given** filesystem state backend is configured, **When** user performs any operation, **Then** state is persisted to the configured directory path.

2. **Given** state directory is mounted from host, **When** container exits, **Then** all state is preserved and accessible on host.

3. **Given** state exists from previous session, **When** new container mounts same state, **Then** operations can continue from previous state.

---

### Edge Cases

- What happens when L1 RPC is unreachable during deployment? System should provide clear error with retry guidance.
- What happens when funder wallet has insufficient funds? System should check ETH and ADI token balances before deploying and report required amounts for each.
- What happens when gas price changes significantly during multi-transaction deployment? System should handle transaction failures gracefully.
- What happens when user tries to upgrade to an unsupported version? System should validate target version before proceeding.
- What happens when state directory has corrupted files? System should validate state integrity on startup.
- What happens when Docker container runs without required mounts? System should detect and report missing mounts clearly.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST be architected with SDK-first approach where core logic resides in reusable library crates, with CLI being a thin wrapper.

- **FR-002**: System MUST operate within Docker containers without installing dependencies (only verifying their existence).

- **FR-003**: System MUST support two Docker files (so that we won't build first image with the dependencies each time, but only docker image with the CLI above the first image as a source): (1) dependencies image with zkstack CLI and foundry-zksync, (2) CLI image with this tool.

- **FR-004**: System MUST use Docker Bake for building parameterized container images:
Commits:
- smart contracts (zksync-era)
- os-integration
- Zk-os
- Genesis.json
-  (Install `foundryup-zksync` (to get the `cast` tool): https://foundry-book.zksync.io/introduction/installation/)

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

- **FR-012**: System MUST support mounting state and config directories when running in Docker.

- **FR-013**: System MUST support creating new ecosystem state from scratch and outputting it to host machine.

- **FR-014**: System MUST automate the manual processes documented in deployment and upgrade guides.

- **FR-015**: System MUST verify external dependency availability (zkstack, forge, cast) before operations with the fixed version.

- **FR-016**: System MUST support configuring L1 RPC URL for contract interactions.

- **FR-017**: System MUST follow config-file-first approach where configuration values (L1 RPC URL, state directory paths, private keys, ADI token address, chain name/ID, gas price) are read from config file by default, with optional CLI flags for overrides. Action-specific parameters like `--to <version>` for upgrades remain required flags.

- **FR-018**: System MUST persist all contract addresses and deployment state to state backend.

- **FR-019**: System MUST generate correct calldata for governance operations (scheduleTransparent, execute).

- **FR-020**: System MUST support both Sepolia testnet and local Anvil deployments.

- **FR-021**: System MUST handle ownership acceptance and transfer operations post-deployment.

- **FR-022**: System MUST support automatic wallet funding where user provides a funded "funder" wallet private key, and the system automatically funds ecosystem wallets (deployer, governor, operator) with required ETH and ADI tokens before operations.

- **FR-023**: System MUST verify sufficient balance (ETH and ADI tokens) in funder wallet before starting deployment operations and report required amounts if insufficient.

- **FR-024**: System MUST allow configuration of ADI token contract address for automatic funding operations.

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

- Users have access to Docker and can run containers.
- L1 RPC endpoints are available and have appropriate rate limits for deployment operations.
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
