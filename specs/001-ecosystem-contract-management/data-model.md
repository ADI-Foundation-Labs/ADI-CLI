# Data Model: Ecosystem Contract Management CLI

**Branch**: `001-ecosystem-contract-management` | **Date**: 2026-01-22

## Overview

This document defines the data entities, relationships, validation rules, and state transitions for the ADI CLI ecosystem contract management system.

## Dependencies for Types

The data model leverages existing crates for standard types:

```toml
# Cargo.toml additions
semver = "1"                    # Protocol version handling
alloy-primitives = "0.8"        # Address, B256, U256, Bytes
alloy-signer = "0.8"            # Wallet signing
alloy-provider = "0.8"          # RPC provider
secrecy = "0.8"                 # Secret string handling
```

---

## Core Entities

### 1. Ecosystem

Top-level container for ZkSync infrastructure. One ecosystem can contain multiple chains.

```rust
use alloy_primitives::{Address, B256};
use semver::Version;

pub struct Ecosystem {
    pub name: String,
    pub settlement_network: SettlementNetwork,
    pub state_path: PathBuf,
    pub contracts: EcosystemContracts,
    pub wallets: EcosystemWallets,
    pub chains: Vec<String>,  // Chain names
    pub protocol_version: Version,  // Using semver crate
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum SettlementNetwork {
    Mainnet,
    Sepolia,
    Localhost,
    Custom { rpc_url: String, chain_id: u64 },
}
```

**Protocol Version Notes:**
- Using `semver::Version` from the semver crate
- ZkSync versions like v29.0.11 map to `Version::new(29, 0, 11)`
- Hex encoding for on-chain: `((major << 32) | (minor << 24) | patch)`

**Validation Rules:**
- `name`: Non-empty, alphanumeric with underscores, max 64 chars
- `settlement_network`: Must be a valid supported network
- `state_path`: Must be writable directory
- `protocol_version`: Must be supported version (v29.x, v30.x)

**State Transitions:**
- `Uninitialized` → `Initialized` (via `adi init ecosystem`)
- `Initialized` → `Deployed` (via `adi deploy ecosystem`)
- `Deployed` → `Upgraded` (via `adi upgrade ecosystem`)

---

### 2. EcosystemContracts

Contract addresses deployed at ecosystem level on the settlement layer. Uses `alloy_primitives::Address` for type safety.

```rust
use alloy_primitives::{Address, B256};

pub struct EcosystemContracts {
    // Core infrastructure
    pub bridgehub_proxy_addr: Address,
    pub state_transition_proxy_addr: Address,
    pub governance_addr: Address,
    pub chain_admin_addr: Address,

    // Verifiers
    pub verifier_addr: Address,
    pub verifier_fflonk_addr: Option<Address>,
    pub verifier_plonk_addr: Option<Address>,

    // DA infrastructure
    pub l1_rollup_da_manager: Address,
    pub rollup_l1_da_validator: Address,

    // Token infrastructure
    pub native_token_vault_addr: Address,
    pub l1_nullifier_addr: Address,
    pub l1_asset_router: Address,

    // Timelock
    pub validator_timelock_addr: Address,

    // Server
    pub server_notifier_proxy_addr: Address,

    // Factory
    pub create2_factory_addr: Address,
    pub create2_factory_salt: B256,  // 32-byte hash
}
```

**Validation Rules:**
- All addresses must be non-zero (checked by `Address::is_zero()`)
- `bridgehub_proxy_addr` must be a deployed contract
- `governance_addr` must implement IGovernance interface

---

### 3. EcosystemWallets

Wallet keypairs used for ecosystem operations.

```rust
use alloy_primitives::Address;
use secrecy::SecretString;

pub struct EcosystemWallets {
    pub deployer: Wallet,
    pub governor: Wallet,
}

pub struct Wallet {
    pub address: Address,
    pub private_key: Option<SecretString>,  // Hidden from serialization via secrecy crate
}
```

**Validation Rules:**
- `address` must be valid (non-zero)
- `private_key` if provided, must derive to matching `address`
- Use `alloy-signer` for key derivation validation

**Security Notes:**
- Private keys wrapped in `SecretString` for zeroization on drop
- Private keys excluded from Debug/Display impls
- File permissions enforced at 0600

---

### 4. Chain

A ZkSync rollup within an ecosystem.

```rust
use alloy_primitives::Address;

pub struct Chain {
    pub name: String,
    pub chain_id: u64,
    pub ecosystem_name: String,
    pub base_token: BaseToken,
    pub prover_mode: ProverMode,
    pub contracts: ChainContracts,
    pub wallets: ChainWallets,
    pub state: ChainState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Chain's base token configuration (also known as Custom Gas Token / CGT)
/// - `Eth`: Default. Uses ETH as native token (address: 0x0000000000000000000000000000000000000001)
/// - `Custom`: Uses an ERC-20 from settlement layer as native token (requires CGT funding)
///
/// Native token behavior:
/// - L2 without CGT: ETH becomes native token
/// - L2 with CGT: ERC-20 from L1 becomes native token
/// - L3 without CGT: Settlement layer native token (e.g., ADI) becomes native token
pub enum BaseToken {
    Eth,  // Default: 0x0000000000000000000000000000000000000001
    Custom {
        address: Address,  // ERC-20 contract address on settlement layer
        symbol: String,
        decimals: u8,
    },
}

pub enum ProverMode {
    NoProofs,
    Gpu,
}

pub enum ChainState {
    Initialized,
    Deployed,  // Chain contracts deployed and registered with Bridgehub
    Running,
    Upgrading,
    Stopped,
}
```

**Validation Rules:**
- `name`: Non-empty, alphanumeric with underscores, max 64 chars
- `chain_id`: Positive integer, unique within ecosystem, not settlement layer chain IDs (1, 11155111)
- `base_token.address`: If custom (CGT), must be valid ERC-20 contract on settlement layer
- `prover_mode`: Must match genesis.json execution version

**Funding Requirements:**
- `BaseToken::Eth`: Fund wallets with ETH only
- `BaseToken::Custom`: Fund wallets with ETH + CGT (Custom Gas Token)

**State Transitions:**
- `Initialized` → `Deployed` (via `adi deploy chain`)
- `Deployed` → `Running` (external: server start)
- `Running` → `Upgrading` (via `adi upgrade chain`)
- `Upgrading` → `Running` (upgrade complete)
- Any → `Stopped` (external: server stop)

---

### 5. ChainContracts

Contract addresses for a specific chain.

```rust
use alloy_primitives::Address;

pub struct ChainContracts {
    // Diamond (main L2 contract on settlement layer)
    pub diamond_proxy_addr: Address,

    // Admin contracts
    pub governance_addr: Address,
    pub chain_admin_addr: Address,

    // Settlement layer contracts
    pub settlement_shared_bridge: Address,
    pub settlement_erc20_bridge: Address,

    // L2 contracts (deployed on L2)
    pub l2_shared_bridge: Address,
    pub l2_erc20_bridge: Address,
    pub l2_legacy_shared_bridge: Option<Address>,

    // Base token bridge (if custom token)
    pub base_token_bridge: Option<Address>,
}
```

---

### 6. ChainWallets

Wallet keypairs for chain operations.

```rust
pub struct ChainWallets {
    pub deployer: Wallet,
    pub governor: Wallet,
    pub operator: Wallet,
    pub prove_operator: Wallet,
    pub execute_operator: Wallet,
}
```

**Funding Requirements:**
| Role             | ETH Required | ADI Required |
| ---------------- | ------------ | ------------ |
| deployer         | 1 ETH        | -            |
| governor         | 1 ETH        | 5 ADI        |
| operator         | 5 ETH        | -            |
| prove_operator   | 5 ETH        | -            |
| execute_operator | 5 ETH        | -            |

---

### 7. ContractDeployment

Record of a deployed contract.

```rust
use alloy_primitives::{Address, B256, Bytes};

pub struct ContractDeployment {
    pub contract_name: String,
    pub address: Address,
    pub tx_hash: B256,
    pub block_number: u64,
    pub deployer: Address,
    pub constructor_args: Bytes,
    pub deployed_at: DateTime<Utc>,
    pub verified: bool,
}
```

---

### 8. Upgrade

Protocol version upgrade record.

```rust
use alloy_primitives::{Address, B256, Bytes};
use semver::Version;

pub struct Upgrade {
    pub id: Uuid,
    pub ecosystem_name: String,
    pub chain_name: Option<String>,  // None for ecosystem-level upgrade
    pub source_version: Version,
    pub target_version: Version,
    pub status: UpgradeStatus,
    pub calldata: UpgradeCalldata,
    pub executed_tx: Option<B256>,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
}

pub enum UpgradeStatus {
    Prepared,           // Calldata generated
    Scheduled,          // scheduleTransparent executed
    Executed,           // execute called
    Failed { reason: String },
}

pub struct UpgradeCalldata {
    pub schedule_transparent: Bytes,
    pub execute: Bytes,
    pub governance_address: Address,
}

/// Forge script output saved to v{VERSION}-ecosystem.toml or v{VERSION}-{chain-name}.toml
/// Contains:
/// - deployed_addresses: New contract addresses (facets, bridges, validators)
/// - contracts_config: Diamond cut data, protocol versions, init parameters
/// - governance_calls: stage0, stage1, stage2 encoded calls
/// - transactions: List of transaction hashes from deployment
///
/// This file is required as input for subsequent upgrades.
```

**State Transitions:**
- `Prepared` → `Scheduled` (scheduleTransparent tx confirmed)
- `Scheduled` → `Executed` (execute tx confirmed)
- Any → `Failed` (transaction reverts)

---

### 9. StateBackend

Abstract interface for state persistence.

```rust
#[async_trait]
pub trait StateBackend: Send + Sync {
    /// Retrieve value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Store value by key
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Delete value by key
    async fn delete(&self, key: &str) -> Result<()>;

    /// List keys with prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> Result<bool>;
}
```

**Key Hierarchy:**
```
ecosystems/{name}/metadata
ecosystems/{name}/contracts
ecosystems/{name}/wallets
ecosystems/{name}/chains/{chain_name}/metadata
ecosystems/{name}/chains/{chain_name}/contracts
ecosystems/{name}/chains/{chain_name}/wallets
ecosystems/{name}/upgrades/{upgrade_id}
ecosystems/{name}/deployments/{contract_name}
```

---

### 10. Config

Application configuration.

```rust
use alloy_primitives::Address;
use secrecy::SecretString;

pub struct Config {
    pub state_dir: PathBuf,
    pub settlement: SettlementConfig,
    pub funder: Option<FunderConfig>,
    pub ecosystem: EcosystemConfig,
    pub docker: DockerConfig,
}

pub struct SettlementConfig {
    pub rpc_url: String,
    pub gas_price: Option<u64>,  // In wei
}

pub struct FunderConfig {
    pub private_key: SecretString,
    pub cgt_address: Option<Address>,  // Only set when base token != ETH
}

pub struct EcosystemConfig {
    pub name: String,
    pub chain_name: String,
    pub chain_id: u64,
}

pub struct DockerConfig {
    pub zksync_era_commit: String,
    pub era_contracts_tag: String,
    pub foundry_zksync_version: String,
}
```

---

## Entity Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                         Ecosystem                                │
│  - name, settlement_network, protocol_version (semver::Version) │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐             │
│  │ EcosystemContracts  │    │  EcosystemWallets   │             │
│  │ - bridgehub         │    │  - deployer         │             │
│  │ - governance        │    │  - governor         │             │
│  │ - verifier          │    └─────────────────────┘             │
│  └─────────────────────┘                                        │
│  (Address = alloy_primitives::Address)                          │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Chain (1..n)                          │   │
│  │  - name, chain_id, base_token, prover_mode              │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │  ┌─────────────────────┐  ┌─────────────────────┐        │   │
│  │  │   ChainContracts    │  │    ChainWallets     │        │   │
│  │  │  - diamond_proxy    │  │  - governor         │        │   │
│  │  │  - chain_admin      │  │  - operator         │        │   │
│  │  │  - bridges          │  │  - prove_operator   │        │   │
│  │  └─────────────────────┘  │  - execute_operator │        │   │
│  │                           └─────────────────────┘        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────┐    ┌─────────────────────┐             │
│  │  Upgrade (0..n)     │    │ Deployment (0..n)   │             │
│  │  - source_version   │    │ - contract_name     │             │
│  │  - target_version   │    │ - address           │             │
│  │  - calldata (Bytes) │    │ - tx_hash (B256)    │             │
│  └─────────────────────┘    └─────────────────────┘             │
└─────────────────────────────────────────────────────────────────┘
```

---

## Serialization Formats

### YAML (Human-readable state files)

```yaml
# ecosystems/adi_ecosystem/metadata.yaml
name: adi_ecosystem
settlement_network: sepolia
protocol_version: "29.0.11"  # semver string format
chains:
  - adi
created_at: 2026-01-22T10:00:00Z
updated_at: 2026-01-22T12:30:00Z
```

```yaml
# ecosystems/adi_ecosystem/contracts.yaml
bridgehub_proxy_addr: "0xf69daaea7f8578933237a9b59f42704ebec36ab9"
governance_addr: "0x1234567890abcdef..."
verifier_addr: "0xabcdef1234567890..."
# ... more addresses
```

```yaml
# ecosystems/adi_ecosystem/wallets.yaml
deployer:
  address: "0x1111111111111111111111111111111111111111"
  # private_key stored separately with restricted permissions
governor:
  address: "0x2222222222222222222222222222222222222222"
```

### TOML (Upgrade configuration input)

```toml
# upgrade-input.toml
era_chain_id = 222
testnet_verifier = true
owner_address = "0xF6A96e4e5b602DDbf34E166729da97dbb2A3bEE2"
old_protocol_version = "0x1d00000000"
latest_protocol_version = "0x1e00000000"

[contracts]
bridgehub_proxy_address = "0xb339725f29090657f39df0c8c0c573f0856a45fe"
create2_factory_salt = "0x85de5677ffea74c9815331db7f5c737a33c161db4cae7d47504a336c4c5bcfdc"
```

---

## Protocol Version Utilities

```rust
use semver::Version;
use alloy_primitives::U256;

/// Convert semver Version to on-chain hex representation
pub fn version_to_hex(version: &Version) -> U256 {
    let major = version.major as u64;
    let minor = version.minor as u64;
    let patch = version.patch as u64;
    U256::from((major << 32) | (minor << 24) | patch)
}

/// Parse on-chain hex to semver Version
pub fn hex_to_version(hex: U256) -> Version {
    let value = hex.as_limbs()[0];
    let major = (value >> 32) as u64;
    let minor = ((value >> 24) & 0xFF) as u64;
    let patch = (value & 0xFFFFFF) as u64;
    Version::new(major, minor, patch)
}

// Example usage:
// Version::new(29, 0, 0) -> 0x1d00000000
// Version::new(30, 0, 0) -> 0x1e00000000
// Version::new(30, 0, 1) -> 0x1e00000001
```

---

## Validation Functions

```rust
use alloy_primitives::Address;
use alloy_signer::Signer;

impl Ecosystem {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.name.is_empty(), "Ecosystem name cannot be empty");
        ensure!(self.name.len() <= 64, "Ecosystem name too long");
        ensure!(
            self.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "Ecosystem name must be alphanumeric with underscores"
        );
        self.contracts.validate()?;
        self.wallets.validate()?;
        Ok(())
    }
}

impl Chain {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.name.is_empty(), "Chain name cannot be empty");
        ensure!(self.chain_id > 0, "Chain ID must be positive");
        ensure!(
            self.chain_id != 1 && self.chain_id != 11155111,
            "Chain ID conflicts with settlement layer networks"
        );
        self.contracts.validate()?;
        self.wallets.validate()?;
        Ok(())
    }
}

impl Wallet {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.address.is_zero(),
            "Wallet address cannot be zero"
        );
        if let Some(ref pk) = self.private_key {
            // Use alloy-signer for validation
            let signer = alloy_signer::LocalWallet::from_str(pk.expose_secret())?;
            ensure!(
                signer.address() == self.address,
                "Private key does not match address"
            );
        }
        Ok(())
    }
}
```

---

## Recommended Alloy Crates

| Crate              | Purpose                                     |
| ------------------ | ------------------------------------------- |
| `alloy-primitives` | Address, B256, U256, Bytes types            |
| `alloy-signer`     | Local wallet signing and address derivation |
| `alloy-provider`   | JSON-RPC provider for settlement layer interactions |
| `alloy-contract`   | Contract interaction helpers                |
| `alloy-rlp`        | RLP encoding for transactions               |
| `alloy-sol-types`  | Solidity type encoding/decoding             |
