//! Real-time output streaming from containers.

use crate::error::{DockerError, Result};
use adi_types::Logger;
use bollard::container::LogsOptions;
use bollard::Docker;
use console::Style;
use crossterm::{cursor, terminal, ExecutableCommand};
use futures_util::StreamExt;
use std::collections::VecDeque;
use std::io::{stderr, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Maximum number of log lines to display in the terminal.
const MAX_DISPLAY_LINES: usize = 10;

/// Maximum width for truncating long lines (excluding the bar prefix).
const LINE_MAX_WIDTH: usize = 76;

/// Cliclack-style bar prefix for log lines.
const BAR_PREFIX: &str = "│  ";

/// Strip ANSI escape sequences from a string.
/// This prevents cursor manipulation codes in container output from breaking our display.
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC and the following sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until we hit a letter (end of sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Manages rolling buffer of log lines and terminal rendering.
struct LogDisplay {
    lines: VecDeque<String>,
    rendered_count: usize,
}

impl LogDisplay {
    fn new() -> Self {
        Self {
            lines: VecDeque::new(),
            rendered_count: 0,
        }
    }

    fn push_line(&mut self, line: &str) {
        // Strip ANSI codes to prevent cursor manipulation conflicts
        let clean = strip_ansi(line);
        let char_count = clean.chars().count();
        let truncated = if char_count > LINE_MAX_WIDTH {
            let prefix: String = clean.chars().take(LINE_MAX_WIDTH - 3).collect();
            format!("{}...", prefix)
        } else {
            clean
        };
        self.lines.push_back(truncated);
        if self.lines.len() > MAX_DISPLAY_LINES {
            self.lines.pop_front();
        }
    }

    fn render(&mut self) -> std::io::Result<()> {
        let mut stderr = stderr();

        // Move cursor up to overwrite previous output
        if self.rendered_count > 0 {
            let count = u16::try_from(self.rendered_count).unwrap_or(u16::MAX);
            stderr.execute(cursor::MoveUp(count))?;
        }

        // Clear from cursor down and print fresh
        stderr.execute(terminal::Clear(terminal::ClearType::FromCursorDown))?;

        // Print current lines with cliclack-style bar prefix (blue dim)
        let bar_style = Style::new().blue().dim();
        for line in &self.lines {
            writeln!(
                stderr,
                "{}",
                bar_style.apply_to(format!("{}{}", BAR_PREFIX, line))
            )?;
        }

        self.rendered_count = self.lines.len();
        stderr.flush()?;
        Ok(())
    }

    fn clear(&mut self) -> std::io::Result<()> {
        let mut stderr = stderr();
        if self.rendered_count > 0 {
            let count = u16::try_from(self.rendered_count).unwrap_or(u16::MAX);
            stderr.execute(cursor::MoveUp(count))?;
            stderr.execute(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        }
        self.rendered_count = 0;
        Ok(())
    }
}

/// Streams container output with progress spinner.
pub(crate) struct OutputStreamer {
    docker: Docker,
    logger: Arc<dyn Logger>,
}

impl OutputStreamer {
    /// Create a new OutputStreamer.
    pub fn new(docker: Docker, logger: Arc<dyn Logger>) -> Self {
        Self { docker, logger }
    }

    /// Stream container logs with real-time display.
    ///
    /// Shows the last 10 lines of output, updating in real-time.
    /// Full output is saved to a log file.
    pub async fn stream_logs(
        &self,
        container_id: &str,
        log_dir: &Path,
        command: &str,
        label: &str,
    ) -> Result<()> {
        let options = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));

        let mut buffer: Vec<u8> = Vec::new();
        let start = Instant::now();

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let log_path = log_dir
            .join("logs")
            .join(format!("{}_{}.log", command, timestamp));

        cliclack::log::step(label)
            .map_err(|e| DockerError::StreamError(format!("Failed to log step: {}", e)))?;

        let mut display = LogDisplay::new();

        let stream_result: Result<()> = loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    display.clear().ok();
                    Self::save_log(&buffer, &log_path)?;
                    break Err(DockerError::StreamError("Interrupted by CTRL+C".to_string()));
                }

                result = stream.next() => {
                    match result {
                        Some(Ok(output)) => {
                            let bytes = output.into_bytes();
                            if let Ok(text) = std::str::from_utf8(&bytes) {
                                for line in text.lines() {
                                    if !line.is_empty() {
                                        display.push_line(line);
                                    }
                                }
                                display.render().ok();
                            }
                            buffer.extend(bytes);
                        }
                        Some(Err(e)) => {
                            self.logger.debug(&format!("Log stream ended: {}", e));
                            break Ok(());
                        }
                        None => {
                            break Ok(());
                        }
                    }
                }
            }
        };

        // Normal completion - clear display and show completion message
        if stream_result.is_ok() {
            display.clear().ok();
            cliclack::log::success(format!("Completed in {}s", start.elapsed().as_secs()))
                .map_err(|e| DockerError::StreamError(format!("Failed to log success: {}", e)))?;
            Self::save_log(&buffer, &log_path)?;
        }

        stream_result
    }

    fn save_log(buffer: &[u8], log_path: &Path) -> Result<()> {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DockerError::StreamError(format!("Failed to create log dir: {}", e))
            })?;
        }
        std::fs::write(log_path, buffer)
            .map_err(|e| DockerError::StreamError(format!("Failed to write log: {}", e)))?;
        let path_styled = Style::new().green().apply_to(log_path.display());
        cliclack::log::info(format!("Full output saved to: {}", path_styled))
            .map_err(|e| DockerError::StreamError(format!("Failed to log: {}", e)))?;
        Ok(())
    }
}
