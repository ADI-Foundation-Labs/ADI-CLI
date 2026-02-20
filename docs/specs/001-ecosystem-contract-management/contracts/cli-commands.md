# CLI Command Contracts

**Branch**: `001-ecosystem-contract-management` | **Date**: 2026-01-22

This document defines the command-line interface contracts for the ADI CLI. Each command specifies its arguments, options, inputs, outputs, and error conditions.

---

## Command Hierarchy

```
adi
├── init
│   ├── ecosystem    # Initialize new ecosystem configuration
│   └── chain        # Initialize chain configuration
├── deploy
│   ├── ecosystem    # Deploy ecosystem contracts to settlement layer
│   └── chain        # Deploy chain contracts to settlement layer
├── upgrade
│   ├── ecosystem    # Upgrade ecosystem contracts
│   └── chain        # Upgrade chain contracts
├── doctor           # Verify dependency availability
└── version
    └── show         # Show version information
```

---

## adi init ecosystem

Initialize a new ZkSync ecosystem configuration.

### Synopsis
```
adi init ecosystem [OPTIONS]
```

### Options
| Option                 | Type    | Required | Default     | Description                              |
| ---------------------- | ------- | -------- | ----------- | ---------------------------------------- |
| `--name`               | string  | No       | from config | Ecosystem name                                        |
| `--settlement-network` | enum    | No       | localhost   | Settlement network (mainnet, sepolia, localhost)      |
| `--settlement-rpc-url` | string  | No       | from config | Settlement layer RPC endpoint URL                     |
| `--chain-name`         | string  | No       | from config | Initial chain name                       |
| `--chain-id`           | u64     | No       | from config | Initial chain ID                         |
| `--prover-mode`        | enum    | No       | no-proofs   | Prover mode (no-proofs, gpu)             |
| `--base-token-address` | address | No       | ETH         | Custom base token contract address       |
| `--wallet-creation`    | enum    | No       | random      | Wallet creation mode (random, provided)  |
| `--state-dir`          | path    | No       | from config | State directory path                     |

### Environment Variables
| Variable                   | Maps To              |
| -------------------------- | -------------------- |
| `ADI_ECOSYSTEM_NAME`       | `--name`             |
| `ADI_SETTLEMENT_RPC_URL`   | `--settlement-rpc-url` |
| `ADI_ECOSYSTEM_CHAIN_NAME` | `--chain-name`       |
| `ADI_ECOSYSTEM_CHAIN_ID`   | `--chain-id`         |
| `ADI_STATE_DIR`            | `--state-dir`        |

### Output
**Success (exit 0):**
```
[INFO] Initializing ecosystem 'adi_ecosystem'
[INFO] Creating ecosystem directory structure
[INFO] Generated deployer wallet: 0x1234...5678
[INFO] Generated governor wallet: 0xabcd...ef01
[INFO] Creating initial chain 'adi' with ID 222

[SUCCESS] Ecosystem initialized at /path/to/state/adi_ecosystem

State files created:
  - ZkStack.yaml
  - configs/wallets.yaml
  - configs/contracts.yaml
  - chains/adi/configs/wallets.yaml
  - chains/adi/configs/genesis.yaml
```

**Error (exit 1):**
```
Error: Failed to initialize ecosystem

Cause: Ecosystem 'adi_ecosystem' already exists

Resolution:
  1. Choose a different ecosystem name with --name
  2. Or remove existing state at ~/.adi_cli/state/adi_ecosystem
```

### Preconditions
- State directory must be writable
- Ecosystem name must not already exist
- zkstack CLI must be available (verified via `adi doctor`)

### Postconditions
- Ecosystem directory structure created
- Wallet keypairs generated and stored
- Chain configuration initialized

---

## adi deploy ecosystem

Deploy ecosystem smart contracts to the settlement layer.

### Synopsis
```
adi deploy ecosystem [OPTIONS]
```

### Options
| Option               | Type   | Required | Default     | Description                       |
| -------------------- | ------ | -------- | ----------- | --------------------------------- |
| `--name`             | string | No       | from config | Ecosystem name                    |
| `--settlement-rpc-url` | string | No     | from config | Settlement layer RPC endpoint URL |
| `--gas-price`        | u64    | No       | auto        | Gas price in wei                  |
| `--dry-run`          | bool   | No       | false       | Simulate without broadcasting     |
| `--auto-fund`        | bool   | No       | true        | Auto-fund wallets from funder     |

### Environment Variables
| Variable                     | Maps To                     |
| ---------------------------- | --------------------------- |
| `ADI_ECOSYSTEM_NAME`         | `--name`                    |
| `ADI_SETTLEMENT_RPC_URL`     | `--settlement-rpc-url`      |
| `ADI_SETTLEMENT_GAS_PRICE`   | `--gas-price`               |
| `ADI_FUNDER_PRIVATE_KEY`     | Funder wallet for auto-fund |

### Output
**Success (exit 0):**
```
[INFO] Deploying ecosystem 'adi_ecosystem' to Sepolia
[INFO] Checking wallet balances...
[INFO]   Deployer: 1.5 ETH (required: 1 ETH) ✓
[INFO]   Governor: 1.2 ETH, 5 ADI (required: 1 ETH, 5 ADI) ✓
[INFO] Deploying contracts with gas price 10 gwei

[PROGRESS] Deploying Bridgehub...
[INFO] Bridgehub deployed at 0xf69d...ab9
[PROGRESS] Deploying Governance...
[INFO] Governance deployed at 0x1234...5678
[PROGRESS] Deploying Verifier...
[INFO] Verifier deployed at 0xabcd...ef01
... (more contracts)

# Note: Contracts below are representative for v0.29.x-v0.30.x. Actual contracts vary by protocol version.
[PROGRESS] Accepting ownership transfers...
[INFO] Server Notifier ownership accepted
[INFO] Validator Timelock ownership accepted
[INFO] Verifier ownership accepted
[INFO] Governance ownership accepted

[SUCCESS] Ecosystem contracts deployed (ownership accepted)

Contract addresses saved to:
  - configs/contracts.yaml

Key addresses:
  - Bridgehub: 0xf69daaea7f8578933237a9b59f42704ebec36ab9
  - Governance: 0x1234567890abcdef...
  - Chain Admin: 0xabcdef1234567890...
```

**Error (exit 1):**
```
Error: Failed to deploy ecosystem contracts

Cause: Insufficient balance in deployer wallet

Details:
  - Wallet: 0x1234...5678
  - Required: 1.5 ETH
  - Available: 0.3 ETH

Resolution:
  1. Fund the deployer wallet with at least 1.2 ETH more
  2. Or configure a funder wallet in ~/.adi_cli/.adi.yml:
     funder:
       private_key: "0x..."
  3. Re-run: adi deploy ecosystem
```

### Preconditions
- Ecosystem must be initialized
- Wallets must have sufficient balance (or funder configured)
- Settlement layer RPC must be reachable
- zkstack CLI and forge must be available

### Postconditions
- All ecosystem contracts deployed to settlement layer
- Contract addresses persisted to state
- Ownership transfers automatically accepted

---

## adi init chain

Initialize a new chain configuration within an ecosystem.

### Synopsis
```
adi init chain [OPTIONS]
```

### Options
| Option                 | Type    | Required | Default     | Description               |
| ---------------------- | ------- | -------- | ----------- | ------------------------- |
| `--ecosystem-name`     | string  | No       | from config | Parent ecosystem name     |
| `--name`               | string  | Yes      | -           | Chain name                |
| `--chain-id`           | u64     | Yes      | -           | Chain ID                  |
| `--base-token-address` | address | No       | ETH         | Custom base token address |
| `--prover-mode`        | enum    | No       | no-proofs   | Prover mode               |

### Output
**Success (exit 0):**
```
[INFO] Initializing chain 'adi' (ID: 222) in ecosystem 'adi_ecosystem'
[INFO] Creating chain directory structure
[INFO] Generated chain wallets

[SUCCESS] Chain 'adi' initialized

Chain configuration saved to:
  - chains/adi/configs/wallets.yaml
  - chains/adi/configs/genesis.yaml

Next step: Deploy chain contracts with:
  adi deploy chain --name adi
```

**Error (exit 1):**
```
Error: Failed to initialize chain

Cause: Chain 'adi' already exists in ecosystem

Resolution:
  1. Choose a different chain name with --name
  2. Or remove existing chain at ~/.adi_cli/state/adi_ecosystem/chains/adi
```

### Preconditions
- Parent ecosystem must be initialized
- Chain name must be unique within ecosystem
- Chain ID must not conflict with settlement layer or existing chains

### Postconditions
- Chain directory structure created
- Chain wallet keypairs generated and stored
- Chain configuration initialized
- NO contracts deployed (use `adi deploy chain`)

---

## adi deploy chain

Deploy chain contracts to the settlement layer and register with Bridgehub.

### Synopsis
```
adi deploy chain [OPTIONS]
```

### Options
| Option                 | Type   | Required | Default     | Description                       |
| ---------------------- | ------ | -------- | ----------- | --------------------------------- |
| `--ecosystem-name`     | string | No       | from config | Parent ecosystem name             |
| `--name`               | string | Yes      | -           | Chain name                        |
| `--settlement-rpc-url` | string | No       | from config | Settlement layer RPC endpoint URL |
| `--gas-price`          | u64    | No       | auto        | Gas price in wei                  |
| `--dry-run`            | bool   | No       | false       | Simulate without broadcasting     |
| `--auto-fund`          | bool   | No       | true        | Auto-fund wallets from funder     |

### Environment Variables
| Variable                   | Maps To                     |
| -------------------------- | --------------------------- |
| `ADI_ECOSYSTEM_NAME`       | `--ecosystem-name`          |
| `ADI_SETTLEMENT_RPC_URL`   | `--settlement-rpc-url`      |
| `ADI_SETTLEMENT_GAS_PRICE` | `--gas-price`               |
| `ADI_FUNDER_PRIVATE_KEY`   | Funder wallet for auto-fund |

### Output
**Success (exit 0):**
```
[INFO] Deploying chain 'adi' (ID: 222) to settlement layer
[INFO] Checking wallet balances...
[INFO]   Chain Deployer: 1.5 ETH (required: 1 ETH) ✓
[INFO]   Chain Governor: 1.2 ETH, 5 ADI (required: 1 ETH, 5 ADI) ✓
[INFO] Deploying chain contracts with gas price 10 gwei

[PROGRESS] Deploying Diamond Proxy...
[INFO] Diamond Proxy deployed at 0x9876...5432
[PROGRESS] Deploying Chain Admin...
[INFO] Chain Admin deployed at 0xfedc...ba98
[PROGRESS] Registering chain with Bridgehub...
[INFO] Chain registered with Bridgehub
# Note: Contracts below are representative for v0.29.x-v0.30.x. Actual contracts vary by protocol version.
[PROGRESS] Accepting ownership transfers...
[INFO] Chain Admin ownership accepted

[SUCCESS] Chain 'adi' deployed and registered (ownership accepted)

Contract addresses saved to:
  - chains/adi/configs/contracts.yaml

Key addresses:
  - Diamond Proxy: 0x9876543210fedcba9876543210fedcba98765432
  - Chain Admin: 0xfedcba9876543210fedcba9876543210fedcba98
  - Governance: 0x1111222233334444555566667777888899990000
```

**Error (exit 1):**
```
Error: Failed to deploy chain contracts

Cause: Insufficient balance in chain deployer wallet

Details:
  - Wallet: 0x1234...5678
  - Required: 1.5 ETH
  - Available: 0.3 ETH

Resolution:
  1. Fund the chain deployer wallet with at least 1.2 ETH more
  2. Or configure a funder wallet in ~/.adi_cli/.adi.yml:
     funder:
       private_key: "0x..."
  3. Re-run: adi deploy chain --name adi
```

### Preconditions
- Chain must be initialized (via `adi init chain`)
- Parent ecosystem must be deployed (contracts exist on settlement layer)
- Chain wallets must have sufficient balance (or funder configured)
- Settlement layer RPC must be reachable
- zkstack CLI and forge must be available

### Postconditions
- Chain contracts deployed to settlement layer
- Chain registered with Bridgehub
- Contract addresses persisted to chain state
- Ownership transfers automatically accepted

---

## adi doctor

Verify external dependency availability.

### Synopsis
```
adi doctor [OPTIONS]
```

### Options
| Option   | Type | Required | Default | Description    |
| -------- | ---- | -------- | ------- | -------------- |
| `--json` | bool | No       | false   | Output as JSON |

### Output
**Success (exit 0):**
```
ADI CLI Dependency Check
========================

[✓] zkstack CLI
    Version: 0.1.0
    Path: /usr/local/bin/zkstack

[✓] forge (foundry-zksync)
    Version: 0.0.2-zksync
    Path: /root/.foundry/bin/forge

[✓] cast (foundry-zksync)
    Version: 0.0.2-zksync
    Path: /root/.foundry/bin/cast

[✓] Configuration
    Config file: ~/.adi_cli/.adi.yml (exists)
    State directory: ~/.adi_cli/state (writable)

All dependencies available.
```

**Failure (exit 1):**
```
ADI CLI Dependency Check
========================

[✓] zkstack CLI
    Version: 0.1.0

[✗] forge (foundry-zksync)
    NOT FOUND

    Install with:
      curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
      foundryup-zksync

[✗] cast (foundry-zksync)
    NOT FOUND

[✓] Configuration
    Config file: ~/.adi_cli/.adi.yml (exists)

2 dependency issues found.
```

**JSON Output:**
```json
{
  "status": "failed",
  "dependencies": {
    "zkstack": { "available": true, "version": "0.1.0", "path": "/usr/local/bin/zkstack" },
    "forge": { "available": false, "error": "Command not found" },
    "cast": { "available": false, "error": "Command not found" }
  },
  "config": {
    "config_file": "~/.adi_cli/.adi.yml",
    "config_exists": true,
    "state_dir": "~/.adi_cli/state",
    "state_writable": true
  }
}
```

---

## adi upgrade ecosystem

Upgrade ecosystem contracts to a new protocol version.

### Synopsis
```
adi upgrade ecosystem --to <VERSION> [OPTIONS]
```

### Options
| Option             | Type   | Required | Default          | Description                                  |
| ------------------ | ------ | -------- | ---------------- | -------------------------------------------- |
| `--to`               | string | Yes      | -                | Target protocol version (e.g., v30)          |
| `--ecosystem-name`   | string | No       | from config      | Ecosystem name                               |
| `--settlement-rpc-url` | string | No     | from config      | Settlement layer RPC endpoint URL            |
| `--gas-price`        | u64    | No       | auto             | Gas price in wei                             |
| `--output-dir`     | path   | No       | ./upgrade-output | Directory for calldata output                |
| `--execute`        | bool   | No       | false            | Execute upgrade (not just generate calldata) |

### Output
**Success (exit 0):**
```
[INFO] Preparing ecosystem upgrade: v29 → v30
[INFO] Generating upgrade calldata...

[SUCCESS] Upgrade calldata generated

Current version: v29.0.11
Target version:  v30.0.0

Calldata saved to:
  - upgrade-output/schedule-transparent.calldata
  - upgrade-output/execute.calldata

Deployment output saved to:
  - upgrade-output/v30-ecosystem.toml

Note: The deployment output file contains new contract addresses, deployment
data, and transaction history. This file is required as input for subsequent
upgrades.

To execute the upgrade:
  1. Review generated calldata
  2. Execute scheduleTransparent via governance:
     cast send 0x<governance> --calldata-file upgrade-output/schedule-transparent.calldata
  3. Execute upgrade:
     cast send 0x<governance> --calldata-file upgrade-output/execute.calldata

Or use --execute flag to execute automatically:
  adi upgrade ecosystem --to v30 --execute
```

### Preconditions
- Ecosystem must be deployed
- Current version must be compatible with target version
- Target version must be supported

### Postconditions
- Upgrade calldata files generated
- Deployment output file saved (v{VERSION}-ecosystem.toml)
- If `--execute`, upgrade executed via governance

---

## adi upgrade chain

Upgrade chain contracts to match ecosystem version.

### Synopsis
```
adi upgrade chain --to <VERSION> [OPTIONS]
```

### Options
| Option             | Type   | Required | Default          | Description               |
| ------------------ | ------ | -------- | ---------------- | ------------------------- |
| `--to`               | string | Yes      | -                | Target protocol version            |
| `--chain-name`       | string | No       | from config      | Chain name                         |
| `--ecosystem-name`   | string | No       | from config      | Ecosystem name                     |
| `--settlement-rpc-url` | string | No     | from config      | Settlement layer RPC URL           |
| `--l2-rpc-url`       | string | No       | from config      | L2 RPC URL                         |
| `--gas-price`      | u64    | No       | auto             | Gas price in wei          |
| `--output-dir`     | path   | No       | ./upgrade-output | Calldata output directory |
| `--execute`        | bool   | No       | false            | Execute upgrade           |

### Output
**Success (exit 0):**
```
[INFO] Preparing chain upgrade: adi v29 → v30
[INFO] Generating chain upgrade calldata...

[SUCCESS] Chain upgrade calldata generated

Calldata saved to:
  - upgrade-output/chain-schedule.calldata
  - upgrade-output/chain-admin.calldata

Deployment output saved to:
  - upgrade-output/v30-<chain-name>.toml

To execute:
  1. Execute schedule calldata via chain admin
  2. Execute chain admin calldata

Or use --execute flag.
```

### Postconditions
- Chain upgrade calldata files generated
- Deployment output file saved (v{VERSION}-{chain-name}.toml)
- If `--execute`, upgrade executed via chain admin

---

## Global Options

These options apply to all commands:

| Option            | Type | Description                                        |
| ----------------- | ---- | -------------------------------------------------- |
| `--config`        | path | Path to config file (default: ~/.adi_cli/.adi.yml) |
| `--verbose`, `-v` | bool | Enable verbose output                              |
| `--quiet`, `-q`   | bool | Suppress non-error output                          |
| `--help`, `-h`    | bool | Show help message                                  |

---

## Exit Codes

| Code | Meaning             |
| ---- | ------------------- |
| 0    | Success             |
| 1    | General error       |
| 2    | Configuration error |
| 3    | Missing dependency  |
| 4    | Network/RPC error   |
| 5    | Insufficient funds  |
| 6    | Contract error      |
| 7    | State error         |
