//! Server parameter and input types.

use adi_types::{ChainContracts, ChainMetadata, ProverMode, PubdataMode, SettlementLayer, Wallets};
use alloy_primitives::Address;
use serde_json::Value;

use super::version::ServerVersion;

/// Server parameter with its environment variable name and value.
pub struct ServerParam {
    /// Docker Compose environment variable name.
    pub env_name: &'static str,
    /// Resolved value, or `None` when not available for the current input.
    pub value: Option<Value>,
    /// Human-readable description of the parameter.
    pub description: &'static str,
}

/// Input data for extracting server parameters.
pub struct ServerParamsInput<'a> {
    /// Deployed chain contract addresses.
    pub contracts: &'a ChainContracts,
    /// Chain wallets (operator, prove/execute operators, fee account).
    pub wallets: &'a Wallets,
    /// Chain metadata (chain ID, prover version, etc).
    pub chain_metadata: &'a ChainMetadata,
    /// Settlement layer RPC URL, when configured.
    pub rpc_url: Option<&'a str>,
    /// Data-availability pubdata mode.
    pub pubdata_mode: PubdataMode,
    /// Settlement layer the chain settles on.
    pub settlement: SettlementLayer,
    /// Prover mode (fake vs GPU provers).
    pub prover_mode: ProverMode,
    /// Base64-encoded compact genesis JSON, when available.
    pub genesis_base64: Option<String>,
    /// Fee collector address, when configured.
    pub fee_collector_address: Option<Address>,
    /// Base (CGT) token address, when configured.
    pub base_token_address: Option<Address>,
    /// ZkSync OS server version to generate parameters for.
    pub server_version: ServerVersion,
}

/// Display a Value as a string for UI rendering.
pub fn display_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}
