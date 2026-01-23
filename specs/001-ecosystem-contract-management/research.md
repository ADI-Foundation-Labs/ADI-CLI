# Research: Ecosystem Contract Management CLI

**Branch**: `001-ecosystem-contract-management` | **Date**: 2026-01-22

## Summary

This document captures research findings for implementing the ADI CLI that automates ZkSync ecosystem contract management within Docker containers.

---

## 1. zkstack CLI Integration

### Decision
Wrap zkstack CLI commands via subprocess execution rather than implementing zkstack functionality directly.

### Rationale
- zkstack CLI is the canonical tool for ZkSync ecosystem management
- Direct subprocess calls maintain compatibility with upstream changes
- Reduces implementation complexity significantly
- Allows pinning to specific commits for reproducibility

### Alternatives Considered
- **Native Rust implementation**: Rejected due to complexity of replicating foundry/forge interactions
- **FFI bindings**: Rejected due to maintenance burden and version coupling

### Key zkstack Commands to Wrap
```bash
# Ecosystem creation
zkstack ecosystem create --zksync-os -v [options]

# Ecosystem initialization (deploys contracts)
zkstack ecosystem init --zksync-os -v --update-submodules false --ignore-prerequisites -a --with-gas-price [price]

# Chain upgrade calldata generation
zkstack dev generate-chain-upgrade --upgrade-version [version] [yaml] [chain-ids] [rpcs]
```

### Implementation Notes
- Use `tokio::process::Command` for async subprocess execution
- Capture stdout/stderr for logging and error handling
- Parse YAML output files for contract addresses
- Handle interactive prompts via `--yes` flags or scripted input

---

## 2. foundry-zksync (forge/cast) Integration

### Decision
Use foundry-zksync's `cast` and `forge` tools for direct contract interactions and calldata encoding.

### Rationale
- Cast provides reliable contract interaction primitives
- Forge scripts handle complex deployment orchestration
- Both are standard tools in ZkSync development workflow

### Key Cast Commands
```bash
# Read contract state
cast call [contract] "[function](args)(returns)" --rpc-url [rpc]

# Send transactions
cast send [contract] "[function](args)" [args] --private-key [pk] --rpc-url [rpc]

# Encode calldata
cast calldata "[function](args)" [args]

# Check balance
cast balance [address] --rpc-url [rpc]
```

### Key Forge Commands
```bash
# Run deployment scripts
forge script [script.sol:Contract] --ffi --rpc-url [rpc] --private-key [pk] --broadcast

# Build contracts
forge build
```

### Implementation Notes
- Wrap cast/forge commands in typed Rust functions
- Parse JSON/hex output for structured data
- Handle gas price estimation and transaction confirmation

---

## 3. Docker Architecture

### Decision
Two-layer Docker image structure with Docker Bake for parameterized builds.

### Rationale
- Separating dependencies from CLI allows faster rebuilds during development
- Docker Bake provides declarative, parameterized build configuration
- Version pinning via build args ensures reproducibility

### Image Structure
```dockerfile
# Image 1: Dependencies (adi-deps)
FROM rust:latest
# Install zkstack CLI from specific commit
# Install foundry-zksync from specific version
# Pre-compile dependencies

# Image 2: CLI (adi-cli)
FROM adi-deps
COPY . /app
RUN cargo build --release
```

### Docker Bake Configuration
```hcl
variable "ZKSYNC_ERA_COMMIT" { default = "7c4c428b1ea3fd75d9884f3e842fb12d847705c1" }
variable "ERA_CONTRACTS_TAG" { default = "zkos-v0.29.11" }
variable "FOUNDRY_ZKSYNC_VERSION" { default = "latest" }

target "deps" {
  dockerfile = "docker/Dockerfile.deps"
  args = {
    ZKSYNC_ERA_COMMIT = ZKSYNC_ERA_COMMIT
    ERA_CONTRACTS_TAG = ERA_CONTRACTS_TAG
    FOUNDRY_ZKSYNC_VERSION = FOUNDRY_ZKSYNC_VERSION
  }
}

target "cli" {
  dockerfile = "docker/Dockerfile"
  contexts = { deps = "target:deps" }
}
```

### Alternatives Considered
- **Single Dockerfile**: Rejected due to slow rebuilds
- **docker-compose only**: Rejected due to limited parameterization

---

## 4. State Backend Design

### Decision
Abstract trait-based state backend with filesystem implementation as default.

### Rationale
- Trait abstraction enables future database backends without code changes
- Filesystem backend is simple, portable, and debuggable
- Key-value interface maps naturally to ecosystem/chain configuration

### Trait Design
```rust
#[async_trait]
pub trait StateBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    async fn exists(&self, key: &str) -> Result<bool>;
}
```

### Key Structure
```
ecosystem/{name}/metadata.yaml
ecosystem/{name}/contracts.yaml
ecosystem/{name}/wallets.yaml
ecosystem/{name}/chains/{chain}/metadata.yaml
ecosystem/{name}/chains/{chain}/contracts.yaml
ecosystem/{name}/chains/{chain}/wallets.yaml
```

### Filesystem Implementation
- Store under configurable base path (default: `~/.adi_cli/state/`)
- YAML serialization for human-readable inspection
- Atomic writes via temp file + rename

### Alternatives Considered
- **SQLite**: Viable future option, more complex than needed initially
- **RocksDB**: Overkill for CLI state management

---

## 5. Wallet Funding Strategy

### Decision
Automatic funding via funder wallet before deployment operations.

### Rationale
- Deployment requires funded wallets; manual funding is error-prone
- Pre-flight balance checks prevent mid-deployment failures
- ERC-20 token funding (CGT) requires separate transfer logic when custom base token is configured

### Implementation
```rust
pub struct FundingConfig {
    pub funder_private_key: String,
    pub cgt_token_address: Option<String>,  // Only set when base token != ETH
    pub eth_amounts: HashMap<WalletRole, U256>,
    pub cgt_amounts: HashMap<WalletRole, U256>,  // Only used when base token != ETH
}

pub async fn fund_wallets(
    config: &FundingConfig,
    wallets: &[Wallet],
    rpc_url: &str,
) -> Result<()> {
    // 1. Check funder balance (ETH, and CGT if custom base token)
    // 2. Calculate total required
    // 3. Fail early if insufficient
    // 4. Transfer ETH to each wallet
    // 5. Transfer CGT to each wallet (if base token != ETH)
}
```

### Required Amounts (from deployment guide)

**Note:** CGT (Custom Gas Token) is only required when chain uses a custom base token (not ETH).
- Chain without CGT: Fund with ETH only
- Chain with CGT: Fund with ETH + CGT

| Wallet                 | ETH   | CGT*  |
| ---------------------- | ----- | ----- |
| Ecosystem Deployer     | 1 ETH | -     |
| Ecosystem Governor     | 1 ETH | 5 CGT |
| Chain Governor         | 1 ETH | 5 CGT |
| Chain Operator         | 5 ETH | -     |
| Chain Prove Operator   | 5 ETH | -     |
| Chain Execute Operator | 5 ETH | -     |

*CGT column only applies when base token address != `0x0000000000000000000000000000000000000001` (ETH)

---

## 6. Ownership Management

### Decision
Ownership acceptance is performed automatically as part of deployment commands.

### Rationale
- Deployment creates pending ownership transfers that must be accepted
- Automatic acceptance during deployment eliminates the need for a separate post-deployment step
- Simplifies the CLI by reducing the number of commands
- Supports both Ownable and Ownable2Step patterns

### Contracts Requiring Ownership Management
| Contract             | Pattern               | Accept Method             |
| -------------------- | --------------------- | ------------------------- |
| Server Notifier      | Ownable2Step          | `acceptOwnership()`       |
| Rollup DA Manager    | Governance-controlled | `governanceAcceptOwner()` |
| Validator Timelock   | Ownable2Step          | `acceptOwnership()`       |
| Verifier             | Ownable2Step          | `acceptOwnership()`       |
| Bridged Token Beacon | Ownable               | No accept needed          |
| Governance           | Ownable2Step          | `acceptOwnership()`       |
| Chain Admin          | Ownable2Step          | `acceptOwnership()`       |

### Implementation
Ownership acceptance is integrated into the deployment flow:
- `adi deploy ecosystem` - Accepts ownership for ecosystem-level contracts after deployment
- `adi deploy chain` - Accepts ownership for chain-level contracts after deployment

---

## 7. Upgrade Workflow

### Decision
Generate and output upgrade calldata for governance execution rather than direct execution.

### Rationale
- Upgrades require governance approval in production
- Outputting calldata allows verification before execution
- Supports both direct execution (devnet) and multisig (mainnet)

### Upgrade Steps Automated
1. Prepare upgrade input TOML from ecosystem state
2. Run forge script to simulate upgrade
3. Extract stage1_calls from generated output
4. Encode scheduleTransparent and execute calldata
5. Output calldata for governance execution

### Implementation
```rust
pub struct UpgradeOutput {
    pub schedule_transparent_calldata: Bytes,
    pub execute_calldata: Bytes,
    pub governance_address: Address,
    pub target_version: ProtocolVersion,
}

pub async fn prepare_ecosystem_upgrade(
    ecosystem: &Ecosystem,
    target_version: &str,
    rpc_url: &str,
) -> Result<UpgradeOutput> {
    // 1. Generate upgrade input TOML
    // 2. Run forge script (simulate)
    // 3. Parse output YAML/TOML
    // 4. Encode governance calldata
}
```

---

## 8. Error Handling Strategy

### Decision
Structured error types with actionable remediation guidance.

### Rationale
- CLI errors should guide users toward resolution
- Error context propagation via `wrap_err()` per constitution
- Different error types for different failure modes

### Error Categories
```rust
pub enum CliError {
    Config(ConfigError),          // Configuration issues
    Dependency(DependencyError),  // Missing zkstack/forge/cast
    Network(NetworkError),        // RPC connectivity issues
    Balance(BalanceError),        // Insufficient funds
    Contract(ContractError),      // Contract interaction failures
    State(StateError),            // State backend errors
    Upgrade(UpgradeError),        // Upgrade-specific errors
}
```

### Error Message Format
```
Error: Failed to deploy ecosystem contracts

Cause: Insufficient ETH balance in deployer wallet

Details:
  - Wallet: 0x1234...5678
  - Required: 1.5 ETH
  - Available: 0.3 ETH

Resolution:
  1. Fund the deployer wallet with at least 1.2 ETH more
  2. Or configure a funder wallet in ~/.adi_cli/.adi.yml
  3. Re-run: adi deploy ecosystem
```

---

## 9. Configuration Schema

### Decision
YAML configuration with environment variable overrides following ADI_ prefix convention.

### Rationale
- YAML is human-readable and widely understood
- Environment variables enable container deployment flexibility
- CLI flags provide ultimate override capability

### Configuration Schema
```yaml
# ~/.adi_cli/.adi.yml
state_dir: ~/.adi_cli/state

settlement:
  rpc_url: http://localhost:8545
  gas_price: 10000000000  # 10 gwei

funder:
  private_key: "0x..."
  cgt_address: "0x2a98B46fe31BA8Be05ef1cE3D36e1f80Db04190D"  # Optional: only needed when base token != ETH

ecosystem:
  name: adi_ecosystem
  chain_name: adi
  chain_id: 222

docker:
  zksync_era_commit: 7c4c428b1ea3fd75d9884f3e842fb12d847705c1
  era_contracts_tag: zkos-v0.29.11
```

### Environment Variable Mapping
```bash
ADI_STATE_DIR=~/.adi_cli/state
ADI_SETTLEMENT_RPC_URL=http://localhost:8545
ADI_SETTLEMENT_GAS_PRICE=10000000000
ADI_FUNDER_PRIVATE_KEY=0x...
ADI_FUNDER_CGT_ADDRESS=0x...  # Optional: only needed when base token != ETH
ADI_ECOSYSTEM_NAME=adi_ecosystem
ADI_ECOSYSTEM_CHAIN_NAME=adi
ADI_ECOSYSTEM_CHAIN_ID=222
```

---

## 10. Version Compatibility Matrix

### Decision
Document and enforce version compatibility between components.

### Rationale
- ZkSync ecosystem has strict version dependencies
- Mismatched versions cause subtle failures
- Version tracking enables upgrade path validation

### Compatibility Matrix (v29)
| Component        | Version               | Commit/Tag                               |
| ---------------- | --------------------- | ---------------------------------------- |
| zkstack CLI      | main                  | 7c4c428b1ea3fd75d9884f3e842fb12d847705c1 |
| zksync-era       | zksync-os-integration | a135c3b09913d49a1323b44ab80e715616934fc7 |
| era-contracts    | v0.29.11              | zkos-v0.29.11                            |
| genesis.json     | v4                    | ec996154d7cb0f3bd2857ff015d061781a9fbbe6 |
| zkSync OS Server | v0.10.1               | -                                        |

### Compatibility Matrix (v30)
| Component        | Version    | Commit/Tag                               |
| ---------------- | ---------- | ---------------------------------------- |
| zkstack CLI      | v30 branch | a48fd5f99a3fad0542b514fc9c508094230b35f4 |
| era-contracts    | v0.30.0    | v30-zksync-os-upgrade branch             |
| zkSync OS Server | v0.12.0    | -                                        |

---

## Open Questions Resolved

1. **How to handle zkstack interactive prompts?**
   - Use `--yes` flag and provide all parameters via CLI flags
   - zkstack supports non-interactive mode for automation

2. **Where should upgrade calldata be generated?**
   - Generated inside Docker container with era-contracts
   - Output as files for governance to execute

3. **How to verify deployment success?**
   - Check contract addresses exist in contracts.yaml
   - Verify contracts are deployed via `cast call`
   - Validate protocol version matches expected

4. **How to handle gas price volatility?**
   - Accept `--gas-price` flag for manual override
   - Use safe default (10 gwei) for local/testnet
   - Document checking Etherscan for mainnet/Sepolia

5. **How to manage multiple ecosystems?**
   - State backend keys prefixed with ecosystem name
   - Config file supports ecosystem.name setting
   - CLI accepts `--ecosystem-name` override
