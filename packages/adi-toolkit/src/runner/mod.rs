//! Toolkit command execution via Docker containers.

mod commands;
mod params;

pub(crate) use params::RunCommandParams;
pub use params::{EcosystemInitParams, ForgeVerifyParams};

use crate::cleanup::cleanup_tmp_dir;
use crate::config::ToolkitConfig;
use crate::error::Result;
use adi_docker::{ContainerConfig, ContainerManager, DockerClient};
use adi_types::{LogCrateLogger, Logger};
use console::style;
use semver::Version;
use std::path::Path;
use std::sync::Arc;

/// Get current user UID:GID for container user mapping (Unix only).
#[cfg(unix)]
fn get_current_user() -> Option<String> {
    use std::process::Command;

    let uid = Command::new("id").arg("-u").output().ok()?;
    let gid = Command::new("id").arg("-g").output().ok()?;

    let uid = String::from_utf8_lossy(&uid.stdout).trim().to_string();
    let gid = String::from_utf8_lossy(&gid.stdout).trim().to_string();

    if uid.is_empty() || gid.is_empty() {
        return None;
    }

    Some(format!("{}:{}", uid, gid))
}

/// Get current user UID:GID for container user mapping (non-Unix stub).
#[cfg(not(unix))]
fn get_current_user() -> Option<String> {
    None
}

/// Escape a string for safe use in a shell command.
///
/// Wraps the value in single quotes and escapes any embedded single quotes.
/// This prevents shell metacharacters from being interpreted.
pub(crate) fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Executes commands inside Docker toolkit containers.
///
/// Container lifecycle: create -> start -> stream output -> wait -> remove
pub struct ToolkitRunner {
    client: DockerClient,
    config: ToolkitConfig,
    logger: Arc<dyn Logger>,
}

impl ToolkitRunner {
    /// Create a new ToolkitRunner by connecting to Docker.
    pub async fn new() -> Result<Self> {
        Self::with_logger(Arc::new(LogCrateLogger)).await
    }

    /// Create a new ToolkitRunner with custom logger.
    pub async fn with_logger(logger: Arc<dyn Logger>) -> Result<Self> {
        let client = DockerClient::with_logger(Arc::clone(&logger)).await?;
        Ok(Self {
            client,
            config: ToolkitConfig::default(),
            logger,
        })
    }

    /// Create a new ToolkitRunner with custom configuration.
    pub async fn with_config(config: ToolkitConfig) -> Result<Self> {
        Self::with_config_and_logger(config, Arc::new(LogCrateLogger)).await
    }

    /// Create a new ToolkitRunner with custom configuration and logger.
    pub async fn with_config_and_logger(
        config: ToolkitConfig,
        logger: Arc<dyn Logger>,
    ) -> Result<Self> {
        let client = DockerClient::with_logger(Arc::clone(&logger)).await?;
        Ok(Self {
            client,
            config,
            logger,
        })
    }

    /// Execute a generic command in the toolkit container.
    /// Logs are saved to state_dir/logs/.
    pub async fn run_command(
        &self,
        command: &[&str],
        state_dir: &Path,
        protocol_version: &Version,
        env_vars: &[(&str, &str)],
        log_command: &str,
        log_label: &str,
    ) -> Result<i64> {
        self.run_command_internal(RunCommandParams {
            command,
            state_dir,
            log_dir: state_dir,
            protocol_version,
            env_vars,
            log_command,
            log_label,
            quiet: false,
        })
        .await
    }

    /// Internal command runner with quiet mode support.
    pub(crate) async fn run_command_internal(&self, params: RunCommandParams<'_>) -> Result<i64> {
        let image_ref = self
            .config
            .image_reference(params.protocol_version, self.logger.as_ref());
        let image_uri = image_ref.full_uri();

        if !params.quiet {
            self.logger.info(&format!(
                "Using toolkit image: {}",
                style(&image_uri).green()
            ));
        }
        self.logger.debug(&format!(
            "Running command: {:?} (state_dir: {}, log_dir: {})",
            params.command,
            params.state_dir.display(),
            params.log_dir.display()
        ));

        self.client.pull_image(&image_uri).await?;

        // Always include CI=true to suppress interactive prompts (e.g., telemetry)
        let mut all_env_vars: Vec<(String, String)> = vec![("CI".to_string(), "true".to_string())];
        all_env_vars.extend(
            params
                .env_vars
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
        );

        let container_config = ContainerConfig {
            state_dir: params.state_dir.to_path_buf(),
            command: params.command.iter().map(|s| (*s).to_string()).collect(),
            env_vars: all_env_vars,
            timeout_seconds: self.config.timeout_seconds,
            log_dir: params.log_dir.to_path_buf(),
            log_command: params.log_command.to_string(),
            log_label: params.log_label.to_string(),
            quiet: params.quiet,
            user: get_current_user(),
            ..Default::default()
        };

        self.logger.debug(&format!(
            "Container config: working_dir={}, timeout={}s",
            container_config.working_dir, container_config.timeout_seconds
        ));

        let manager =
            ContainerManager::with_logger(self.client.inner().clone(), Arc::clone(&self.logger));
        let result = manager.run(&image_uri, &container_config).await;

        // Always clean up tmp directory (keep only *.md files), even on error/interrupt
        let tmp_dir = params.state_dir.join(".tmp");
        if tmp_dir.exists() {
            let failed = matches!(&result, Ok(code) if *code != 0);
            let logger = Arc::clone(&self.logger);
            tokio::task::spawn_blocking(move || {
                if failed {
                    log_crash_reports(&tmp_dir, logger.as_ref());
                }
                cleanup_tmp_dir(&tmp_dir, logger.as_ref());
            })
            .await
            .ok();
        }

        let exit_code = result?;
        self.logger
            .debug(&format!("Command completed with exit code: {}", exit_code));

        Ok(exit_code)
    }
}

/// Log any crash reports found in the given directory.
fn log_crash_reports(dir: &Path, logger: &dyn Logger) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();
        if filename_str.starts_with("report-") && filename_str.ends_with(".toml") {
            logger.error(&format!(
                "Crash report available at: {}",
                entry.path().display()
            ));
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn test_shell_escape_metacharacters() {
        assert_eq!(shell_escape("a; rm -rf /"), "'a; rm -rf /'");
        assert_eq!(shell_escape("$(whoami)"), "'$(whoami)'");
        assert_eq!(shell_escape("foo`bar`"), "'foo`bar`'");
    }

    #[test]
    fn test_shell_escape_single_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_shell_escape_empty() {
        assert_eq!(shell_escape(""), "''");
    }

    #[cfg(unix)]
    #[test]
    fn test_get_current_user_format() {
        let user = get_current_user();
        let user = user.unwrap();
        assert!(user.contains(':'), "Expected UID:GID format, got: {}", user);
        let parts: Vec<&str> = user.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].parse::<u32>().is_ok(), "UID should be numeric");
        assert!(parts[1].parse::<u32>().is_ok(), "GID should be numeric");
    }
}
