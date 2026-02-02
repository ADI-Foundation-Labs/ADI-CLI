//! Real-time output streaming from containers.

use crate::error::{DockerError, Result};
use bollard::container::LogsOptions;
use bollard::Docker;
use crossterm::cursor::{RestorePosition, SavePosition};
use crossterm::terminal::{Clear, ClearType};
use crossterm::ExecutableCommand;
use futures_util::StreamExt;
use std::collections::VecDeque;
use std::io::{self, Write};

/// Default number of lines in sliding window.
const DEFAULT_WINDOW_SIZE: usize = 10;

/// Sliding window buffer for terminal output.
///
/// Maintains a fixed-size buffer of the most recent lines.
struct SlidingWindow {
    lines: VecDeque<String>,
    max_lines: usize,
    current_line: String,
}

impl SlidingWindow {
    /// Create a new sliding window with specified capacity.
    fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            current_line: String::new(),
        }
    }

    /// Add text chunk, handling partial lines.
    fn push(&mut self, text: &str) {
        self.current_line.push_str(text);

        // Split on newlines, keeping incomplete line in buffer
        while let Some(pos) = self.current_line.find('\n') {
            let line = self.current_line[..pos].to_string();
            self.current_line = self.current_line[pos + 1..].to_string();

            if self.lines.len() >= self.max_lines {
                self.lines.pop_front();
            }
            self.lines.push_back(line);
        }
    }

    /// Get current lines for rendering.
    fn lines(&self) -> &VecDeque<String> {
        &self.lines
    }
}

/// Streams container output to terminal in real-time.
pub(crate) struct OutputStreamer {
    docker: Docker,
}

impl OutputStreamer {
    /// Create a new OutputStreamer.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Stream container logs to stdout/stderr in real-time.
    ///
    /// In debug mode, shows full output. Otherwise uses a sliding
    /// window showing only the last N lines.
    pub async fn stream_logs(&self, container_id: &str) -> Result<()> {
        let options = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));
        let mut stdout = io::stdout();

        let debug_mode = log::max_level() >= log::LevelFilter::Debug;

        if debug_mode {
            self.stream_full(&mut stream, &mut stdout).await
        } else {
            self.stream_windowed(&mut stream, &mut stdout).await
        }
    }

    /// Stream full output (debug mode).
    async fn stream_full<S>(&self, stream: &mut S, stdout: &mut io::Stdout) -> Result<()>
    where
        S: StreamExt<
                Item = std::result::Result<bollard::container::LogOutput, bollard::errors::Error>,
            > + Unpin,
    {
        println!();

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    let bytes = output.into_bytes();
                    stdout
                        .write_all(&bytes)
                        .map_err(|e| DockerError::StreamError(e.to_string()))?;
                    stdout
                        .flush()
                        .map_err(|e| DockerError::StreamError(e.to_string()))?;
                }
                Err(e) => {
                    log::debug!("Log stream ended: {}", e);
                    break;
                }
            }
        }

        println!();
        Ok(())
    }

    /// Stream with sliding window (normal mode).
    async fn stream_windowed<S>(&self, stream: &mut S, stdout: &mut io::Stdout) -> Result<()>
    where
        S: StreamExt<
                Item = std::result::Result<bollard::container::LogOutput, bollard::errors::Error>,
            > + Unpin,
    {
        let mut window = SlidingWindow::new(DEFAULT_WINDOW_SIZE);

        // Save cursor position
        stdout
            .execute(SavePosition)
            .map_err(|e| DockerError::StreamError(e.to_string()))?;

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    let bytes = output.into_bytes();
                    let text = String::from_utf8_lossy(&bytes);
                    window.push(&text);

                    // Restore cursor position and clear from cursor to end of screen
                    stdout
                        .execute(RestorePosition)
                        .map_err(|e| DockerError::StreamError(e.to_string()))?;
                    stdout
                        .execute(Clear(ClearType::FromCursorDown))
                        .map_err(|e| DockerError::StreamError(e.to_string()))?;

                    // Print lines
                    for line in window.lines() {
                        println!("{}", line);
                    }

                    stdout
                        .flush()
                        .map_err(|e| DockerError::StreamError(e.to_string()))?;
                }
                Err(e) => {
                    log::debug!("Log stream ended: {}", e);
                    break;
                }
            }
        }

        println!();
        Ok(())
    }
}
