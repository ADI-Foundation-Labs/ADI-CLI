# v29 Ecosystem Deployment on Sepolia/Anvil

Area: Deployment
Created by: Mykhailo Slyvka
Edited at: January 21, 2026 4:05 PM
Type: Tutorial
Status: Done
zkOS Execution Version: 4

Let’s work in some dir with path `/home/ubuntu/dry_run`.

# Ecosystem

1. Install latest zkstack:

    ```bash
    cd dry_run
    git clone https://github.com/matter-labs/zksync-era.git zksync-era-main
    cd zksync-era-main
    # we need to have zkstack from `main` branch, for v29 better use commit 7c4c428b1ea3fd75d9884f3e842fb12d847705c1 of main branch, so
    # we don't use the latest (that is used for v30 ecosystem),
    # and we don't use `zksync-os-integration` branch (for v29 ecosystem),
    # otherwise - random errors will happen, trust us!
    git checkout 7c4c428b1ea3fd75d9884f3e842fb12d847705c1
    cargo install --path zkstack_cli/crates/zkstack --force --locked
    ```

2. Checkout zksync-era to `zksync-os-integration (rev a135c3b09913d49a1323b44ab80e715616934fc7)` and update submodules:
    1. `git fetch`
    2. `git checkout a135c3b09913d49a1323b44ab80e715616934fc7`(run `git stash` before the checkout if `yarn.lock` was modified during `cargo install`)
    3. `git submodule update --init --recursive`
    4. `cd contracts`

    1. `git checkout zkos-v0.29.11`- checkout to proper zkos version of `era-contracts` (https://github.com/matter-labs/era-contracts/releases)
    2. `git submodule update --init --recursive`
3. Copy zksync os `genesis.json` from **[genesis.json](https://raw.githubusercontent.com/matter-labs/zksync-os-server/ec996154d7cb0f3bd2857ff015d061781a9fbbe6/genesis/genesis.json) (v4)** to `zksync-era` dir. The file must be created on path `/home/ubuntu/dry_run/zksync-era/etc/env/file_based/genesis.json`
    1. `cd ..`
    2. `nano etc/env/file_based/genesis.json`
    3. insert file
4. Now zkstack and contracts are ready for deployment so we can start creating an ecosystem
    1. `cd ..`
    2. (Optional, if you do not have foundry-zksync) -  `foundryup-zksync` (it installs `forge` by zksync)
    3. Create ecosystem configuration (it’s a set of smart contracts and EOAs)

        ```bash
        zkstack ecosystem create --zksync-os -v
        ```

        ## Sepolia

        Use this to one liner to skip manual parameter selection, but

        1. pay attention on the ***code location*** parameter,
        2. pay attention to the ***prover mode*** (must be no-proofs for setup without a real GPU),

        ```bash
        zkstack ecosystem create --zksync-os -v --ecosystem-name dry_run_ecosystem --l1-network sepolia --link-to-code /home/ubuntu/dry_run/zksync-era --chain-name adi --chain-id 222 --prover-mode no-proofs --l1-batch-commit-data-generator-mode rollup --wallet-creation random --base-token-address 0x2a98B46fe31BA8Be05ef1cE3D36e1f80Db04190D --base-token-price-nominator 1 --base-token-price-denominator 1 --evm-emulator false --start-containers false
        ```

        | What do you want to name the ecosystem?                       | dry_run_ecosystem                          |
        | ------------------------------------------------------------- | ------------------------------------------ |
        | Select the origin of zksync-era repository                    | I have the code already                    |
        | Where's the code located?                                     | /home/ubuntu/dry_run/zksync-era            |
        | Select the L1 network                                         | Sepolia                                    |
        | What do you want to name the chain?                           | dry_run_chain                              |
        | What's the chain id?                                          | 222                                        |
        | Select how do you want to create the wallet                   | Random                                     |
        | Select the prover mode                                        | Gpu/NoProofs                               |
        | (NoProofs for testnet without GPU)                            |
        | Select the commit data generator mode                         | Rollup                                     |
        | Select the base token to use                                  | Custom                                     |
        | What is the token address?                                    | 0x2a98B46fe31BA8Be05ef1cE3D36e1f80Db04190D |
        | What is the base token price nominator?                       | 1                                          |
        | What is the base token price denominator?                     | 1                                          |
        | Enable EVM emulator?                                          | No                                         |
        | Do you want to start containers after creating the ecosystem? | No                                         |

        ## **Local (Anvil)**

        | What do you want to name the ecosystem?                       | dry_run_ecosystem               |
        | ------------------------------------------------------------- | ------------------------------- |
        | Select the origin of zksync-era repository                    | I have the code already         |
        | Where's the code located?                                     | /home/ubuntu/dry_run/zksync-era |
        | Select the L1 network                                         | Localhost                       |
        | What do you want to name the chain?                           | dry_run_chain                   |
        | What's the chain id?                                          | 222                             |
        | Select how do you want to create the wallet                   | Random                          |
        | Select the prover mode                                        | NoProofs                        |
        | Select the commit data generator mode                         | Rollup                          |
        | Select the base token to use                                  | ETH                             |
        | Enable EVM emulator?                                          | No                              |
        | Do you want to start containers after creating the ecosystem? | No                              |
5. We need to deposit wallets that will be used during deployment & rollup operational wallets.
    1. Deployment wallets
        1. Ecosystem wallets located on `~/dry_run/dry_run_ecosystem/configs/wallets.yaml`:
            - `deployer` - deposit 1 ETH
            - `governor` - deposit 1 ETH & 5 ADI
        2. Chain wallets located on `~/dry_run/dry_run_ecosystem/chains/dry_run_chain/configs/wallets.yaml` :
            - `governor` - deposit 1 ETH & 5 ADI
    2. Operational wallets located on `~/dry_run/dry_run_ecosystem/chains/dry_run_chain/configs/wallets.yaml` :
        - `operator` - deposit 5 ETH (if chain will be long-live)
        - `prove_operator` - deposit 5 ETH  (if chain will be long-live)
        - `execute_operator` - deposit 5 ETH  (if chain will be long-live)

Alternatively, on the Local (Anvil) deployment, we can use script [`fund.sh`](http://fund.sh) that will fund our accounts:

```bash
RPC_URL=http://localhost:8545
PRIVKEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
find . -type f -name 'wallets.yaml' | while read -r file; do
  echo "Processing $file …"

  # extract all addresses (strips leading spaces and the "address:" prefix)
  grep -E '^[[:space:]]*address:' "$file" \
    | sed -E 's/^[[:space:]]*address:[[:space:]]*//' \
    | while read -r addr; do

      if [[ $addr =~ ^0x[0-9a-fA-F]{40}$ ]]; then
        echo "→ Sending 10 ETH to $addr"
        cast send "$addr" \
          --value 10ether \
          --private-key "$PRIVKEY" \
          --rpc-url "$RPC_URL"
      else
        echo "⚠️  Skipping invalid address: '$addr'" >&2
      fi

    done
done
```

- Save it to ecosystem dir - `/home/ubuntu/local/dry_run_ecosystem/fund.sh`
- Give permission - `chmod +x [fund.sh](http://fund.sh/)`
- Execute script so all wallets will be funded - `./fund.sh`
1. (Optional - if ownership will be transferred). Prepare the wallets that will be new ecosystem & chain owners:
    1. {newEcosystemOwner}
    2. {newChainOwner}
2. Deposit accounts so we can approve ownership later
3. (Optional - if ownership will be transfered). We need to set excplicit Dual Verifier owner before the deployment, because it cannot be changed later without the upgrade.
    1. in file `~/dry_run/zksync-era/contracts/l1-contracts/deploy-scripts/DeployUtils.s.sol` on line 426. change `config.ownerAddress` {newEcosystemOwner} address.

        ```
         } else if (compareStrings(contractName, "Verifier")) {
        			 ...
                return
                    abi.encode(
                        addresses.stateTransition.verifierFflonk,
                        addresses.stateTransition.verifierPlonk,
                        config.ownerAddress -> Change to {newEcosystemOwner} address
                    );
        ```


4. Change create2 tx gas limit so it can be executed on Sepolia after Fusaka update:
    1. in file `~/dry_run/zksync-era/contracts/l1-contracts/deploy-scripts/Utils.sol` on line 288 change `gas: 20_000_000` to `gas: 16_700_000`

        ```diff
        --- a/l1-contracts/deploy-scripts/Utils.sol
        +++ b/l1-contracts/deploy-scripts/Utils.sol
        @@ -285,7 +285,7 @@ library Utils {
                 }

                 vm.broadcast();
        -        (bool success, bytes memory data) = _factory.call{gas: 20_000_000}(abi.encodePacked(_salt, _bytecode));
        +        (bool success, bytes memory data) = _factory.call{gas: 16_700_000}(abi.encodePacked(_salt, _bytecode));
                 contractAddress = bytesToAddress(data);
        ```

5. 🚀 **Init ecosystem** and deploy smart contracts on L1:
    1. `cd dry_run_ecosystem`
    2. `zkstack ecosystem init` like this (keep in mind that on Sepolia gas price could be higher, so better to check current gas price and set proper `--with-gas-price`):

        ```python
        zkstack ecosystem init --zksync-os -v --update-submodules false --ignore-prerequisites -a --with-gas-price -a 10000000000
        ```

        ([check Sepolia current gas price](https://sepolia.etherscan.io/)!)

    3. Answer the questions


        | Do you want to deploy some test ERC20s?                                                                                                | No                                                |     |
        | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- | --- |
        | What is the RPC URL of the network?                                                                                                    | {Provide Sepolia RPC URL with **no rate limits**} |     |
        | Do you want to setup observability? (Grafana)                                                                                          | No                                                |     |
        | Do you want to deploy ecosystem contracts? (Not needed if you already have an existing one)                                            | Yes                                               |     |
        | It is recommended to have 5.00… ETH on the address 0x…. to deploy contracts. Current balance is 1.00… ETH. How do you want to proceed? | Proceed with the deployment anyway                |     |
        | (answer multiple times)                                                                                                                | Proceed with the deployment anyway                |     |
        | Do you want to deploy Paymaster contract?                                                                                              | No                                                |     |

    ⚠ In case of any contract deployment error:

    ```bash
    mv ~/dry_run_ecosystem ~/dry_run_ecosystem_bkp

    # Re-create ecosystem again (answer the questions as in the step 4)
    zkstack ecosystem create --zksync-os -v

    # Re-use the same previous EOA's to avoid topping them up
    cp ~/dry_run_ecosystem_bkp/configs/wallets.yaml ~/dry_run_ecosystem/configs/
    cp ~/dry_run_ecosystem_bkp/chains/dry_run_chain/configs/wallets.yaml ~/dry_run_ecosystem/chains/dry_run_chain/configs/
    ```

6. **Deployment may leave unfinished pending ownership which need to be transferred**
Contracts addresses could be found in file `~/dry_run/dry_run_ecosystem/chains/dry_run_chain/configs/contracts.yaml` :
    1. Accept pending ownership after the deployment:
        - Server Notifier:

        ```bash
        	SEPOLIA_RPC={https://SEPOLIA_RPC_URL}
        ```

        ```bash
        ACCEPT_OWNERSHIP_DATA=$(cast calldata "acceptOwnership()")
        ```

        ```jsx
        MULTICALL_DATA=$(cast calldata "multicall((address,uint256,bytes)[],bool)" \
        "[( {ecosystem_contracts.server_notifier_proxy_addr} ,0,$ACCEPT_OWNERSHIP_DATA)]" \
        true)
        ```

        ```bash
        cast send {ecosystem_contracts.chain_admin} \
        $MULTICALL_DATA \
        --private-key={ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}\
        --rpc-url $SEPOLIA_RPC
        ```

        - RollupDA Manager:

        ```bash
        cd ../zksync-era-main/contracts/l1-contracts
        ```

        ```bash
        forge script deploy-scripts/AdminFunctions.s.sol \
          --sig "governanceAcceptOwner(address,address)" \
          {ecosystem_contracts.governance_not_governer} \
          {ecosystem_contracts.l1_rollup_da_manager} \
          --private-key "{ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}" \
          --broadcast \
          --rpc-url "$SEPOLIA_RPC"
        ```

        - Validator Timelock:

        ```bash
        cast send {l1.validator_timelock_addr} "acceptOwnership()" \
          --rpc-url "$SEPOLIA_RPC" \
          --private-key "{ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}"
        ```

        - Verifier:

        ```bash
        cast send {l1.verifier_addr} "acceptOwnership()" \
        --rpc-url $SEPOLIA_RPC \
        --private-key={ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}
        ```

    2. (Optional) Send ecosystem ownership:
        - Ecosystem Governance:

        ```jsx
        cast send {ecosystem_contracts.governance} \
        "transferOwnership(address)" \
        {newEcosystemOwner Address} \
        --rpc-url $SEPOLIA_RPC \
        --private-key={ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}
        ```

        - Ecosystem Chain Admin

        ```bash
        cast send {ecosystem_contracts.chain_admin} \
        "transferOwnership(address)" \
        {newEcosystemOwner Address} \
        --rpc-url $SEPOLIA_RPC \
        --private-key={ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}
        ```

        - Ecosystem Bridged Token Beacon

        ```bash
        1. Get Bridged Token Beacon:
        cast call {ecosystem_contracts.native_token_vault_addr}\
        "bridgedTokenBeacon()(address)"\
        --rpc-url $SEPOLIA_RPC

        2. Pass address that was returned from previous call
        cast send {bridgedTokenBeacon} \
        "transferOwnership(address)" \
        {newEcosystemOwner Address} \
        --rpc-url $SEPOLIA_RPC \
        --private-key={ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}
        ```

        - Ecosystem Validator Timelock

        ```bash
        cast send {ecosystem_contracts.validator_timelock_addr} \
        "transferOwnership(address)" \
        {newEcosystemOwner Address} \
        --rpc-url $SEPOLIA_RPC \
        --private-key={ecosystemGovernorPrivKey (from /configs/wallets.yaml.{governor})}
        ```

    3. Accept ecosystem ownership using newEcosystemOwner Private Key.

        NOTE: Bridged Token Beacon do not inherit Ownable2Step, it has just Ownable, so we do not need to accept it’s ownership.

        - Ecosystem Governance

        ```bash
        cast send {ecosystem_contracts.governance} \
        "acceptOwnership()" \
        --rpc-url $SEPOLIA_RPC \
        --private-key {newEcosystemOwner Private Key}
        ```

        - Ecosystem Chain Admin

        ```bash
        cast send {ecosystem_contracts.chain_admin} \
        "acceptOwnership()" \
        --rpc-url $SEPOLIA_RPC \
        --private-key {newEcosystemOwner Private Key}
        ```

        - Ecosystem Validator Timelock

        ```bash
        cast send {ecosystem_contracts.validator_timelock_addr} \
        "acceptOwnership()" \
        --rpc-url $SEPOLIA_RPC \
        --private-key {newEcosystemOwner Private Key}
        ```

    4. Send chain ownership:
        - Chain Governance

        ```bash
        cast send {l1.governance_addr} \
        "transferOwnership(address)" \
        {newChainOwner Address}
        --rpc-url $SEPOLIA_RPC \
        --private-key={chainGovernorPrivKey (from /chains/dry_run_chain/configs/wallets.yaml.{governor})}
        ```

        - Chain Chain Admin

        ```bash
        cast send {l1.chain_admin_addr} \
        "transferOwnership(address)" \
        {newChainOwner Address}
        --rpc-url $SEPOLIA_RPC \
        --private-key={chainGovernorPrivKey (from /chains/dry_run_chain/configs/wallets.yaml.{governor})}
        ```

    5. Accept chain ownership using newChainOwner Private Key:
        - Chain Governance

        ```bash
        cast send {l1.governance_addr} \
        "acceptOwnership()" \
        --rpc-url $SEPOLIA_RPC \
        --private-key {newChainOwner Private Key}
        ```

        - Chain Chain Admin

        ```bash
        cast send {l1.chain_admin_addr} \
        "acceptOwnership()" \
        --rpc-url $SEPOLIA_RPC \
        --private-key {newChainOwner Private Key}
        ```


1. Register Plonk and Fflonk verifiers in DualVerifier for execution version **4** (**according to genesis.json!**):
    - Get Plonk and Fflonk verifiers:

        ```bash
        cast call {ecosystem_contracts.verifier_addr} "plonkVerifiers(uint32)(address)"   0 --rpc-url $SEPOLIA_RPC
        -> Will return {plonkVerifierAddr}
        ```

        ```bash
        cast call {ecosystem_contracts.verifier_addr} "fflonkVerifiers(uint32)(address)"   0 --rpc-url $SEPOLIA_RPC
        -> Will return {fflonkVerifierAddr}
        ```

    - Register verifier for a new version (e.g. 4):

        ```bash
        cast send {ecosystem_contracts.verifier_addr} \
        "addVerifier(uint32,address,address)" \
        4 \
        {fflonkVerifierAddr} \
        {plonkVerifierAddr} \
        --rpc-url $SEPOLIA_RPC \
        --private-key {ecosystemOwner Private Key (L1 governor by default)}
        ```

2. Rollup is ready for running on server

---

# ZkSync OS Server

<aside>
💡

Running a server with fake proofs (without provers).

</aside>

1. Add `docker-compose-server.yml` file so we can run server with FAKE PROOFS:

    ```yaml
    version: "3.8"
    services:
      server:
        image: harbor.sre.ideasoft.io/adi-chain/server:v0.10.1-b6
        container_name: server
        restart: unless-stopped
        # network_mode: host
        user: "0:0"
        working_dir: /app
        ports:
          - "3050:3050"
        environment:
          # This flags make from server an External Node
          # sequencer_block_replay_download_address: "127.0.0.1:3153"
          # general_main_node_rpc_url: "http://127.0.0.1:3998"

          sequencer_base_fee_override: 1000
          sequencer_pubdata_price_override: 1
          sequencer_native_price_override: 1
          sequencer_block_dump_path: "/chain/db/block_dumps"

          general_rocks_db_path: "/chain/db/node1"
          general_l1_rpc_url: "http://127.0.0.1:8545"

          genesis_bridgehub_address: "0xf69daaea7f8578933237a9b59f42704ebec36ab9"
          genesis_chain_id: 222

          prover_api_object_store_file_backed_base_path: "/shared"
          prover_api_component_enabled: "false"
          prover_api_fake_fri_provers_enabled: "true"
          prover_api_fake_snark_provers_enabled: "true"
          prover_api_address: "0.0.0.0:3320"
          prover_input_generator_app_bin_unpack_path: "/chain/db/app_bins"
          prover_input_generator_maximum_in_flight_blocks: 120

          batcher_batch_timeout: "10s"
          batcher_blocks_per_batch_limit: 10

          l1_sender_operator_commit_pk: 0x38adba37e0fbb25c749c09c17d5363e03e6ecbb76cb654eef36ebcff4a55b5ac
          l1_sender_operator_prove_pk: 0xd8b77c558f2705b0cb081e36180903e4a6cc3a379b2a2318b942ac7c5d4f1540
          l1_sender_operator_execute_pk: 0x40bb928b2e3c5123516fe677918142582723c69b9eadeb0c83749222b83bfa70
          l1_sender_max_fee_per_gas_gwei: 2
          l1_sender_max_priority_fee_per_gas_gwei: 1

          RUST_LOG: "info,zksync_os_server=info,zksync_os_sequencer=info,zksync_os_merkle_tree=info,zksync_os_priority_tree=info"
        volumes:
          - ./volumes/chain:/chain
          - ./volumes/shared:/shared
    ```

    Change server parameters:

    | **Set `docker-compose-server.yaml` parameters** | **to values from `contracts.yaml` or `wallets.yaml`** | **found in the ecosystem paths**                                  |
    | ----------------------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------- |
    | `genesis_bridgehub_address`                     | {ecosystem_contracts.bridgehub_proxy_addr}            | `./dry_run_ecosystem/chains/dry_run_chain/configs/contracts.yaml` |
    | `l1_sender_operator_commit_pk`                  | {operator.private_key}                                | `./dry_run_ecosystem/chains/dry_run_chain/configs/wallets.yaml`   |
    | `l1_sender_operator_prove_pk`                   | {prove_operator.private_key}                          | `./dry_run_ecosystem/chains/dry_run_chain/configs/wallets.yaml`   |
    | `l1_sender_operator_execute_pk`                 | {execute_operator.private_key}                        | `./dry_run_ecosystem/chains/dry_run_chain/configs/wallets.yaml`   |

    | **Set `docker-compose-server.yaml` RPC URL** | **to values**                                                                 |
    | -------------------------------------------- | ----------------------------------------------------------------------------- |
    | `general_l1_rpc_url`                         | 1. local RPC URL is typically [http://localhost:8445](http://localhost:8445/) |

    2. remote RPC URL can be found in the docs (e.g. Sepolia RPC URL) |
2. Create volumes:

    ```bash
    mkdir -p volumes/chain volumes/shared
    ```

3. Run the server:

    ```bash
    docker compose -f docker-compose-server.yml up
    ```

    1. Verify that new blocks and transactions are accepted and fake proofs are generated:

    ```bash
    cast send 0x036903B8D85BBad12b110D96c532Ce58EFa38203  --value 1 --rpc-url http://127.0.0.1:3050 --private-key {governor.private_key from /home/ubuntu/local/dry_run_ecosystem/configs/wallets.yaml}
    ```

    New txs, blocks and batches must be created after you run the command above 10-15 times


## Troubleshooting

### 1. Commit of the 1st batch failed

The server (sequencer) always commits the very first (genesis) batch. It may fail.

**Cause**: `genesis.json` may be incompatible with the server version

**Measure**: re-create ecosystem from scratch (alas)

![image.png](v29%20Ecosystem%20Deployment%20on%20Sepolia%20Anvil/image.png)

### 2. …

## Running a chain with real proofs

You need to do several additional steps.

1. Add to `docker-compose.yml` prover images:

```bash
  ...existing docker-compose.yml

  fri-prover:
    image: ghcr.io/matter-labs/zksync-os-prover-fri:cd9af6e-1762442214302
    container_name: test_fri
    restart: unless-stopped
    depends_on: [server]
    environment:
      RUST_LOG: "debug"
    volumes:
      - prover:/prover
    command: ["--base-url","http://127.0.0.1:3320","--app-bin-path","/multiblock_batch.bin", "--enabled-logging"]
    gpus:
      - device_ids: ["MIG-2f381c76-8bd1-5074-98a5-e9e37d0e81a6"]
        capabilities: ["gpu"]
    network_mode: host
  snark-prover:
    image: ghcr.io/matter-labs/zksync-os-prover-snark:cd9af6e-1762442214930
    container_name: test_snark
    restart: unless-stopped
    depends_on: [server, fri-prover]
    environment:
      RUST_MIN_STACK: "267108864"
    volumes:
      - prover:/prover
    command: ["run-prover",
              "--sequencer-url","http://127.0.0.1:3320",
              "--binary-path","/multiblock_batch.bin",
              "--trusted-setup-file","/setup_compact.key",
              "--output-dir","/prover"]
    gpus:
      - device_ids: ["MIG-44e310b7-590f-5d79-b567-51023ba3535a"]
        capabilities: ["gpu"]
    network_mode: host

volumes:
	chain_data:
	shared:
	prover:
```

1. On step 5, during the creation of the ecosystem, instead of  “`Select the prover mode` - ~~NoProofs~~” → use “`Select the prover mode` - **Gpu**”
2. In `docker-compose.yml` set envs:
    - `prover_api_component_enabled`: "true"
    - `prover_api_fake_fri_provers_enabled`: "false"
    - `prover_api_fake_snark_provers_enabled`: "false"

---

# ADI Block Explorer

<aside>
💡

ADI Block Explorer is a fork of Matter Labs `block-explorer`.

</aside>

1. Add explorer config with path `/home/ubuntu/local/dry_run_ecosystem/configs/explorer.config.js`

    ```jsx
    window["##runtimeConfig"] = {
      appEnvironment: "default",
      environmentConfig: {
        networks: [
          {
            name: "Local Chain",
            l2NetworkName: "Local Chain",
            l2ChainId: 222,
            rpcUrl: "http://127.0.0.1:3050",
            apiUrl: "http://127.0.0.1:3002",
            hostnames: [],
            icon: "/images/icons/adi-logo.svg",
            maintenance: false,
            published: true,
            prividium: false,
          },
        ],
      },
    };
    ```

2. Add docker-compose file with path  `/home/ubuntu/local/dry_run_ecosystem/docker-compose-explorer.yml` for explorer services:

    <aside>
    💡

    Fix the `explorer-app`’s path to where your ecosystem is located: `/home/ubuntu/local/dry_run_ecosystem/configs/explorer.config.js`

    </aside>

    ```yaml
    services:
      postgres-explorer:
        image: "postgres:14"
        command: postgres -c 'max_connections=1000'
        ulimits:
          nofile:
            soft: 1048576
            hard: 1048576
        ports:
          - 127.0.0.1:5433:5432
        volumes:
          - type: volume
            source: postgres-explorer-data
            target: /var/lib/postgresql/data
        environment:
          - POSTGRES_PASSWORD=notsecurepassword
          - POSTGRES_DB=zksync_test_db

      data-fetcher:
        image: registry.sre.ideasoft.io/adi-foundation/l1.5/zksync-explorer/data-fetcher:latest
        platform: linux/amd64
        ports:
        - 3040:3040
        environment:
          PORT: '3040'
          NODE_ENV: development
          BLOCKCHAIN_RPC_URL: http://host.docker.internal:3050
          # BLOCKCHAIN_RPC_URL: http://10.150.16.13:3050
          LOG_LEVEL: verbose
        extra_hosts:
        - host.docker.internal:host-gateway

      api:
        image: registry.sre.ideasoft.io/adi-foundation/l1.5/zksync-explorer/api:latest
        platform: linux/amd64
        ports:
        - 3002:3002
        environment:
          NODE_ENV: development
          PRIVIDIUM: 'false'
          PORT: '3002'
          LOG_LEVEL: verbose
          DATABASE_HOST: postgres-explorer
          DATABASE_USER: postgres
          DATABASE_PASSWORD: notsecurepassword
          DATABASE_NAME: zksync_test_db
    # ENVs for Custom Base Token setup
          BASE_TOKEN_SYMBOL: ADI
          BASE_TOKEN_NAME: 'ADI Token'
          BASE_TOKEN_DECIMALS: 18
          BASE_TOKEN_L1_ADDRESS: 0x2a98B46fe31BA8Be05ef1cE3D36e1f80Db04190D
          ETH_TOKEN_L2_ADDRESS: 0x3Ca3cc93E135cCb14b64184CaD140F5434c58429
          BASE_TOKEN_ICON_URL: 'https://api-minio-cdn.adifoundation.ai/adi/the-logo.svg'
        depends_on:
        - postgres-explorer
        - worker
        extra_hosts:
        - host.docker.internal:host-gateway

      worker:
        image: registry.sre.ideasoft.io/adi-foundation/l1.5/zksync-explorer/worker:latest
        platform: linux/amd64
        environment:
          DATABASE_NAME: zksync_test_db
          NODE_ENV: development
          BLOCKCHAIN_RPC_URL: http://host.docker.internal:3050
          # BLOCKCHAIN_RPC_URL: http://10.150.16.13:3050
          DATABASE_HOST: postgres-explorer
          PORT: '3001'
          BATCHES_PROCESSING_POLLING_INTERVAL: '1000'
          LOG_LEVEL: verbose
          DATA_FETCHER_URL: http://data-fetcher:3040
          DATABASE_PORT: '5432'
          DATABASE_USER: postgres
          DATABASE_PASSWORD: notsecurepassword
        depends_on:
          - postgres-explorer
        extra_hosts:
        - host.docker.internal:host-gateway

      explorer-app:
        image: registry.sre.ideasoft.io/adi-foundation/l1.5/zksync-explorer/app:latest
        platform: linux/amd64
        ports:
          - "3010:3010"
        volumes:
          - /home/ubuntu/local/dry_run_ecosystem/configs/explorer.config.js:/usr/src/app/packages/app/dist/config.js
        environment:
          - PORT=3010
          - VITE_VERSION=1.0.0
        restart: unless-stopped

    name: test-services
    volumes:
      postgres-explorer-data:
    ```

3. Start Explorer - `docker compose -f docker-compose-explorer.yml up -d` . It will be accessible on address [http://127.0.0.1:3010/](http://10.150.16.13:3010/)
