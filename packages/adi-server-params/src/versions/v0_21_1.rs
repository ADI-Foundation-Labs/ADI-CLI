//! Server v0.21.1 parameter extraction.

use adi_types::{ProverMode, PubdataMode, SettlementLayer};
use serde_json::Value;

use super::constants::{
    BASE_TOKEN_PRICE_UPDATER_ENABLED, BATCH_TIMEOUT, BLOCKS_PER_BATCH_LIMIT, BLOCK_DUMP_PATH,
    BLOCK_TIME, EXTERNAL_PRICE_API_CLIENT_SOURCE, FUSAKA_UPGRADE_TIMESTAMP, GENESIS_INPUT_PATH,
    L1_SENDER_REQUIRED_CONFIRMATIONS, L1_WATCHER_CONFIRMATIONS, MAX_FEE_PER_BLOB_GAS_GWEI,
    MAX_IN_FLIGHT_BLOCKS, MAX_TXS_IN_BLOCK, OBJECT_STORE_BASE_PATH, OBSERVABILITY_LOG_FORMAT,
    OBSERVABILITY_LOG_USE_COLOR, POLL_INTERVAL, PROVER_API_ADDR, PUBDATA_PRICE_OVERRIDE,
    ROCKS_DB_PATH, RUST_LOG_VALUE, SETTLE_L1_BASE_FEE_OVERRIDE, SETTLE_L1_MAX_FEE_PER_GAS_GWEI,
    SETTLE_L1_MAX_PRIORITY_FEE_GWEI, SETTLE_L1_NATIVE_PRICE_OVERRIDE, SETTLE_L2_BASE_FEE_OVERRIDE,
    SETTLE_L2_MAX_FEE_PER_GAS_GWEI, SETTLE_L2_MAX_PRIORITY_FEE_GWEI,
    SETTLE_L2_NATIVE_PRICE_OVERRIDE,
};
use super::{build_forced_prices_json, num_val, str_val};
use crate::params::{ServerParam, ServerParamsInput};

pub(super) fn extract(input: &ServerParamsInput<'_>) -> Vec<ServerParam> {
    let mut params = vec![
        ServerParam {
            env_name: "RUST_LOG",
            value: str_val(RUST_LOG_VALUE),
            description: "Logging configuration",
        },
        ServerParam {
            env_name: "l1_provider_rpc_url",
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
            env_name: "l1_sender_operator_commit_sk",
            value: input
                .wallets
                .operator
                .as_ref()
                .map(|w| Value::String(w.expose_private_key().to_string())),
            description: "Operator private key (commit batches)",
        },
        ServerParam {
            env_name: "l1_sender_operator_prove_sk",
            value: input
                .wallets
                .prove_operator
                .as_ref()
                .map(|w| Value::String(w.expose_private_key().to_string())),
            description: "Prove operator private key",
        },
        ServerParam {
            env_name: "l1_sender_operator_execute_sk",
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
            env_name: "prover_api_enabled",
            value: str_val("true"),
            description: "Enable prover API component",
        },
        ServerParam {
            env_name: "prover_api_proof_storage_path",
            value: str_val(OBJECT_STORE_BASE_PATH),
            description: "Prover object store base path",
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
        ServerParam {
            env_name: "external_price_api_client_source",
            value: str_val(EXTERNAL_PRICE_API_CLIENT_SOURCE),
            description: "External price API client source",
        },
        ServerParam {
            env_name: "base_token_price_updater_enabled",
            value: str_val(BASE_TOKEN_PRICE_UPDATER_ENABLED),
            description: "Enable base token price updater",
        },
        ServerParam {
            env_name: "observability_log_format",
            value: str_val(OBSERVABILITY_LOG_FORMAT),
            description: "Observability log format",
        },
        ServerParam {
            env_name: "observability_log_use_color",
            value: str_val(OBSERVABILITY_LOG_USE_COLOR),
            description: "Use color in observability logs",
        },
        ServerParam {
            env_name: "external_price_api_client_forced_prices__json",
            value: Some(Value::String(build_forced_prices_json(
                input.base_token_address,
            ))),
            description: "Forced prices JSON for external price API client",
        },
        ServerParam {
            env_name: "l1_watcher_confirmations",
            value: str_val(L1_WATCHER_CONFIRMATIONS),
            description: "L1 block confirmations required by the L1 watcher",
        },
        ServerParam {
            env_name: "l1_sender_required_confirmations",
            value: str_val(L1_SENDER_REQUIRED_CONFIRMATIONS),
            description: "L1 block confirmations required by the L1 sender",
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

    // Pubdata sending mode (server-side name for the DA mode).
    params.push(ServerParam {
        env_name: "l1_sender_pubdata_mode",
        value: str_val(input.pubdata_mode.server_pubdata_mode()),
        description: "Pubdata sending mode",
    });

    // Fee tier is chosen by the SETTLEMENT LAYER, not the DA transport. Blobs
    // (EIP-4844) only exist on Ethereum L1, so blobs always implies L1 settlement;
    // otherwise the configured settlement layer decides. This is what lets an L2
    // that posts calldata keep L1 fees instead of the pricier L2-settlement fees.
    let effective_settlement = match input.pubdata_mode {
        PubdataMode::Blobs => SettlementLayer::L1,
        PubdataMode::Calldata | PubdataMode::CustomDa => input.settlement,
    };
    let (base_fee, max_fee, max_priority, native_price) = if effective_settlement.is_l1() {
        (
            SETTLE_L1_BASE_FEE_OVERRIDE,
            SETTLE_L1_MAX_FEE_PER_GAS_GWEI,
            SETTLE_L1_MAX_PRIORITY_FEE_GWEI,
            SETTLE_L1_NATIVE_PRICE_OVERRIDE,
        )
    } else {
        (
            SETTLE_L2_BASE_FEE_OVERRIDE,
            SETTLE_L2_MAX_FEE_PER_GAS_GWEI,
            SETTLE_L2_MAX_PRIORITY_FEE_GWEI,
            SETTLE_L2_NATIVE_PRICE_OVERRIDE,
        )
    };
    params.extend([
        ServerParam {
            env_name: "fee_base_fee_override",
            value: str_val(base_fee),
            description: "Base fee override (by settlement layer)",
        },
        ServerParam {
            env_name: "l1_sender_max_fee_per_gas_gwei",
            value: num_val(max_fee),
            description: "Max fee per gas in gwei (by settlement layer)",
        },
        ServerParam {
            env_name: "l1_sender_max_priority_fee_per_gas_gwei",
            value: num_val(max_priority),
            description: "Max priority fee per gas in gwei (by settlement layer)",
        },
        ServerParam {
            env_name: "fee_native_price_override",
            value: str_val(native_price),
            description: "Native price override (by settlement layer)",
        },
    ]);

    // DA-transport fee bits are chosen by the pubdata mode: blobs pay a blob-gas
    // fee; calldata/custom-DA set a pubdata price instead.
    match input.pubdata_mode {
        PubdataMode::Blobs => params.push(ServerParam {
            env_name: "l1_sender_max_fee_per_blob_gas_gwei",
            value: num_val(MAX_FEE_PER_BLOB_GAS_GWEI),
            description: "Max fee per blob gas in gwei (blobs transport)",
        }),
        PubdataMode::Calldata | PubdataMode::CustomDa => params.push(ServerParam {
            env_name: "fee_pubdata_price_override",
            value: str_val(PUBDATA_PRICE_OVERRIDE),
            description: "Pubdata price override (calldata transport)",
        }),
    }

    // External DA adapter settings (only meaningful in custom DA mode). The
    // provider endpoints/keys are deployment-specific and left unset here.
    if matches!(input.pubdata_mode, PubdataMode::CustomDa) {
        params.extend([
            ServerParam {
                env_name: "external_da_enabled",
                value: str_val("true"),
                description: "Enable external DA integration",
            },
            ServerParam {
                env_name: "external_da_provider",
                value: str_val("avail"),
                description: "External DA provider identifier",
            },
        ]);
    }

    params
}
