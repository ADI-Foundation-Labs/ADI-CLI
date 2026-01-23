# CLI Command Contracts

**Branch**: `001-ecosystem-contract-management` | **Date**: 2026-01-22

This document defines the command-line interface contracts for the ADI CLI. Each command specifies its arguments, options, inputs, outputs, and error conditions.

---

## Command Hierarchy

```
adi
├── init
│   ├── ecosystem    # Initialize new ecosystem configuration
│   └── chain        # Initialize and register a chain
├── deploy
│   └── ecosystem    # Deploy ecosystem contracts to L1
├── upgrade
│   ├── ecosystem    # Upgrade ecosystem contracts
│   └── chain        # Upgrade chain contracts
├── accept
│   └── ownership    # Accept pending ownership transfers
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
| `--name`               | string  | No       | from config | Ecosystem name                           |
| `--l1-network`         | enum    | No       | localhost   | L1 network (mainnet, sepolia, localhost) |
| `--l1-rpc-url`         | string  | No       | from config | L1 RPC endpoint URL                      |
| `--chain-name`         | string  | No       | from config | Initial chain name                       |
| `--chain-id`           | u64     | No       | from config | Initial chain ID                         |
| `--prover-mode`        | enum    | No       | no-proofs   | Prover mode (no-proofs, gpu)             |
| `--base-token-address` | address | No       | ETH         | Custom base token contract address       |
| `--wallet-creation`    | enum    | No       | random      | Wallet creation mode (random, provided)  |
| `--state-dir`          | path    | No       | from config | State directory path                     |

### Environment Variables
| Variable                   | Maps To        |
| -------------------------- | -------------- |
| `ADI_ECOSYSTEM_NAME`       | `--name`       |
| `ADI_L1_RPC_URL`           | `--l1-rpc-url` |
| `ADI_ECOSYSTEM_CHAIN_NAME` | `--chain-name` |
| `ADI_ECOSYSTEM_CHAIN_ID`   | `--chain-id`   |
| `ADI_STATE_DIR`            | `--state-dir`  |

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

Deploy ecosystem smart contracts to L1.

### Synopsis
```
adi deploy ecosystem [OPTIONS]
```

### Options
| Option         | Type   | Required | Default     | Description                   |
| -------------- | ------ | -------- | ----------- | ----------------------------- |
| `--name`       | string | No       | from config | Ecosystem name                |
| `--l1-rpc-url` | string | No       | from config | L1 RPC endpoint URL           |
| `--gas-price`  | u64    | No       | auto        | Gas price in wei              |
| `--dry-run`    | bool   | No       | false       | Simulate without broadcasting |
| `--auto-fund`  | bool   | No       | true        | Auto-fund wallets from funder |

### Environment Variables
| Variable                 | Maps To                     |
| ------------------------ | --------------------------- |
| `ADI_ECOSYSTEM_NAME`     | `--name`                    |
| `ADI_L1_RPC_URL`         | `--l1-rpc-url`              |
| `ADI_L1_GAS_PRICE`       | `--gas-price`               |
| `ADI_FUNDER_PRIVATE_KEY` | Funder wallet for auto-fund |

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

[SUCCESS] Ecosystem contracts deployed

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
- L1 RPC must be reachable
- zkstack CLI and forge must be available

### Postconditions
- All ecosystem contracts deployed to L1
- Contract addresses persisted to state
- Pending ownership transfers may exist

---

## adi init chain

Initialize and register a new chain within an ecosystem.

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
| `--l1-rpc-url`         | string  | No       | from config | L1 RPC endpoint URL       |
| `--gas-price`          | u64     | No       | auto        | Gas price in wei          |

### Output
**Success (exit 0):**
```
[INFO] Initializing chain 'adi' (ID: 222) in ecosystem 'adi_ecosystem'
[INFO] Deploying chain contracts...
[INFO] Registering chain with Bridgehub...

[SUCCESS] Chain 'adi' initialized and registered

Chain contracts:
  - Diamond Proxy: 0x9876...5432
  - Chain Admin: 0xfedc...ba98
  - Governance: 0x1111...2222

Chain configuration saved to:
  - chains/adi/configs/contracts.yaml
  - chains/adi/configs/wallets.yaml
  - chains/adi/configs/genesis.yaml
```

### Preconditions
- Parent ecosystem must be deployed
- Chain name must be unique within ecosystem
- Chain ID must not conflict with L1 or existing chains

### Postconditions
- Chain contracts deployed to L1
- Chain registered with Bridgehub
- Chain configuration persisted to state

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
| `--to`             | string | Yes      | -                | Target protocol version (e.g., v30)          |
| `--ecosystem-name` | string | No       | from config      | Ecosystem name                               |
| `--l1-rpc-url`     | string | No       | from config      | L1 RPC endpoint URL                          |
| `--gas-price`      | u64    | No       | auto             | Gas price in wei                             |
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
| `--to`             | string | Yes      | -                | Target protocol version   |
| `--chain-name`     | string | No       | from config      | Chain name                |
| `--ecosystem-name` | string | No       | from config      | Ecosystem name            |
| `--l1-rpc-url`     | string | No       | from config      | L1 RPC URL                |
| `--l2-rpc-url`     | string | No       | from config      | L2 RPC URL                |
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

To execute:
  1. Execute schedule calldata via chain admin
  2. Execute chain admin calldata

Or use --execute flag.
```

---

## adi accept ownership

Accept pending ownership transfers post-deployment.

### Synopsis
```
adi accept ownership [OPTIONS]
```

### Options
| Option             | Type   | Required | Default     | Description                 |
| ------------------ | ------ | -------- | ----------- | --------------------------- |
| `--ecosystem-name` | string | No       | from config | Ecosystem name              |
| `--chain-name`     | string | No       | all chains  | Specific chain name         |
| `--l1-rpc-url`     | string | No       | from config | L1 RPC URL                  |
| `--dry-run`        | bool   | No       | false       | Show what would be accepted |

### Output
**Success (exit 0):**
```
[INFO] Checking pending ownership transfers...

Pending transfers found:
  - Server Notifier: pending from 0x1111... to 0x2222...
  - Validator Timelock: pending from 0x3333... to 0x4444...
  - Verifier: pending from 0x5555... to 0x6666...

[PROGRESS] Accepting Server Notifier ownership...
[SUCCESS] Server Notifier ownership accepted

[PROGRESS] Accepting Validator Timelock ownership...
[SUCCESS] Validator Timelock ownership accepted

[PROGRESS] Accepting Verifier ownership...
[SUCCESS] Verifier ownership accepted

[SUCCESS] All pending ownership transfers accepted
```

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
