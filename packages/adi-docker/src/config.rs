//! Container configuration types.

use std::path::PathBuf;

/// Default timeout in seconds (30 minutes).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 1800;

/// Configuration for creating a container.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Working directory inside the container.
    pub working_dir: String,

    /// Host directory to mount as /workspace.
    pub state_dir: PathBuf,

    /// Command to execute.
    pub command: Vec<String>,

    /// Environment variables as (key, value) pairs.
    pub env_vars: Vec<(String, String)>,

    /// Use host network mode.
    pub host_network: bool,

    /// Timeout in seconds.
    pub timeout_seconds: u64,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            working_dir: "/workspace".to_string(),
            state_dir: PathBuf::new(),
            command: Vec::new(),
            env_vars: Vec::new(),
            host_network: true,
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        }
    }
}
