//! Container configuration types.

use std::path::PathBuf;

/// Default timeout in seconds (1 hour).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 3600;

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

    /// Directory for log files.
    pub log_dir: PathBuf,

    /// Command name for log filename (e.g., "init", "deploy").
    pub log_command: String,

    /// Label to show in progress messages (e.g., "Initializing...").
    pub log_label: String,

    /// Suppress terminal output (still saves logs to file).
    pub quiet: bool,

    /// User to run container as (format: "uid:gid"). Unix-only.
    pub user: Option<String>,
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
            log_dir: PathBuf::new(),
            log_command: "container".to_string(),
            log_label: "Running...".to_string(),
            quiet: false,
            user: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContainerConfig::default();
        assert_eq!(config.working_dir, "/workspace");
        assert_eq!(config.state_dir, PathBuf::new());
        assert!(config.command.is_empty());
        assert!(config.env_vars.is_empty());
        assert!(config.host_network);
        assert_eq!(config.timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
        assert_eq!(config.log_command, "container");
        assert_eq!(config.log_label, "Running...");
        assert!(!config.quiet);
        assert!(config.user.is_none());
    }
}
