//! Versioned ZkSync OS server parameter generation.
//!
//! Given a [`ServerVersion`] and the chain's on-chain/wallet/metadata state,
//! [`extract`] produces the Docker Compose environment variables the
//! ZkSync OS server needs to run. Version-specific parameter differences are
//! selected inside `extract` — each [`ServerVersion`] variant has its own
//! module under `versions/` with its own extraction function.

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod params;
mod version;
mod versions;

pub use params::{display_value, ServerParam, ServerParamsInput};
pub use version::ServerVersion;
pub use versions::extract;
