# Quickstart: ADI CLI Ecosystem Contract Management

This guide covers the basic setup and usage of the ADI CLI for managing ZkSync ecosystem smart contracts.

## Prerequisites

- Docker installed and running (Docker Desktop 4.x+ or Docker Engine 20.10+)
- Access to a settlement layer RPC endpoint (Mainnet, Sepolia, or local Anvil)
- Funder wallet with sufficient ETH for deployments
- Minimum 8GB RAM available for Docker containers

## Docker Toolkit Architecture

The ADI CLI orchestrates pre-built Docker toolkit images containing all required ZkSync development tools. This architecture ensures reproducible deployments with version-pinned dependencies.

### Toolkit Image Contents

Each toolkit image (`adi-toolkit`) contains:
- **zkstack CLI**: ZkSync ecosystem management tool (from specific commit)
- **foundry-zksync**: ZkSync-compatible forge and cast tools
- **era-contracts**: Smart contract sources for upgrade scripts
- **Python dependencies**: eth-abi, eth-hash for upgrade calldata encoding

### Version-Specific Images

| Image Tag | Protocol Version | Era Contracts | Use Case |
|-----------|------------------|---------------|----------|
| `adi-toolkit:v29` | v0.29.x | zkos-v0.29.11 | Initial deployment |
| `adi-toolkit:v30` | v0.30.x | v30-zksync-os-upgrade | Upgrade target |

### Building Toolkit Images

Build toolkit images locally using Docker Bake:

```bash
# Build all toolkit versions (v29 + v30)
docker buildx bake -f docker/docker-bake.hcl

# Build specific version
docker buildx bake -f docker/docker-bake.hcl toolkit-v29

# Build with custom registry
REGISTRY=my-registry.io/project docker buildx bake -f docker/docker-bake.hcl
```

### Pulling Pre-Built Images

Pre-built images are available from the configured registry:

```bash
# Pull v29 toolkit
docker pull registry.sre.ideasoft.io/adi-foundation/adi-chain/cli/adi-toolkit:v29

# Pull v30 toolkit
docker pull registry.sre.ideasoft.io/adi-foundation/adi-chain/cli/adi-toolkit:v30
```

### Container Execution Model

The CLI uses ephemeral containers for each operation:

1. **Container Creation**: A new container is created from the toolkit image
2. **Volume Mounts**: State directory (`~/.adi_cli/state/`) is mounted for persistence
3. **Command Execution**: zkstack/forge commands run inside the container
4. **Output Streaming**: Real-time output is streamed to the terminal
5. **Container Cleanup**: Container is removed after operation completes

```text
Host Machine
┌─────────────────────────────────────────────────────────┐
│  adi-cli (Rust binary)                                  │
│  ├── Commands (Clap)                                    │
│  ├── Docker Orchestrator (Bollard)                      │
│  └── Config/State (~/.adi_cli/)                         │
└────────────────────┬────────────────────────────────────┘
                     │ Docker API
┌────────────────────▼────────────────────────────────────┐
│  Docker Daemon                                          │
│  └── adi-toolkit:v{VERSION} (ephemeral container)       │
│      ├── zkstack CLI                                    │
│      ├── foundry-zksync (forge, cast)                   │
│      └── era-contracts                                  │
└─────────────────────────────────────────────────────────┘
```

### Docker Configuration

Configure the toolkit image source in `~/.adi_cli/.adi.yml`:

```yaml
docker:
  registry: registry.sre.ideasoft.io/adi-foundation/adi-chain/cli
  image_name: adi-toolkit
```

Override via environment variables:

```bash
export ADI_DOCKER_REGISTRY=my-registry.io/project
export ADI_DOCKER_IMAGE_NAME=adi-toolkit
```

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
# Check Docker status
docker info

# Start Docker
open -a Docker  # macOS
sudo systemctl start docker  # Linux

# Verify Docker is accessible
docker ps
```

**Error: Toolkit image not found**

```bash
# Pull specific version
docker pull registry.sre.ideasoft.io/adi-foundation/adi-chain/cli/adi-toolkit:v29

# Or build locally
docker buildx bake -f docker/docker-bake.hcl toolkit-v29
```

**Error: Insufficient Docker resources**

The toolkit build requires significant resources. Ensure Docker has:
- At least 8GB RAM allocated
- At least 20GB disk space available

```bash
# Check Docker resource usage
docker system df

# Clean up unused resources
docker system prune -a
```

**Error: Build cache issues**

```bash
# Clear Docker build cache
docker builder prune --all

# Rebuild without cache
docker buildx bake -f docker/docker-bake.hcl --no-cache
```

**Error: Container network issues**

If containers cannot reach the settlement layer RPC:

```bash
# Test connectivity from container
docker run --rm adi-toolkit:v29 curl -s https://sepolia.infura.io/health

# Use host network mode if needed (configured in CLI)
# The CLI handles network configuration automatically
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
