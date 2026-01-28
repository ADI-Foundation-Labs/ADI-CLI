# Quickstart: ADI CLI Ecosystem Contract Management

This guide covers the basic setup and usage of the ADI CLI for managing ZkSync ecosystem smart contracts.

## Prerequisites

- Docker installed and running
- Access to a settlement layer RPC endpoint (Mainnet, Sepolia, or local Anvil)
- Funder wallet with sufficient ETH for deployments

## Configuration

The CLI uses a configuration file at `~/.adi_cli/.adi.yml`:

```yaml
state_dir: ~/.adi_cli/state

settlement:
  rpc_url: https://sepolia.infura.io/v3/YOUR_KEY
  gas_price: null  # Optional, in wei

funder:
  private_key: "0x..."  # Funder wallet private key
  cgt_address: null     # Custom Gas Token address (if not using ETH)

ecosystem:
  name: my_ecosystem
  chain_name: my_chain
  chain_id: 270

docker:
  registry: harbor.io/adi
  image_name: adi-toolkit
```

Environment variables can override config values using the `ADI_` prefix:

```bash
export ADI_SETTLEMENT_RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY
export ADI_FUNDER_PRIVATE_KEY=0x...
```

## State Directory Structure

The ADI CLI persists all ecosystem and chain state in a structured directory hierarchy. By default, state is stored in `~/.adi_cli/state/`.

### Directory Layout

```text
~/.adi_cli/state/
├── ecosystems/
│   └── {ecosystem_name}/
│       ├── metadata              # Ecosystem configuration (YAML)
│       ├── contracts             # Deployed contract addresses (YAML)
│       ├── wallets               # Wallet addresses (YAML, keys stored separately)
│       └── chains/
│           └── {chain_name}/
│               ├── metadata      # Chain configuration (YAML)
│               ├── contracts     # Chain contract addresses (YAML)
│               └── wallets       # Chain wallet addresses (YAML)
├── upgrades/
│   └── {ecosystem_name}/
│       └── {upgrade_id}          # Upgrade records with calldata
└── .backups/
    └── {key_path}/
        └── {timestamp}           # Automatic backups before destructive operations
```

### Key Hierarchy

State keys follow a hierarchical pattern:

| Key Pattern | Description |
|-------------|-------------|
| `ecosystems/{name}/metadata` | Ecosystem configuration and status |
| `ecosystems/{name}/contracts` | Ecosystem contract addresses |
| `ecosystems/{name}/wallets` | Ecosystem wallet addresses |
| `ecosystems/{name}/chains/{chain}/metadata` | Chain configuration |
| `ecosystems/{name}/chains/{chain}/contracts` | Chain contract addresses |
| `ecosystems/{name}/chains/{chain}/wallets` | Chain wallet addresses |
| `upgrades/{ecosystem}/{id}` | Upgrade transaction records |
| `.backups/{key}/{timestamp}` | Automatic backups |

### State Integrity

On startup, the CLI validates state integrity:

1. **Directory validation**: Ensures the state directory exists and is writable
2. **Orphan detection**: Identifies temporary files from interrupted operations
3. **File integrity**: Verifies all state files are readable

If issues are found, the CLI reports them with actionable guidance:

```text
Warning: Found 2 orphaned temporary files
  - ~/.adi_cli/state/ecosystems/test/.metadata.tmp
  - ~/.adi_cli/state/ecosystems/test/.contracts.tmp

Run 'adi state cleanup' to remove orphaned files.
```

### Automatic Backups

Before destructive operations (delete, overwrite), the CLI automatically creates timestamped backups:

```text
# Backup location
.backups/ecosystems/my_ecosystem/metadata/20260128_143022_123

# Restore from backup
adi state restore ecosystems/my_ecosystem/metadata

# List available backups
adi state list-backups ecosystems/my_ecosystem/metadata
```

Backups are stored in the `.backups/` directory with timestamps in the format `YYYYMMDD_HHMMSS_mmm`.

### Atomic Writes

All state writes use atomic operations to prevent corruption:

1. Data is written to a temporary file (`.{filename}.tmp`)
2. File is synced to disk
3. Temporary file is atomically renamed to target path

This ensures state files are never partially written, even if the process is interrupted.

## Basic Workflow

### 1. Initialize Ecosystem

```bash
adi init ecosystem \
  --name my_ecosystem \
  --settlement-network sepolia
```

This creates:
- Ecosystem configuration in state directory
- Generated wallets (deployer, governor)
- ZkStack.yaml configuration file

### 2. Deploy Ecosystem Contracts

```bash
adi deploy ecosystem
```

Deploys core infrastructure contracts:
- Bridgehub
- State Transition Manager
- Governance
- Verifier
- DA validators

### 3. Initialize Chain

```bash
adi init chain \
  --name my_chain \
  --chain-id 270 \
  --base-token eth \
  --prover-mode no-proofs
```

### 4. Deploy Chain Contracts

```bash
adi deploy chain --chain-name my_chain
```

Deploys chain-specific contracts and registers with Bridgehub.

### 5. Upgrade (when needed)

```bash
# Prepare upgrade calldata
adi upgrade ecosystem --to v30

# Prepare chain upgrade
adi upgrade chain --chain-name my_chain --to v30
```

## Local Development with Anvil

For local testing, run a local Anvil instance:

```bash
# Start Anvil
anvil --host 0.0.0.0 --port 8545

# Configure CLI for local development
export ADI_SETTLEMENT_RPC_URL=http://localhost:8545
export ADI_FUNDER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80  # Anvil default

# Run commands against local network
adi init ecosystem --name test --settlement-network localhost
adi deploy ecosystem
```

## Troubleshooting

### State Directory Issues

**Error: State directory not writable**

```bash
# Check permissions
ls -la ~/.adi_cli/

# Fix permissions
chmod 755 ~/.adi_cli
chmod 755 ~/.adi_cli/state
```

**Error: State file corrupted**

```bash
# List available backups
adi state list-backups ecosystems/my_ecosystem/metadata

# Restore from most recent backup
adi state restore ecosystems/my_ecosystem/metadata
```

### Docker Issues

**Error: Docker daemon not running**

```bash
# Start Docker
open -a Docker  # macOS
sudo systemctl start docker  # Linux
```

**Error: Toolkit image not found**

```bash
# Pull manually
docker pull harbor.io/adi/adi-toolkit:v29.0.11
```

### Deployment Issues

**Error: Insufficient funds**

The CLI automatically checks wallet balances before deployment. Ensure the funder wallet has sufficient ETH:

- Ecosystem deployment: ~2 ETH
- Chain deployment: ~1 ETH per chain

**Error: Transaction reverted**

Check the settlement layer for:
- Correct RPC URL
- Network congestion (increase gas price)
- Contract state conflicts
