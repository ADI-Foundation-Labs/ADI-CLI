//! Real-time Docker container output streaming.
//!
// Note: This module is part of the Docker orchestration API (T094-T098).
// It will be used by the toolkit runner and future command implementations.
#![allow(dead_code)]
//!
//! Provides functionality for streaming stdout and stderr from running
//! containers to the terminal in real-time.
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::docker::{DockerClient, ContainerManager, OutputStreamer};
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let client = DockerClient::connect()?;
//!     let streamer = OutputStreamer::new(client.clone());
//!
//!     // Stream output while container is running
//!     streamer.stream_logs(&container_id, |output| {
//!         match output {
//!             OutputLine::Stdout(line) => print!("{}", line),
//!             OutputLine::Stderr(line) => eprint!("{}", line),
//!         }
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};
use bollard::container::{AttachContainerOptions, LogOutput, LogsOptions};
use futures_util::StreamExt;

use super::client::DockerClient;

/// Streams container output in real-time.
#[derive(Debug, Clone)]
pub struct OutputStreamer {
    client: DockerClient,
}

/// A line of output from a container.
#[derive(Debug, Clone)]
pub enum OutputLine {
    /// Standard output.
    Stdout(String),
    /// Standard error.
    Stderr(String),
}

impl OutputLine {
    /// Get the content regardless of stream type.
    pub fn content(&self) -> &str {
        match self {
            OutputLine::Stdout(s) | OutputLine::Stderr(s) => s,
        }
    }

    /// Check if this is stderr output.
    #[allow(dead_code)] // May be used for filtering
    pub fn is_stderr(&self) -> bool {
        matches!(self, OutputLine::Stderr(_))
    }

    /// Check if this is stdout output.
    #[allow(dead_code)] // May be used for filtering
    pub fn is_stdout(&self) -> bool {
        matches!(self, OutputLine::Stdout(_))
    }
}

/// Collected output from a container.
#[derive(Debug, Clone, Default)]
pub struct CollectedOutput {
    /// All stdout lines.
    pub stdout: String,
    /// All stderr lines.
    pub stderr: String,
}

impl CollectedOutput {
    /// Create empty collected output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a line to the appropriate stream.
    fn append(&mut self, line: &OutputLine) {
        match line {
            OutputLine::Stdout(s) => self.stdout.push_str(s),
            OutputLine::Stderr(s) => self.stderr.push_str(s),
        }
    }

    /// Get combined output (stdout + stderr).
    #[allow(dead_code)] // May be used for simple output handling
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

impl OutputStreamer {
    /// Create a new output streamer.
    ///
    /// # Arguments
    ///
    /// * `client` - Docker client to use.
    pub fn new(client: DockerClient) -> Self {
        Self { client }
    }

    /// Stream container logs with a callback.
    ///
    /// This method attaches to a running container's stdout and stderr
    /// and calls the callback for each line of output.
    ///
    /// # Arguments
    ///
    /// * `container_id` - Container ID to stream from.
    /// * `callback` - Function called for each output line.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// streamer.stream_logs(&id, |line| {
    ///     println!("{}", line.content());
    /// }).await?;
    /// ```
    pub async fn stream_logs<F>(&self, container_id: &str, mut callback: F) -> Result<()>
    where
        F: FnMut(OutputLine),
    {
        let options = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stream = self.client.inner().logs(container_id, Some(options));

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    let line = log_output_to_line(output);
                    callback(line);
                }
                Err(e) => {
                    // Check for expected end-of-stream errors
                    let err_str = e.to_string();
                    if err_str.contains("broken pipe")
                        || err_str.contains("connection reset")
                        || err_str.contains("EOF")
                    {
                        // Container finished, this is expected
                        break;
                    }
                    return Err(e)
                        .wrap_err_with(|| format!("Error streaming logs from: {container_id}"));
                }
            }
        }

        Ok(())
    }

    /// Attach to a container and stream output in real-time.
    ///
    /// Unlike `stream_logs`, this method attaches before the container
    /// starts, ensuring no output is missed.
    ///
    /// # Arguments
    ///
    /// * `container_id` - Container ID to attach to.
    /// * `callback` - Function called for each output line.
    pub async fn attach_and_stream<F>(&self, container_id: &str, mut callback: F) -> Result<()>
    where
        F: FnMut(OutputLine),
    {
        let options = AttachContainerOptions::<String> {
            stdout: Some(true),
            stderr: Some(true),
            stream: Some(true),
            logs: Some(true),
            ..Default::default()
        };

        let attach_result = self
            .client
            .inner()
            .attach_container(container_id, Some(options))
            .await
            .wrap_err_with(|| format!("Failed to attach to container: {container_id}"))?;

        let mut output_stream = attach_result.output;

        while let Some(result) = output_stream.next().await {
            match result {
                Ok(output) => {
                    let line = log_output_to_line(output);
                    callback(line);
                }
                Err(e) => {
                    // Check for expected end-of-stream errors
                    let err_str = e.to_string();
                    if err_str.contains("broken pipe")
                        || err_str.contains("connection reset")
                        || err_str.contains("EOF")
                    {
                        break;
                    }
                    return Err(e)
                        .wrap_err_with(|| format!("Error in attach stream: {container_id}"));
                }
            }
        }

        Ok(())
    }

    /// Collect all output from a container into memory.
    ///
    /// This is useful when you need to parse the output after
    /// the container completes.
    ///
    /// # Arguments
    ///
    /// * `container_id` - Container ID to collect from.
    ///
    /// # Returns
    ///
    /// Collected stdout and stderr.
    pub async fn collect_output(&self, container_id: &str) -> Result<CollectedOutput> {
        let mut collected = CollectedOutput::new();

        self.stream_logs(container_id, |line| {
            collected.append(&line);
        })
        .await?;

        Ok(collected)
    }

    /// Get the Docker client reference.
    pub fn client(&self) -> &DockerClient {
        &self.client
    }
}

/// Convert Bollard LogOutput to our OutputLine type.
fn log_output_to_line(output: LogOutput) -> OutputLine {
    match output {
        LogOutput::StdOut { message } => {
            OutputLine::Stdout(String::from_utf8_lossy(&message).to_string())
        }
        LogOutput::StdErr { message } => {
            OutputLine::Stderr(String::from_utf8_lossy(&message).to_string())
        }
        LogOutput::Console { message } => {
            // Console output (when TTY is enabled)
            OutputLine::Stdout(String::from_utf8_lossy(&message).to_string())
        }
        LogOutput::StdIn { message } => {
            // Stdin echo (shouldn't happen in our use case)
            OutputLine::Stdout(String::from_utf8_lossy(&message).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_line_content() {
        let stdout = OutputLine::Stdout("hello".to_string());
        let stderr = OutputLine::Stderr("error".to_string());

        assert_eq!(stdout.content(), "hello");
        assert_eq!(stderr.content(), "error");
        assert!(stdout.is_stdout());
        assert!(!stdout.is_stderr());
        assert!(stderr.is_stderr());
        assert!(!stderr.is_stdout());
    }

    #[test]
    fn test_collected_output() {
        let mut collected = CollectedOutput::new();
        collected.append(&OutputLine::Stdout("line1\n".to_string()));
        collected.append(&OutputLine::Stderr("err1\n".to_string()));
        collected.append(&OutputLine::Stdout("line2\n".to_string()));

        assert_eq!(collected.stdout, "line1\nline2\n");
        assert_eq!(collected.stderr, "err1\n");
        assert_eq!(collected.combined(), "line1\nline2\nerr1\n");
    }
}
