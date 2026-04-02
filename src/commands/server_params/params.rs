//! Server parameter extraction logic.

use adi_types::{ChainContracts, ChainMetadata, ProverMode, Wallets};
use alloy_primitives::Address;
use serde_json::Value;

use super::constants::{
    APP_BIN_UNPACK_PATH, BATCH_TIMEOUT, BLOCKS_PER_BATCH_LIMIT, BLOCK_DUMP_PATH, BLOCK_TIME,
    FUSAKA_UPGRADE_TIMESTAMP, GENESIS_INPUT_PATH, L2_BASE_FEE_OVERRIDE,
    L2_MAX_FEE_PER_BLOB_GAS_GWEI, L2_MAX_FEE_PER_GAS_GWEI, L2_MAX_PRIORITY_FEE_GWEI,
    L2_NATIVE_PRICE_OVERRIDE, L3_BASE_FEE_OVERRIDE, L3_MAX_FEE_PER_GAS_GWEI,
    L3_MAX_PRIORITY_FEE_GWEI, L3_NATIVE_PRICE_OVERRIDE, L3_PUBDATA_PRICE_OVERRIDE,
    MAX_IN_FLIGHT_BLOCKS, MAX_TXS_IN_BLOCK, OBJECT_STORE_BASE_PATH, POLL_INTERVAL, PROVER_API_ADDR,
    ROCKS_DB_PATH, RUST_LOG_VALUE,
};

/// Server parameter with its environment variable name and value.
pub(super) struct ServerParam {
    pub env_name: &'static str,
    pub value: Option<Value>,
    pub description: &'static str,
}

/// Helper to create a string Value.
fn str_val(s: &str) -> Option<Value> {
    Some(Value::String(s.to_string()))
}

/// Helper to create a u64 Value.
fn num_val(n: u64) -> Option<Value> {
    Some(serde_json::json!(n))
}

/// Input data for extracting server parameters.
pub(super) struct ServerParamsInput<'a> {
    pub contracts: &'a ChainContracts,
    pub wallets: &'a Wallets,
    pub chain_metadata: &'a ChainMetadata,
    pub rpc_url: Option<&'a str>,
    pub blobs: bool,
    pub prover_mode: ProverMode,
    pub genesis_base64: Option<String>,
    pub fee_collector_address: Option<Address>,
}

/// Extract server parameters from the given input.
pub(super) fn extract(input: &ServerParamsInput<'_>) -> Vec<ServerParam> {
    let mut params = vec![
        ServerParam {
            env_name: "RUST_LOG",
            value: str_val(RUST_LOG_VALUE),
            description: "Logging configuration",
        },
        ServerParam {
            env_name: "general_l1_rpc_url",
            value: input.rpc_url.map(|s| Value::String(s.to_string())),
            description: "Settlement layer RPC URL",
        },
        ServerParam {
            env_name: "general_rocks_db_path",
            value: str_val(ROCKS_DB_PATH),
            description: "RocksDB storage path",
        },
        ServerParam {
            env_name: "genesis_chain_id",
            value: num_val(input.chain_metadata.chain_id),
            description: "Chain ID",
        },
        ServerParam {
            env_name: "genesis_bridgehub_address",
            value: input
                .contracts
                .ecosystem_contracts
                .as_ref()
                .and_then(|c| c.bridgehub_proxy_addr)
                .map(|addr| Value::String(format!("{addr}"))),
            description: "Bridgehub proxy contract address",
        },
        ServerParam {
            env_name: "genesis_bytecode_supplier_address",
            value: input
                .contracts
                .ecosystem_contracts
                .as_ref()
                .and_then(|c| c.l1_bytecodes_supplier_addr)
                .map(|addr| Value::String(format!("{addr}"))),
            description: "L1 bytecodes supplier contract address",
        },
        ServerParam {
            env_name: "genesis_genesis_input_path",
            value: str_val(GENESIS_INPUT_PATH),
            description: "Genesis input file path",
        },
        ServerParam {
            env_name: "l1_sender_fusaka_upgrade_timestamp",
            value: num_val(FUSAKA_UPGRADE_TIMESTAMP),
            description: "Fusaka upgrade timestamp",
        },
        ServerParam {
            env_name: "l1_sender_operator_commit_pk",
            value: input
                .wallets
                .operator
                .as_ref()
                .map(|w| Value::String(w.expose_private_key().to_string())),
            description: "Operator private key (commit batches)",
        },
        ServerParam {
            env_name: "l1_sender_operator_prove_pk",
            value: input
                .wallets
                .prove_operator
                .as_ref()
                .map(|w| Value::String(w.expose_private_key().to_string())),
            description: "Prove operator private key",
        },
        ServerParam {
            env_name: "l1_sender_operator_execute_pk",
            value: input
                .wallets
                .execute_operator
                .as_ref()
                .map(|w| Value::String(w.expose_private_key().to_string())),
            description: "Execute operator private key",
        },
        ServerParam {
            env_name: "l1_sender_poll_interval",
            value: str_val(POLL_INTERVAL),
            description: "L1 sender poll interval",
        },
        ServerParam {
            env_name: "l1_watcher_poll_interval",
            value: str_val(POLL_INTERVAL),
            description: "L1 watcher poll interval",
        },
        ServerParam {
            env_name: "prover_api_address",
            value: str_val(PROVER_API_ADDR),
            description: "Prover API listen address",
        },
        ServerParam {
            env_name: "prover_api_component_enabled",
            value: str_val("true"),
            description: "Enable prover API component",
        },
        ServerParam {
            env_name: "prover_api_object_store_file_backed_base_path",
            value: str_val(OBJECT_STORE_BASE_PATH),
            description: "Prover object store base path",
        },
        ServerParam {
            env_name: "prover_input_generator_app_bin_unpack_path",
            value: str_val(APP_BIN_UNPACK_PATH),
            description: "App binary unpack path",
        },
        ServerParam {
            env_name: "prover_input_generator_maximum_in_flight_blocks",
            value: str_val(MAX_IN_FLIGHT_BLOCKS),
            description: "Max in-flight blocks for prover input",
        },
        ServerParam {
            env_name: "sequencer_block_dump_path",
            value: str_val(BLOCK_DUMP_PATH),
            description: "Block dump path",
        },
        ServerParam {
            env_name: "sequencer_block_time",
            value: str_val(BLOCK_TIME),
            description: "Block time interval",
        },
        ServerParam {
            env_name: "sequencer_max_transactions_in_block",
            value: num_val(MAX_TXS_IN_BLOCK),
            description: "Max transactions per block",
        },
        ServerParam {
            env_name: "sequencer_fee_collector_address",
            value: input
                .fee_collector_address
                .map(|a| Value::String(format!("{a}"))),
            description: "Fee collector address",
        },
        ServerParam {
            env_name: "batcher_batch_timeout",
            value: str_val(BATCH_TIMEOUT),
            description: "Batcher batch timeout",
        },
        ServerParam {
            env_name: "batcher_blocks_per_batch_limit",
            value: num_val(BLOCKS_PER_BATCH_LIMIT),
            description: "Blocks per batch limit",
        },
    ];

    // Only include genesis when available (json/upload mode)
    if let Some(genesis) = &input.genesis_base64 {
        params.push(ServerParam {
            env_name: "genesis",
            value: Some(Value::String(genesis.clone())),
            description: "Base64-encoded compact genesis JSON",
        });
    }

    // Prover mode conditional fields
    let fake_provers_enabled = match input.prover_mode {
        ProverMode::NoProofs => "true",
        ProverMode::Gpu => "false",
    };
    params.extend([
        ServerParam {
            env_name: "prover_api_fake_snark_provers_enabled",
            value: str_val(fake_provers_enabled),
            description: "Enable fake SNARK provers",
        },
        ServerParam {
            env_name: "prover_api_fake_fri_provers_enabled",
            value: str_val(fake_provers_enabled),
            description: "Enable fake FRI provers",
        },
    ]);

    // L2/L3 conditional fields
    if input.blobs {
        // L2 mode
        params.extend([
            ServerParam {
                env_name: "sequencer_base_fee_override",
                value: str_val(L2_BASE_FEE_OVERRIDE),
                description: "Sequencer base fee override (L2)",
            },
            ServerParam {
                env_name: "l1_sender_max_fee_per_gas_gwei",
                value: num_val(L2_MAX_FEE_PER_GAS_GWEI),
                description: "Max fee per gas in gwei (L2)",
            },
            ServerParam {
                env_name: "l1_sender_max_priority_fee_per_gas_gwei",
                value: num_val(L2_MAX_PRIORITY_FEE_GWEI),
                description: "Max priority fee per gas in gwei (L2)",
            },
            ServerParam {
                env_name: "sequencer_native_price_override",
                value: str_val(L2_NATIVE_PRICE_OVERRIDE),
                description: "Sequencer native price override (L2)",
            },
            ServerParam {
                env_name: "l1_sender_max_fee_per_blob_gas_gwei",
                value: num_val(L2_MAX_FEE_PER_BLOB_GAS_GWEI),
                description: "Max fee per blob gas in gwei (L2)",
            },
        ]);
    } else {
        // L3 mode
        params.extend([
            ServerParam {
                env_name: "l1_sender_pubdata_mode",
                value: str_val("Calldata"),
                description: "Pubdata sending mode (Calldata for L3)",
            },
            ServerParam {
                env_name: "sequencer_base_fee_override",
                value: str_val(L3_BASE_FEE_OVERRIDE),
                description: "Sequencer base fee override (L3)",
            },
            ServerParam {
                env_name: "l1_sender_max_fee_per_gas_gwei",
                value: num_val(L3_MAX_FEE_PER_GAS_GWEI),
                description: "Max fee per gas in gwei (L3)",
            },
            ServerParam {
                env_name: "l1_sender_max_priority_fee_per_gas_gwei",
                value: num_val(L3_MAX_PRIORITY_FEE_GWEI),
                description: "Max priority fee per gas in gwei (L3)",
            },
            ServerParam {
                env_name: "sequencer_native_price_override",
                value: str_val(L3_NATIVE_PRICE_OVERRIDE),
                description: "Sequencer native price override (L3)",
            },
            ServerParam {
                env_name: "sequencer_pubdata_price_override",
                value: str_val(L3_PUBDATA_PRICE_OVERRIDE),
                description: "Sequencer pubdata price override (L3)",
            },
        ]);
    }

    params
}

/// Display a Value as a string for UI rendering.
pub(super) fn display_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use adi_types::{ChainContracts, ChainMetadata, ProverMode, Wallets};
    use std::collections::HashMap;

    fn default_metadata() -> ChainMetadata {
        serde_yaml::from_str(
            r#"
id: 1
name: test_chain
chain_id: 99980
prover_version: NoProofs
l1_network: Sepolia
link_to_code: /code
configs: /configs
rocks_db_path: /db
artifacts_path: /artifacts
l1_batch_commit_data_generator_mode: Rollup
base_token:
  address: "0x0000000000000000000000000000000000000001"
  nominator: 1
  denominator: 1
wallet_creation: Random
evm_emulator: false
tight_ports: false
vm_option: ZKSyncOsVM
contracts_path: /contracts
default_configs_path: /defaults
"#,
        )
        .unwrap()
    }

    fn make_input(blobs: bool, prover_mode: ProverMode) -> ServerParamsInput<'static> {
        let contracts: &'static ChainContracts = Box::leak(Box::new(ChainContracts::default()));
        let wallets: &'static Wallets = Box::leak(Box::new(Wallets::default()));
        let metadata: &'static ChainMetadata = Box::leak(Box::new(default_metadata()));

        ServerParamsInput {
            contracts,
            wallets,
            chain_metadata: metadata,
            rpc_url: Some("http://localhost:8545"),
            blobs,
            prover_mode,
            genesis_base64: Some("dGVzdA==".to_string()),
            fee_collector_address: Some(Address::ZERO),
        }
    }

    fn to_map(params: &[ServerParam]) -> HashMap<&str, Option<Value>> {
        params
            .iter()
            .map(|p| (p.env_name, p.value.clone()))
            .collect()
    }

    #[test]
    fn l2_mode_includes_blob_gas_excludes_pubdata_mode() {
        let input = make_input(true, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(
            map["l1_sender_max_fee_per_blob_gas_gwei"],
            num_val(L2_MAX_FEE_PER_BLOB_GAS_GWEI)
        );
        assert_eq!(
            map["sequencer_base_fee_override"],
            str_val(L2_BASE_FEE_OVERRIDE)
        );
        assert_eq!(
            map["sequencer_native_price_override"],
            str_val(L2_NATIVE_PRICE_OVERRIDE)
        );
        assert!(!map.contains_key("l1_sender_pubdata_mode"));
        assert!(!map.contains_key("sequencer_pubdata_price_override"));
    }

    #[test]
    fn l3_mode_includes_pubdata_mode_excludes_blob_gas() {
        let input = make_input(false, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(map["l1_sender_pubdata_mode"], str_val("Calldata"));
        assert_eq!(
            map["sequencer_pubdata_price_override"],
            str_val(L3_PUBDATA_PRICE_OVERRIDE)
        );
        assert_eq!(
            map["sequencer_base_fee_override"],
            str_val(L3_BASE_FEE_OVERRIDE)
        );
        assert_eq!(
            map["sequencer_native_price_override"],
            str_val(L3_NATIVE_PRICE_OVERRIDE)
        );
        assert!(!map.contains_key("l1_sender_max_fee_per_blob_gas_gwei"));
    }

    #[test]
    fn prover_mode_noproofs_enables_fake_provers() {
        let input = make_input(true, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(
            map["prover_api_fake_snark_provers_enabled"],
            str_val("true")
        );
        assert_eq!(map["prover_api_fake_fri_provers_enabled"], str_val("true"));
    }

    #[test]
    fn prover_mode_gpu_disables_fake_provers() {
        let input = make_input(true, ProverMode::Gpu);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(
            map["prover_api_fake_snark_provers_enabled"],
            str_val("false")
        );
        assert_eq!(map["prover_api_fake_fri_provers_enabled"], str_val("false"));
    }

    #[test]
    fn numeric_fields_are_numbers() {
        let input = make_input(true, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert!(map["genesis_chain_id"].as_ref().unwrap().is_u64());
        assert!(map["l1_sender_fusaka_upgrade_timestamp"]
            .as_ref()
            .unwrap()
            .is_u64());
        assert!(map["sequencer_max_transactions_in_block"]
            .as_ref()
            .unwrap()
            .is_u64());
        assert!(map["l1_sender_max_fee_per_gas_gwei"]
            .as_ref()
            .unwrap()
            .is_u64());
    }

    #[test]
    fn static_fields_present() {
        let input = make_input(true, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(map["RUST_LOG"], str_val(RUST_LOG_VALUE));
        assert_eq!(map["general_rocks_db_path"], str_val(ROCKS_DB_PATH));
        assert_eq!(map["batcher_batch_timeout"], str_val(BATCH_TIMEOUT));
        assert_eq!(map["sequencer_block_time"], str_val(BLOCK_TIME));
        assert_eq!(map["l1_sender_poll_interval"], str_val(POLL_INTERVAL));
    }
}
