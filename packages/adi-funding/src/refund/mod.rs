//! Refund SDK — drains ETH (and optional ERC20 tokens) from wallets back to a receiver.
//!
//! This is the inverse of funding: each wallet signs its own transfer to a single
//! receiver address. Uses continue-on-error semantics so partial refunds succeed.

mod execute;
mod plan;
mod types;

pub use execute::execute_refund;
pub use plan::build_refund_plan;
pub use types::{RefundConfig, RefundEntry, RefundPlan, RefundResult, RefundTarget};
