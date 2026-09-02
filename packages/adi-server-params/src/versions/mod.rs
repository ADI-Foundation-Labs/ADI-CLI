//! Version-specific server parameter extraction.
//!
//! Each [`ServerVersion`] variant gets its own module with an `extract`
//! function producing that version's Docker Compose environment variables.

mod constants;
mod v0_21_1;

use alloy_primitives::Address;
use serde_json::Value;

use self::constants::{BASE_TOKEN_FORCED_PRICE, ETH_FORCED_PRICE, ETH_FORCED_PRICE_ADDRESS};
use crate::params::{ServerParam, ServerParamsInput};
use crate::version::ServerVersion;

/// Extract server parameters from the given input.
pub fn extract(input: &ServerParamsInput<'_>) -> Vec<ServerParam> {
    // Version-specific parameter differences are implemented in each
    // version's own module.
    match input.server_version {
        ServerVersion::V0211 => v0_21_1::extract(input),
    }
}

/// Helper to create a string Value.
fn str_val(s: &str) -> Option<Value> {
    Some(Value::String(s.to_string()))
}

/// Helper to create a u64 Value.
fn num_val(n: u64) -> Option<Value> {
    Some(serde_json::json!(n))
}

/// Build the JSON-stringified forced-prices map for the external price API client.
///
/// Always includes the ETH placeholder address. When a base (CGT) token address is
/// provided, it is also included using a lowercase hex encoding.
fn build_forced_prices_json(base_token_address: Option<Address>) -> String {
    let mut prices = serde_json::Map::new();
    prices.insert(
        ETH_FORCED_PRICE_ADDRESS.to_string(),
        serde_json::json!(ETH_FORCED_PRICE),
    );
    if let Some(addr) = base_token_address {
        prices.insert(
            format!("{addr:#x}"),
            serde_json::json!(BASE_TOKEN_FORCED_PRICE),
        );
    }
    serde_json::to_string(&Value::Object(prices)).unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::HashMap;

    use adi_types::{
        ChainContracts, ChainMetadata, ProverMode, PubdataMode, SettlementLayer, Wallets,
    };
    use alloy_primitives::Address;
    use serde_json::Value;

    use super::{extract, num_val, str_val};
    use crate::params::{ServerParam, ServerParamsInput};
    use crate::version::ServerVersion;
    use crate::versions::constants::{
        BASE_TOKEN_FORCED_PRICE, BASE_TOKEN_PRICE_UPDATER_ENABLED, BATCH_TIMEOUT, BLOCK_TIME,
        ETH_FORCED_PRICE, ETH_FORCED_PRICE_ADDRESS, EXTERNAL_PRICE_API_CLIENT_SOURCE,
        MAX_FEE_PER_BLOB_GAS_GWEI, OBSERVABILITY_LOG_FORMAT, OBSERVABILITY_LOG_USE_COLOR,
        POLL_INTERVAL, PUBDATA_PRICE_OVERRIDE, ROCKS_DB_PATH, RUST_LOG_VALUE,
        SETTLE_L1_BASE_FEE_OVERRIDE, SETTLE_L1_MAX_FEE_PER_GAS_GWEI,
        SETTLE_L1_MAX_PRIORITY_FEE_GWEI, SETTLE_L1_NATIVE_PRICE_OVERRIDE,
        SETTLE_L2_BASE_FEE_OVERRIDE, SETTLE_L2_MAX_FEE_PER_GAS_GWEI,
        SETTLE_L2_NATIVE_PRICE_OVERRIDE,
    };

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

    fn make_input(
        pubdata_mode: PubdataMode,
        prover_mode: ProverMode,
    ) -> ServerParamsInput<'static> {
        make_input_with_base_token(pubdata_mode, prover_mode, None)
    }

    fn make_input_with_base_token(
        pubdata_mode: PubdataMode,
        prover_mode: ProverMode,
        base_token_address: Option<Address>,
    ) -> ServerParamsInput<'static> {
        make_input_full(
            pubdata_mode,
            SettlementLayer::L2,
            prover_mode,
            base_token_address,
        )
    }

    fn make_input_with_settlement(
        pubdata_mode: PubdataMode,
        settlement: SettlementLayer,
    ) -> ServerParamsInput<'static> {
        make_input_full(pubdata_mode, settlement, ProverMode::NoProofs, None)
    }

    fn make_input_full(
        pubdata_mode: PubdataMode,
        settlement: SettlementLayer,
        prover_mode: ProverMode,
        base_token_address: Option<Address>,
    ) -> ServerParamsInput<'static> {
        let contracts: &'static ChainContracts = Box::leak(Box::new(ChainContracts::default()));
        let wallets: &'static Wallets = Box::leak(Box::new(Wallets::default()));
        let metadata: &'static ChainMetadata = Box::leak(Box::new(default_metadata()));

        ServerParamsInput {
            contracts,
            wallets,
            chain_metadata: metadata,
            rpc_url: Some("http://localhost:8545"),
            pubdata_mode,
            settlement,
            prover_mode,
            genesis_base64: Some("dGVzdA==".to_string()),
            fee_collector_address: Some(Address::ZERO),
            base_token_address,
            server_version: ServerVersion::V0211,
        }
    }

    fn to_map(params: &[ServerParam]) -> HashMap<&str, Option<Value>> {
        params
            .iter()
            .map(|p| (p.env_name, p.value.clone()))
            .collect()
    }

    #[test]
    fn l2_mode_sets_pubdata_mode_to_blobs() {
        let input = make_input(PubdataMode::Blobs, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(map["l1_sender_pubdata_mode"], str_val("Blobs"));
        assert_eq!(
            map["l1_sender_max_fee_per_blob_gas_gwei"],
            num_val(MAX_FEE_PER_BLOB_GAS_GWEI)
        );
        assert_eq!(
            map["fee_base_fee_override"],
            str_val(SETTLE_L1_BASE_FEE_OVERRIDE)
        );
        assert_eq!(
            map["fee_native_price_override"],
            str_val(SETTLE_L1_NATIVE_PRICE_OVERRIDE)
        );
        assert!(!map.contains_key("fee_pubdata_price_override"));
    }

    #[test]
    fn l3_mode_includes_pubdata_mode_excludes_blob_gas() {
        let input = make_input(PubdataMode::Calldata, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(map["l1_sender_pubdata_mode"], str_val("Calldata"));
        assert_eq!(
            map["fee_pubdata_price_override"],
            str_val(PUBDATA_PRICE_OVERRIDE)
        );
        assert_eq!(
            map["fee_base_fee_override"],
            str_val(SETTLE_L2_BASE_FEE_OVERRIDE)
        );
        assert_eq!(
            map["fee_native_price_override"],
            str_val(SETTLE_L2_NATIVE_PRICE_OVERRIDE)
        );
        assert!(!map.contains_key("l1_sender_max_fee_per_blob_gas_gwei"));
    }

    #[test]
    fn blobs_always_uses_l1_settlement_fees() {
        // Blobs imply Ethereum L1 settlement regardless of the settlement field.
        for settlement in [SettlementLayer::L1, SettlementLayer::L2] {
            let input = make_input_with_settlement(PubdataMode::Blobs, settlement);
            let params = extract(&input);
            let map = to_map(&params);
            assert_eq!(
                map["l1_sender_max_fee_per_gas_gwei"],
                num_val(SETTLE_L1_MAX_FEE_PER_GAS_GWEI)
            );
            assert_eq!(
                map["l1_sender_max_priority_fee_per_gas_gwei"],
                num_val(SETTLE_L1_MAX_PRIORITY_FEE_GWEI)
            );
        }
    }

    #[test]
    fn calldata_on_l2_settlement_uses_l2_fees() {
        // Default case (settles on an L2 -> the chain is an L3): high fees.
        let input = make_input_with_settlement(PubdataMode::Calldata, SettlementLayer::L2);
        let params = extract(&input);
        let map = to_map(&params);
        assert_eq!(
            map["l1_sender_max_fee_per_gas_gwei"],
            num_val(SETTLE_L2_MAX_FEE_PER_GAS_GWEI)
        );
        assert_eq!(
            map["fee_base_fee_override"],
            str_val(SETTLE_L2_BASE_FEE_OVERRIDE)
        );
    }

    #[test]
    fn calldata_on_l1_settlement_uses_l1_fees() {
        // The new capability: an L2 that posts calldata keeps L1 fees, not L3 fees.
        let input = make_input_with_settlement(PubdataMode::Calldata, SettlementLayer::L1);
        let params = extract(&input);
        let map = to_map(&params);
        assert_eq!(
            map["l1_sender_max_fee_per_gas_gwei"],
            num_val(SETTLE_L1_MAX_FEE_PER_GAS_GWEI)
        );
        assert_eq!(
            map["fee_base_fee_override"],
            str_val(SETTLE_L1_BASE_FEE_OVERRIDE)
        );
        // Transport bit is still calldata (pubdata price, no blob-gas fee).
        assert_eq!(
            map["fee_pubdata_price_override"],
            str_val(PUBDATA_PRICE_OVERRIDE)
        );
        assert!(!map.contains_key("l1_sender_max_fee_per_blob_gas_gwei"));
    }

    #[test]
    fn prover_mode_noproofs_enables_fake_provers() {
        let input = make_input(PubdataMode::Blobs, ProverMode::NoProofs);
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
        let input = make_input(PubdataMode::Blobs, ProverMode::Gpu);
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
        let input = make_input(PubdataMode::Blobs, ProverMode::NoProofs);
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
        let input = make_input(PubdataMode::Blobs, ProverMode::NoProofs);
        let params = extract(&input);
        let map = to_map(&params);

        assert_eq!(map["RUST_LOG"], str_val(RUST_LOG_VALUE));
        assert_eq!(map["general_rocks_db_path"], str_val(ROCKS_DB_PATH));
        assert_eq!(map["batcher_batch_timeout"], str_val(BATCH_TIMEOUT));
        assert_eq!(map["sequencer_block_time"], str_val(BLOCK_TIME));
        assert_eq!(map["l1_sender_poll_interval"], str_val(POLL_INTERVAL));
        assert_eq!(
            map["external_price_api_client_source"],
            str_val(EXTERNAL_PRICE_API_CLIENT_SOURCE)
        );
        assert_eq!(
            map["base_token_price_updater_enabled"],
            str_val(BASE_TOKEN_PRICE_UPDATER_ENABLED)
        );
        assert_eq!(
            map["observability_log_format"],
            str_val(OBSERVABILITY_LOG_FORMAT)
        );
        assert_eq!(
            map["observability_log_use_color"],
            str_val(OBSERVABILITY_LOG_USE_COLOR)
        );
    }

    #[test]
    fn forced_prices_json_has_eth_only_when_no_base_token() {
        let input = make_input_with_base_token(PubdataMode::Blobs, ProverMode::NoProofs, None);
        let params = extract(&input);
        let map = to_map(&params);

        let raw = map["external_price_api_client_forced_prices__json"]
            .as_ref()
            .unwrap();
        let json_str = raw.as_str().unwrap();
        let parsed: serde_json::Map<String, Value> = serde_json::from_str(json_str).unwrap();

        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[ETH_FORCED_PRICE_ADDRESS].as_f64().unwrap(),
            ETH_FORCED_PRICE
        );
    }

    #[test]
    fn forced_prices_json_includes_base_token_when_set() {
        let cgt: Address = "0x2a98B46fe31BA8Be05ef1cE3D36e1f80Db04190D"
            .parse()
            .unwrap();
        let input = make_input_with_base_token(PubdataMode::Blobs, ProverMode::NoProofs, Some(cgt));
        let params = extract(&input);
        let map = to_map(&params);

        let raw = map["external_price_api_client_forced_prices__json"]
            .as_ref()
            .unwrap();
        let json_str = raw.as_str().unwrap();
        let parsed: serde_json::Map<String, Value> = serde_json::from_str(json_str).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[ETH_FORCED_PRICE_ADDRESS].as_f64().unwrap(),
            ETH_FORCED_PRICE
        );
        let cgt_key = "0x2a98b46fe31ba8be05ef1ce3d36e1f80db04190d";
        assert_eq!(parsed[cgt_key].as_f64().unwrap(), BASE_TOKEN_FORCED_PRICE);
    }
}
