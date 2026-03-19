//! SDK for upgrading ZkSync ecosystem contracts.
//!
//! This crate provides the upgrade orchestration logic for ZkSync
//! ecosystem and chain contracts.

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod error;

pub use error::{Result, UpgradeError};

pub mod versions;

pub use versions::{get_handler, is_supported, PostUpgradeHook, VersionHandler};
