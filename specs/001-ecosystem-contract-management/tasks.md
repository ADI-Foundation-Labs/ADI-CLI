# Tasks: Ecosystem Contract Management CLI

**Input**: Design documents from `/specs/001-ecosystem-contract-management/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli-commands.md

**Tests**: Not explicitly requested in feature specification. Tests can be added incrementally.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure:
- `src/` - Source code
- `src/commands/` - Command implementations
- `src/state/` - State backend
- `src/ecosystem/` - Ecosystem domain logic
- `src/chain/` - Chain domain logic
- `src/external/` - External tool wrappers
- `src/funding/` - Wallet funding logic
- `docker/` - Docker files
- `tests/` - Test directories

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Add alloy and crypto dependencies (alloy-primitives, alloy-signer, alloy-provider, semver, secrecy, uuid, async-trait) to Cargo.toml
- [ ] T002 [P] Create src/state/mod.rs with StateBackend trait definition
- [ ] T003 [P] Create src/ecosystem/mod.rs module structure
- [ ] T004 [P] Create src/chain/mod.rs module structure
- [ ] T005 [P] Create src/external/mod.rs module structure
- [ ] T006 [P] Create src/funding/mod.rs module structure

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

### Data Model Types

- [ ] T007 [P] Create Wallet struct with Address and SecretString in src/ecosystem/wallets.rs
- [ ] T008 [P] Create SettlementNetwork enum (Mainnet, Sepolia, Localhost, Custom) in src/ecosystem/config.rs
- [ ] T009 [P] Create BaseToken enum (Eth, Custom) in src/chain/config.rs
- [ ] T010 [P] Create ProverMode enum (NoProofs, Gpu) in src/chain/config.rs
- [ ] T011 [P] Create ChainState enum in src/chain/config.rs
- [ ] T012 Implement EcosystemContracts struct with alloy Address types in src/ecosystem/contracts.rs
- [ ] T013 Implement EcosystemWallets struct in src/ecosystem/wallets.rs
- [ ] T014 Implement Ecosystem struct with validation in src/ecosystem/config.rs
- [ ] T015 [P] Implement ChainContracts struct in src/chain/contracts.rs
- [ ] T016 [P] Implement ChainWallets struct in src/chain/wallets.rs
- [ ] T017 Implement Chain struct with validation in src/chain/config.rs
- [ ] T018 [P] Create protocol version utilities (version_to_hex, hex_to_version) in src/ecosystem/mod.rs

### State Backend

- [ ] T019 Implement FilesystemBackend struct in src/state/filesystem.rs
- [ ] T020 Implement StateBackend trait for FilesystemBackend (get, set, delete, list_keys, exists) in src/state/filesystem.rs
- [ ] T021 Add atomic write support (temp file + rename) in src/state/filesystem.rs
- [ ] T022 Export FilesystemBackend from src/state/mod.rs

### External Tool Wrappers

- [ ] T023 [P] Create ZkstackCli struct with async command execution in src/external/zkstack.rs
- [ ] T024 [P] Create ForgeCli struct with script execution support in src/external/forge.rs
- [ ] T025 [P] Create CastCli struct with call/send/calldata methods in src/external/cast.rs
- [ ] T026 Implement version checking for all external tools in src/external/mod.rs
- [ ] T027 Export all external tool wrappers from src/external/mod.rs

### Configuration Enhancement

- [ ] T028 Add SettlementConfig struct (rpc_url, gas_price) to src/config.rs
- [ ] T029 Add FunderConfig struct (private_key, cgt_address: Option) to src/config.rs
- [ ] T030 Add EcosystemConfig struct (name, chain_name, chain_id) to src/config.rs
- [ ] T031 Add DockerConfig struct (zksync_era_commit, era_contracts_tag, foundry_zksync_version) to src/config.rs
- [ ] T032 Update Config struct to include settlement, funder, ecosystem, docker fields in src/config.rs
- [ ] T033 Add environment variable mappings for new config fields (ADI_SETTLEMENT_RPC_URL, ADI_FUNDER_PRIVATE_KEY, etc.) in src/config.rs

### CLI Base Structure

- [ ] T034 Create src/commands/init/mod.rs with Init subcommand enum
- [ ] T035 Create src/commands/deploy/mod.rs with Deploy subcommand enum
- [ ] T036 Create src/commands/upgrade/mod.rs with Upgrade subcommand enum
- [ ] T037 Register all new command modules in src/commands/mod.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Initialize New Ecosystem (Priority: P1) MVP

**Goal**: Allow chain operators to initialize a new ZkSync ecosystem configuration from scratch

**Independent Test**: Run `adi init ecosystem` in Docker container, verify ZkStack.yaml, wallets.yaml, and contracts.yaml are generated in state directory

### Implementation for User Story 1

- [ ] T040 [US1] Implement wallet generation using alloy-signer in src/ecosystem/wallets.rs
- [ ] T041 [US1] Implement ecosystem directory structure creation in src/ecosystem/config.rs
- [ ] T042 [US1] Create InitEcosystem command struct with Clap options in src/commands/init/ecosystem.rs
- [ ] T043 [US1] Implement zkstack ecosystem create wrapper in src/external/zkstack.rs
- [ ] T044 [US1] Implement InitEcosystem::run() - parse options, create ecosystem, generate wallets in src/commands/init/ecosystem.rs
- [ ] T045 [US1] Add ecosystem state persistence (save metadata, wallets, contracts to filesystem) in src/commands/init/ecosystem.rs
- [ ] T046 [US1] Add colored output for init ecosystem progress and success in src/commands/init/ecosystem.rs
- [ ] T047 [US1] Register InitEcosystem in src/commands/init/mod.rs

**Checkpoint**: User Story 1 complete - can initialize ecosystem and verify generated files

---

## Phase 4: User Story 2 - Deploy Ecosystem Contracts (Priority: P1)

**Goal**: Deploy ecosystem smart contracts to settlement layer so chain infrastructure is established

**Independent Test**: Run `adi deploy ecosystem` against local Anvil, verify bridgehub, governance, and verifier contracts deployed with addresses in contracts.yaml

### Implementation for User Story 2

- [ ] T048 [US2] Implement balance checking for ETH using cast in src/funding/transfer.rs
- [ ] T049 [US2] Implement balance checking for ERC-20 tokens using cast in src/funding/transfer.rs
- [ ] T050 [US2] Implement ETH transfer using cast send in src/funding/transfer.rs
- [ ] T051 [US2] Implement ERC-20 transfer using cast send in src/funding/transfer.rs
- [ ] T052 [US2] Implement fund_wallets() with pre-flight balance validation in src/funding/mod.rs
- [ ] T053 [US2] Create DeployEcosystem command struct with Clap options in src/commands/deploy/ecosystem.rs
- [ ] T054 [US2] Implement zkstack ecosystem init wrapper for contract deployment in src/external/zkstack.rs
- [ ] T055 [US2] Implement contract address parsing from zkstack output YAML in src/ecosystem/contracts.rs
- [ ] T056 [US2] Implement DeployEcosystem::run() - check balances, auto-fund, deploy, persist addresses in src/commands/deploy/ecosystem.rs
- [ ] T057 [US2] Add progress output for multi-step deployment in src/commands/deploy/ecosystem.rs
- [ ] T058 [US2] Add deployment error handling with actionable resolution guidance in src/commands/deploy/ecosystem.rs
- [ ] T059 [US2] Register DeployEcosystem in src/commands/deploy/mod.rs

**Checkpoint**: User Stories 1 AND 2 complete - can initialize and deploy ecosystem

---

## Phase 5: User Story 3 - Initialize Chain Configuration (Priority: P1)

**Goal**: Initialize a new chain configuration within an ecosystem

**Independent Test**: Run `adi init chain` after ecosystem initialization, verify chain directory structure and config files (wallets.yaml, genesis.yaml) are created

### Implementation for User Story 3

- [ ] T060 [US3] Implement chain wallet generation in src/chain/wallets.rs
- [ ] T061 [US3] Implement chain directory structure creation in src/chain/config.rs
- [ ] T062 [US3] Create InitChain command struct with Clap options (NO deployment options) in src/commands/init/chain.rs
- [ ] T063 [US3] Implement InitChain::run() - validate ecosystem, create chain config in src/commands/init/chain.rs
- [ ] T064 [US3] Add chain state persistence (wallets.yaml, genesis.yaml) in src/commands/init/chain.rs
- [ ] T065 [US3] Register InitChain in src/commands/init/mod.rs

**Checkpoint**: User Story 3 complete - can initialize chain configuration

---

## Phase 5b: User Story 3b - Deploy Chain Contracts (Priority: P1)

**Goal**: Deploy chain contracts to settlement layer and register with Bridgehub

**Independent Test**: Run `adi deploy chain` after ecosystem deployment, verify chain contracts deployed and chain registered with bridgehub

### Implementation for User Story 3b

- [ ] T066 [US3b] Create DeployChain command struct with Clap options in src/commands/deploy/chain.rs
- [ ] T067 [US3b] Implement zkstack chain create/register wrapper in src/external/zkstack.rs
- [ ] T068 [US3b] Implement chain contract address parsing from zkstack output in src/chain/contracts.rs
- [ ] T069 [US3b] Implement DeployChain::run() - check balances, auto-fund, deploy contracts, register with bridgehub in src/commands/deploy/chain.rs
- [ ] T070 [US3b] Add deployment progress output for chain contracts in src/commands/deploy/chain.rs
- [ ] T071 [US3b] Add deployment error handling with actionable guidance in src/commands/deploy/chain.rs
- [ ] T072 [US3b] Add chain contract state persistence (contracts.yaml) in src/commands/deploy/chain.rs
- [ ] T073 [US3b] Register DeployChain in src/commands/deploy/mod.rs

**Checkpoint**: User Stories 3 AND 3b complete - can initialize and deploy chain

---

## Phase 6: User Story 4 - Verify Dependency Availability (Priority: P2)

**Goal**: Verify required external tools (zkstack, forge, cast) are available before operations

**Independent Test**: Run `adi doctor`, verify status report for zkstack, forge, cast, and configuration

### Implementation for User Story 4

- [ ] T074 [US4] Create Doctor command struct with --json option in src/commands/doctor.rs
- [ ] T075 [US4] Implement dependency availability checking using which/command --version in src/commands/doctor.rs
- [ ] T076 [US4] Implement config file existence and state directory writability checks in src/commands/doctor.rs
- [ ] T077 [US4] Implement Doctor::run() with formatted colored output in src/commands/doctor.rs
- [ ] T078 [US4] Implement JSON output mode for doctor command in src/commands/doctor.rs
- [ ] T079 [US4] Add installation guidance for missing dependencies in src/commands/doctor.rs
- [ ] T080 [US4] Register Doctor in src/commands/mod.rs

**Checkpoint**: Can verify environment before operations

---

## Phase 7: User Story 5 - Upgrade Ecosystem Contracts (Priority: P2)

**Goal**: Upgrade ecosystem contracts to a new protocol version for new features and security fixes

**Independent Test**: Deploy v29 ecosystem, run `adi upgrade ecosystem --to v30`, verify upgrade calldata generated

### Implementation for User Story 5

- [ ] T081 [US5] Create Upgrade struct with status, calldata, versions in src/ecosystem/config.rs
- [ ] T082 [US5] Create UpgradeCalldata struct (schedule_transparent, execute, governance_address) in src/ecosystem/config.rs
- [ ] T083 [US5] Create UpgradeStatus enum (Prepared, Scheduled, Executed, Failed) in src/ecosystem/config.rs
- [ ] T084 [US5] Implement upgrade input TOML generation from ecosystem state in src/external/forge.rs
- [ ] T085 [US5] Implement forge script execution for upgrade simulation in src/external/forge.rs
- [ ] T086 [US5] Implement calldata extraction from forge script output in src/external/forge.rs
- [ ] T087 [US5] Create UpgradeEcosystem command struct with --to, --execute, --output-dir options in src/commands/upgrade/ecosystem.rs
- [ ] T088 [US5] Implement UpgradeEcosystem::run() - generate calldata, optionally execute in src/commands/upgrade/ecosystem.rs
- [ ] T089 [US5] Add calldata file output to upgrade-output directory in src/commands/upgrade/ecosystem.rs
- [ ] T090 [US5] Add execution instructions output for governance in src/commands/upgrade/ecosystem.rs
- [ ] T091 [US5] Register UpgradeEcosystem in src/commands/upgrade/mod.rs

**Checkpoint**: Can prepare ecosystem upgrades

---

## Phase 8: User Story 6 - Upgrade Chain Contracts (Priority: P2)

**Goal**: Upgrade chain contracts to match ecosystem protocol version

**Independent Test**: After ecosystem upgrade, run `adi upgrade chain --to v30`, verify chain upgrade calldata generated

### Implementation for User Story 6

- [ ] T092 [US6] Implement zkstack generate-chain-upgrade wrapper in src/external/zkstack.rs
- [ ] T093 [US6] Create UpgradeChain command struct with --to, --chain-name, --execute options in src/commands/upgrade/chain.rs
- [ ] T094 [US6] Implement UpgradeChain::run() - validate ecosystem version, generate chain calldata in src/commands/upgrade/chain.rs
- [ ] T095 [US6] Add DA validator pair update instructions in upgrade output in src/commands/upgrade/chain.rs
- [ ] T096 [US6] Register UpgradeChain in src/commands/upgrade/mod.rs

**Checkpoint**: Can prepare chain upgrades after ecosystem upgrades

---

## Phase 9: User Story 7 - Manage State Backend (Priority: P3)

**Goal**: Ensure state persistence works correctly across container restarts and mounts

**Independent Test**: Perform operations, exit container, mount same state, verify operations can continue

### Implementation for User Story 7

- [ ] T097 [US7] Implement state integrity validation on startup in src/state/filesystem.rs
- [ ] T098 [US7] Add state directory mount detection and warning in src/state/filesystem.rs
- [ ] T099 [US7] Add state backup before destructive operations in src/state/filesystem.rs
- [ ] T100 [US7] Document state directory structure in quickstart.md

**Checkpoint**: All user stories complete

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Docker support, automation, and final touches

### Docker Infrastructure

- [ ] T101 [P] Create docker/Dockerfile.deps with zkstack CLI and foundry-zksync installation
- [ ] T102 [P] Create docker/Dockerfile for CLI image on top of deps
- [ ] T103 Create docker/docker-bake.hcl with parameterized build configuration
- [ ] T104 Add Docker volume mount examples to quickstart.md

### Development Automation

- [ ] T105 Update Taskfile.yml with docker build targets (deps, cli, all)
- [ ] T106 [P] Add Taskfile task for running tests
- [ ] T107 [P] Add Taskfile task for local Anvil deployment testing

### Final Validation

- [ ] T108 Validate all commands work per quickstart.md local Anvil flow
- [ ] T109 Validate all commands work per quickstart.md Sepolia flow
- [ ] T110 Verify error messages include actionable remediation steps

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-9)**: All depend on Foundational phase completion
  - P1 stories (US1, US2, US3, US3b) should be done in order as they build on each other
  - P2 stories (US4, US5, US6) can start after P1 but US6 depends on US5
  - P3 story (US7) can proceed independently after Foundational
- **Polish (Phase 10)**: Can start after Foundational, full validation after all stories complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - No dependencies on other stories
- **User Story 2 (P1)**: Depends on US1 (needs initialized ecosystem to deploy)
- **User Story 3 (P1)**: Depends on US1 (needs initialized ecosystem for chain config)
- **User Story 3b (P1)**: Depends on US2 AND US3 (needs deployed ecosystem and initialized chain)
- **User Story 4 (P2)**: Can start after Foundational - Independent utility command
- **User Story 5 (P2)**: Can start after Foundational - Independent upgrade logic
- **User Story 6 (P2)**: Depends on US5 (chain upgrade follows ecosystem upgrade)
- **User Story 7 (P3)**: Can start after Foundational - State backend refinements

### Within Each User Story

- Data structures before commands
- External tool wrappers before command implementations
- Core implementation before error handling and output formatting
- Command registration after implementation complete

### Parallel Opportunities

**Phase 1 - All Setup tasks can run in parallel:**
```
T002, T003, T004, T005, T006
```

**Phase 2 - Data model types can run in parallel:**
```
T007, T008, T009, T010, T011
T012, T013 → T014 (Ecosystem depends on contracts/wallets)
T015, T016 → T017 (Chain depends on contracts/wallets)
```

**Phase 2 - External wrappers can run in parallel:**
```
T023, T024, T025 → T026, T027
```

**Phase 2 - Config structs can run in parallel:**
```
T028, T029, T030, T031 → T032, T033
```

**Phase 10 - Docker and Taskfile can run in parallel:**
```
T101, T102 → T103
T105, T106, T107
```

---

## Implementation Strategy

### MVP First (User Stories 1-3b Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Initialize Ecosystem)
4. Complete Phase 4: User Story 2 (Deploy Ecosystem)
5. Complete Phase 5: User Story 3 (Initialize Chain)
6. Complete Phase 5b: User Story 3b (Deploy Chain)
7. **STOP and VALIDATE**: Test full flow against local Anvil
8. Deploy/demo if ready - this is a functional MVP

### Incremental Delivery

1. **Foundation**: Setup + Foundational → Core infrastructure ready
2. **MVP Milestone**: US1 + US2 + US3 + US3b → Can deploy complete ecosystem with chain
3. **Operational Tooling**: US4 (doctor) → Better user experience
4. **Upgrade Support**: US5 + US6 → Can upgrade existing deployments
5. **Robustness**: US7 → Production-ready state management
6. **Containerization**: Phase 10 → Docker-ready distribution

### Critical Path

```
Setup → Foundational → US1 → US2 → US3 → US3b → MVP Complete
                    ↘ US4 (parallel)
                    ↘ US5 → US6
                    ↘ US7 (after Foundational)
```

---

## Notes

- [P] tasks = different files, no dependencies within same phase
- [Story] label maps task to specific user story for traceability
- P1 stories must complete in order (init ecosystem → deploy ecosystem → init chain → deploy chain)
- P2/P3 stories can proceed independently after their dependencies are met
- Strict Clippy lints enforced: no unwrap, no panic, no indexing
- All error handling via eyre::Result with wrap_err()
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
