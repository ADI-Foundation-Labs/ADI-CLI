# adi-cli

[Rust](https://www.rust-lang.org/)
[Docker Required](https://www.docker.com/)

SDK-first Rust CLI for managing ZkSync ecosystem smart contracts. Runs on the host machine and orchestrates pre-built Docker toolkit containers (zkstack, foundry-zksync, era-contracts).

## Prerequisites

- **Rust** — [Install via rustup](https://rustup.rs/)
- **Docker** — [Install Docker](https://www.docker.com/get-started/)

## Installation

### Quick Install (recommended)

The installer requires a GitLab personal access token with `**api`** scope.
[Create a token here](https://gitlab.sre.ideasoft.io/-/user_settings/personal_access_tokens?name=adi-cli-install&scopes=api).

```bash
export GITLAB_TOKEN="glpat-..."
curl -fsSL --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  "https://gitlab.sre.ideasoft.io/api/v4/projects/348/repository/files/install.sh/raw?ref=main" | bash
```

To install a specific version:

```bash
curl -fsSL --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  "https://gitlab.sre.ideasoft.io/api/v4/projects/348/repository/files/install.sh/raw?ref=main" | bash -s -- v0.1.0
```

The installer detects your OS and architecture, downloads the correct binary, and places it in `~/.cargo/bin/`.

### Install via Cargo

```bash
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --git ssh://git@gitlab.sre.ideasoft.io/adi-foundation/adi-chain/cli.git
```

### Building from Source

```bash
git clone ssh://git@gitlab.sre.ideasoft.io/adi-foundation/adi-chain/cli.git adi-cli
cd adi-cli
cargo build --release
cp ./target/release/adi ~/.local/bin/
```

### Verify

```bash
adi version
```

## Shell Completions

Enable tab-completion for commands, flags, and arguments:

**Bash:**

```bash
mkdir -p ~/.local/share/bash-completion/completions
adi completions bash > ~/.local/share/bash-completion/completions/adi
```

**Zsh (Oh My Zsh):**

```bash
mkdir -p ~/.oh-my-zsh/completions
adi completions zsh > ~/.oh-my-zsh/completions/_adi
```

**Fish:**

```bash
mkdir -p ~/.config/fish/completions
adi completions fish > ~/.config/fish/completions/adi.fish
```

Restart your shell or run `source ~/.zshrc` (for zsh) to activate.

## Configuration

Config is loaded from (highest priority first): `--config` flag → `ADI_CONFIG` env var → `~/.adi.yml`. Environment variables (`ADI__*`) and CLI flags override any file values.

### Minimal Config

```yaml
# ~/.adi.yml
protocol_version: v0.30.1

ecosystem:
  name: adi_ecosystem
  l1_network: sepolia
  rpc_url: https://sepolia.infura.io/v3/YOUR_KEY
  chains:
    - name: my-chain
      chain_id: 222
      prover_mode: no-proofs

funding:
  deployer_eth: 100.0
  governor_eth: 40.0

gas_multiplier: 200
```

All fields have sensible defaults. See `adi config` to inspect the merged configuration. For the full annotated config with all options, see [docs/configuration.md](docs/configuration.md).

### Environment Variables


| Variable                  | Purpose                                                      |
| ------------------------- | ------------------------------------------------------------ |
| `ADI_FUNDER_KEY`          | Private key (hex) of the wallet that funds ecosystem wallets |
| `ADI_PRIVATE_KEY`         | Private key (hex) for accepting ownership as new owner       |
| `ADI_RPC_URL`             | Settlement layer RPC endpoint                                |
| `ADI_EXPLORER_URL`        | Block explorer API URL for contract verification             |
| `ADI_EXPLORER_API_KEY`    | Block explorer API key (optional for public explorers)       |
| `ADI_CONFIG`              | Path to an alternative config file                           |
| `ADI__PROTOCOL_VERSION`   | Default protocol version (e.g., `v0.30.1`)                   |
| `ADI__TOOLKIT__IMAGE_TAG` | Override Docker image tag (e.g., `latest`)                   |
| `ADI_OPERATOR`            | Operator address (PRECOMMITTER, COMMITTER, REVERTER roles)   |
| `ADI_PROVE_OPERATOR`      | Prove operator address (PROVER role)                         |
| `ADI_EXECUTE_OPERATOR`    | Execute operator address (EXECUTOR role)                     |
| `AWS_ACCESS_KEY_ID`       | AWS access key for S3 state sync                             |
| `AWS_SECRET_ACCESS_KEY`   | AWS secret key for S3 state sync                             |
| `RUST_LOG`                | Logging verbosity: `error`, `warn`, `info`, `debug`, `trace` |


Override any config value using `ADI__` prefix with double underscores as path separators:

```bash
export ADI__ECOSYSTEM__NAME=production
export ADI__ECOSYSTEM__RPC_URL=http://localhost:8545
```

## Usage

Typical workflow: `init` → `deploy` → `verify`

### init — Create ecosystem

```bash
adi init
```

Generates wallet keys, ecosystem metadata, and initial chain config under `~/.adi_cli/state/`. All flags are optional and fall back to `~/.adi.yml`.

### deploy — Fund wallets, deploy contracts, and handle ownership

```bash
# Preview the funding plan
adi deploy --dry-run

# Execute
export ADI_FUNDER_KEY="0x..."
adi deploy
```

Runs three phases: funding, deployment, and ownership. Use `--skip-funding` or `--skip-deployment` to skip individual phases. `--gas-multiplier` controls gas price buffer (default 200 = 100% buffer, use 300 for Sepolia/congested networks).

**Post-deploy ownership** is automatic and depends on your `ownership` config:

| Config                      | Behavior                                                                                 |
| --------------------------- | ---------------------------------------------------------------------------------------- |
| No ownership configured     | Governor accepts ownership. Governor remains the owner.                                  |
| `new_owner` address only    | Governor accepts, then transfers to new owner. New owner must run `adi accept` manually. |
| `new_owner` + `private_key` | Governor accepts, transfers, and new owner accepts automatically. Fully hands-off.       |

Ownership can be configured at both ecosystem (`ecosystem.ownership`) and per-chain (`ecosystem.chains[].ownership`) levels.

**Local development with Anvil:**

```bash
anvil --block-base-fee-per-gas 1 --disable-min-priority-fee --disable-block-gas-limit
```

Set `l1_network: sepolia` and `rpc_url: http://host.docker.internal:8545` in config (Docker containers can't access `localhost` directly).

### accept — Accept ownership transfers

Standalone command for cases where acceptance wasn't done during deploy (e.g., new owner accepting after a transfer-only deploy).

```bash
# As new owner
export ADI_PRIVATE_KEY="0x..."
adi accept --chain my-chain --yes

# Export calldata for multisig submission
adi accept --calldata --use-governor
```

Use `--dry-run` to preview, `--chain <name>` to include chain-level contracts.

### transfer — Transfer ownership

Standalone command to transfer ownership outside of the deploy flow.

```bash
adi transfer --new-owner 0x1234...abcd --yes
```

Accepts pending transfers then calls `transferOwnership()` on each contract. The new owner must run `adi accept` afterward. Use `--dry-run` to preview.

### ecosystem — View ecosystem info

```bash
adi ecosystem
adi ecosystem --chain my-chain
```

### owners — View contract owners

```bash
adi owners
adi owners --chain my-chain
```

### verify — Verify contracts on block explorers

```bash
adi verify
adi verify --chain my-chain
```

### add — Add a chain to an existing ecosystem

```bash
adi add
```

Adds a new chain to an already deployed ecosystem. Use `--force` to overwrite an existing chain. After adding, run `deploy` again to deploy the new chain's contracts.

## State

All ecosystem state is persisted as YAML files under `~/.adi_cli/state/`:

```
~/.adi_cli/state/<ecosystem-name>/
├── ZkStack.yaml                    # Ecosystem metadata
├── configs/
│   ├── wallets.yaml                # Wallet addresses and keys (keep secure)
│   ├── contracts.yaml              # Deployed contract addresses
│   ├── initial_deployments.yaml
│   ├── erc20_deployments.yaml
│   └── apps.yaml
└── chains/<chain-name>/
    ├── ZkStack.yaml                # Chain metadata
    └── configs/
        ├── wallets.yaml
        ├── contracts.yaml
        ├── genesis.yaml
        ├── general.yaml
        └── secrets.yaml
```

Optional S3 synchronization is available for backup and sharing state across machines. Configure the `s3` section in `~/.adi.yml` to enable.

## Build toolkit image locally

With [Task](https://taskfile.dev):

```bash
task build:docker:local
task build:docker:local NO_CACHE=1 # build without cache, thanks Cap o7
```

