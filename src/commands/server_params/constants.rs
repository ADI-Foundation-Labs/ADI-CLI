//! Static constants for server parameter values.

/// Default RUST_LOG value for the ZkSync OS server.
pub const RUST_LOG_VALUE: &str = "info,zksync_os_server=info,zksync_os_sequencer=info,zksync_os_merkle_tree=info,zksync_os_priority_tree=info";

/// RocksDB path inside the container.
pub const ROCKS_DB_PATH: &str = "/chain/db/node1";

/// Genesis input file path inside the container.
pub const GENESIS_INPUT_PATH: &str = "/chain/genesis/genesis.json";

/// Fusaka upgrade timestamp (hardcoded).
pub const FUSAKA_UPGRADE_TIMESTAMP: u64 = 1771883505;

/// Prover API listen address.
pub const PROVER_API_ADDR: &str = "0.0.0.0:3320";

/// Prover object store base path.
pub const OBJECT_STORE_BASE_PATH: &str = "/chain/db/shared";

/// Application binary unpack path.
pub const APP_BIN_UNPACK_PATH: &str = "/chain/db/node1/app_bins";

/// Maximum in-flight blocks for prover input generator.
pub const MAX_IN_FLIGHT_BLOCKS: &str = "30";

/// Block dump path for the sequencer.
pub const BLOCK_DUMP_PATH: &str = "/chain/db/node1/block_dumps";

/// Block time interval.
pub const BLOCK_TIME: &str = "1s";

/// Maximum transactions per block.
pub const MAX_TXS_IN_BLOCK: u64 = 3000;

/// Batcher batch timeout.
pub const BATCH_TIMEOUT: &str = "3600s";

/// Batcher blocks per batch limit.
pub const BLOCKS_PER_BATCH_LIMIT: u64 = 1400;

/// Poll interval for L1 sender and watcher.
pub const POLL_INTERVAL: &str = "500ms";

// ========== L2 values (blobs=true) ==========

/// L2 sequencer base fee override.
pub const L2_BASE_FEE_OVERRIDE: &str = "0x8085C39000";

/// L2 max fee per gas in gwei.
pub const L2_MAX_FEE_PER_GAS_GWEI: u64 = 25;

/// L2 max priority fee per gas in gwei.
pub const L2_MAX_PRIORITY_FEE_GWEI: u64 = 25;

/// L2 sequencer native price override.
pub const L2_NATIVE_PRICE_OVERRIDE: &str = "0x694920";

/// L2 max fee per blob gas in gwei.
pub const L2_MAX_FEE_PER_BLOB_GAS_GWEI: u64 = 25;

// ========== L3 values (blobs=false) ==========

/// L3 sequencer base fee override.
pub const L3_BASE_FEE_OVERRIDE: &str = "0x3e8";

/// L3 max fee per gas in gwei.
pub const L3_MAX_FEE_PER_GAS_GWEI: u64 = 1500;

/// L3 max priority fee per gas in gwei.
pub const L3_MAX_PRIORITY_FEE_GWEI: u64 = 1500;

/// L3 sequencer native price override.
pub const L3_NATIVE_PRICE_OVERRIDE: &str = "0x1";

/// L3 sequencer pubdata price override.
pub const L3_PUBDATA_PRICE_OVERRIDE: &str = "0x1";

// ========== External price API (forced prices) ==========

/// External price API client source mode.
pub const EXTERNAL_PRICE_API_CLIENT_SOURCE: &str = "Forced";

/// Base token price updater enabled flag.
pub const BASE_TOKEN_PRICE_UPDATER_ENABLED: &str = "true";

/// Observability log format.
pub const OBSERVABILITY_LOG_FORMAT: &str = "terminal";

/// Observability log use-color flag.
pub const OBSERVABILITY_LOG_USE_COLOR: &str = "true";

/// ETH placeholder address used in the forced-prices map.
pub const ETH_FORCED_PRICE_ADDRESS: &str = "0x0000000000000000000000000000000000000001";

/// Forced price for ETH in the external price API client map.
pub const ETH_FORCED_PRICE: f64 = 3000.0;

/// Forced price for the chain's base (CGT) token in the external price API client map.
pub const BASE_TOKEN_FORCED_PRICE: f64 = 1.0;
