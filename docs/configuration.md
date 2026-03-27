# Configuration Reference

Complete reference for `~/.adi.yml`. For a minimal starting config, see the [README](../README.md#minimal-config).

## Full Annotated Config

```yaml
# ~/.adi.yml

# Where to store ecosystem state (wallets, contracts, chain configs)
# Default: ~/.adi_cli/state
state_dir: ~/.adi_cli/state

# Enable verbose logging for troubleshooting
# Default: false (can also use -d flag)
debug: false

# Default protocol version for toolkit Docker image
# Used by init, add, and deploy when --protocol-version is not provided
protocol_version: v0.30.1

# State storage backend (currently only "filesystem" is supported)
# Default: filesystem
state_backend: filesystem

# Ecosystem configuration
ecosystem:
  # Name used for the ecosystem directory and identification
  # Default: adi_ecosystem
  name: adi_ecosystem

  # Settlement layer network where contracts are deployed
  # Options: localhost (Anvil), sepolia (testnet), mainnet
  # Default: sepolia
  l1_network: sepolia

  # Settlement layer RPC endpoint
  # For Anvil (local): http://host.docker.internal:8545
  # For Sepolia: https://sepolia.infura.io/v3/YOUR_KEY
  rpc_url: https://sepolia.infura.io/v3/YOUR_KEY

  # Ecosystem-level ownership (for Governance, Bridgehub, etc.)
  # ownership:
  #   new_owner: "0x..."

  # Chain configurations (supports multiple chains)
  chains:
    - name: my-chain
      # Unique numeric chain identifier
      # Default: 222
      chain_id: 222

      # Proof generation mode
      # no-proofs: Development/testing (fast, no real proofs)
      # gpu: Production (requires GPU prover infrastructure)
      # Default: no-proofs
      prover_mode: no-proofs

      # Enable EVM bytecode emulator for running unmodified Ethereum contracts
      # Default: false
      evm_emulator: false

      # Use blob-based pubdata (EIP-4844)
      # true: Uses blobs (L2 chains settling on L1)
      # false: Uses calldata (L3 chains settling on L2)
      # Default: false
      blobs: false

      # Custom ERC20 token for gas payments (omit to use native ETH)
      # base_token_address: "0x..."
      # base_token_price_nominator: 1
      # base_token_price_denominator: 1

      # Predefined operator addresses
      # Override randomly generated addresses for validator roles
      # operators:
      #   operator: "0x..."         # PRECOMMITTER, COMMITTER, REVERTER roles
      #   prove_operator: "0x..."   # PROVER role
      #   execute_operator: "0x..." # EXECUTOR role

      # Per-chain funding (ETH amounts in ether)
      # funding:
      #   operator_eth: 30.0
      #   prove_operator_eth: 30.0
      #   execute_operator_eth: 30.0

      # Per-chain ownership configuration
      # ownership:
      #   new_owner: "0x..."

# Ecosystem-level wallet funding during deployment
# NOTE: Per-chain operator funding is configured via ecosystem.chains[].funding
funding:
  # SECURITY: Use ADI_FUNDER_KEY environment variable instead
  # Never commit private keys to config files
  # funder_key: "0x..."

  # ETH amounts for ecosystem-level wallets (in ether)
  #
  # For Sepolia TESTING (short-term, minimal funding):
  #   deployer_eth: 1.0
  #   governor_eth: 1.0
  #   governor_cgt_units: 5.0
  #
  # For PRODUCTION or long-running chains:
  deployer_eth: 100.0
  governor_eth: 40.0
  governor_cgt_units: 5.0     # Custom gas token (if using custom base token)

# Gas price multiplier percentage (default: 200 = 100% buffer).
# Applied to all on-chain transactions (deploy, accept, transfer).
# The CLI fetches current gas price and multiplies by this percentage.
# Recommended: 200 for Anvil, 300 for Sepolia/mainnet.
gas_multiplier: 200

# S3 synchronization for state backup and sharing
# s3:
#   enabled: true
#   tenant_id: my-tenant       # Used as S3 key prefix
#   bucket: adi-state           # S3 bucket name
#   region: us-east-1           # AWS region
#   endpoint_url: http://localhost:9000  # For MinIO/LocalStack

# Override Docker toolkit image settings
# toolkit:
#   image_tag: "latest"         # Custom tag instead of protocol version
```

## Config Resolution Priority

Config file sources are mutually exclusive (only one file is loaded):

1. `--config` flag
2. `ADI_CONFIG` environment variable
3. `~/.adi.yml` (default)

Override sources are always applied on top:

4. `ADI__*` environment variables
5. CLI flags (highest priority)

## Environment Variables

| Variable | Purpose |
|---|---|
| `ADI_FUNDER_KEY` | Private key (hex) of the wallet that funds ecosystem wallets |
| `ADI_PRIVATE_KEY` | Private key (hex) for accepting ownership as new owner |
| `ADI_RPC_URL` | Settlement layer RPC endpoint |
| `ADI_EXPLORER_URL` | Block explorer API URL for contract verification |
| `ADI_EXPLORER_API_KEY` | Block explorer API key (optional for public explorers) |
| `ADI_CONFIG` | Path to an alternative config file |
| `ADI__PROTOCOL_VERSION` | Default protocol version (e.g., `v0.30.1`) |
| `ADI__TOOLKIT__IMAGE_TAG` | Override Docker image tag (e.g., `latest`) |
| `ADI_OPERATOR` | Operator address (PRECOMMITTER, COMMITTER, REVERTER roles) |
| `ADI_PROVE_OPERATOR` | Prove operator address (PROVER role) |
| `ADI_EXECUTE_OPERATOR` | Execute operator address (EXECUTOR role) |
| `AWS_ACCESS_KEY_ID` | AWS access key for S3 state sync |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key for S3 state sync |
| `ADI__S3__ENABLED` | Enable S3 sync (`true`/`false`) |
| `ADI__S3__TENANT_ID` | Tenant identifier for S3 key prefix |
| `ADI__S3__BUCKET` | S3 bucket name |
| `RUST_LOG` | Logging verbosity: `error`, `warn`, `info`, `debug`, `trace` |

Override any config value using the `ADI__` prefix with double underscores as path separators:

```bash
export ADI__ECOSYSTEM__NAME=production
export ADI__ECOSYSTEM__RPC_URL=http://localhost:8545
```

## S3 State Synchronization

When enabled (`s3.enabled: true`), the CLI automatically archives and uploads ecosystem state to S3 after write operations (init, deploy). Archives are stored at `s3://{bucket}/{tenant_id}/{ecosystem-name}.tar.gz`.

### Manual Sync Commands

```bash
# Sync current state to S3
adi state sync --ecosystem-name my-ecosystem

# Restore state from S3
adi state restore --ecosystem-name my-ecosystem

# Force restore (overwrite local state)
adi state restore --ecosystem-name my-ecosystem --force
```

### Using with MinIO (Local Development)

```bash
docker run -p 9000:9000 -p 9001:9001 minio/minio server /data --console-address ":9001"
```

```yaml
s3:
  enabled: true
  tenant_id: dev
  bucket: adi-state
  endpoint_url: http://localhost:9000
```

Set credentials via `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` environment variables.

## Deprecated Config Fields

These fields still work but will be removed in a future release:

- **Top-level `ownership`** — use `ecosystem.ownership` or `ecosystem.chains[].ownership` instead
- **Top-level `operators`** — use `ecosystem.chains[].operators` instead
