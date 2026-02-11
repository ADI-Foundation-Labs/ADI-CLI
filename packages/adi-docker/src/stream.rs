//! Real-time output streaming from containers.

use crate::error::{DockerError, Result};
use bollard::container::LogsOptions;
use bollard::Docker;
use colored::Colorize;
use crossterm::cursor::MoveUp;
use crossterm::terminal::{Clear, ClearType};
use crossterm::QueueableCommand;
use futures_util::StreamExt;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::{Duration, Instant};

/// Number of lines to show in progress display.
const DISPLAY_LINES: usize = 10;

/// Interval between progress updates.
const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(2);

/// Strip all ANSI escape sequences from text.
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() || ch == ';' || ch == '?' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                chars.next();
            } else {
                chars.next();
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Streams container output with progress display.
pub(crate) struct OutputStreamer {
    docker: Docker,
}

impl OutputStreamer {
    /// Create a new OutputStreamer.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Stream container logs with progress display.
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
        let mut stdout = io::stdout();

        let mut buffer: Vec<u8> = Vec::new();
        let start = Instant::now();
        let is_tty = stdout.is_terminal();
        let mut lines_printed: u16 = 0;
        let mut last_update = Instant::now();
        let mut has_output = false;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let log_path = log_dir
            .join("logs")
            .join(format!("{}_{}.log", command, timestamp));

        // Print header
        println!("{}", label.blue());

        let stream_result: Result<()> = loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log::warn!("Interrupted by CTRL+C, saving log...");
                    if is_tty && lines_printed > 0 {
                        let _ = stdout.queue(MoveUp(lines_printed));
                        let _ = stdout.queue(Clear(ClearType::FromCursorDown));
                        let _ = stdout.flush();
                    }
                    Self::save_log(&buffer, &log_path)?;
                    break Err(DockerError::StreamError("Interrupted by CTRL+C".to_string()));
                }

                result = stream.next() => {
                    match result {
                        Some(Ok(output)) => {
                            buffer.extend(output.into_bytes());

                            // Show output immediately on first data, then every SNAPSHOT_INTERVAL
                            let should_update = !has_output || last_update.elapsed() >= SNAPSHOT_INTERVAL;

                            if should_update {
                                has_output = true;
                                last_update = Instant::now();

                                if is_tty {
                                    if lines_printed > 0 {
                                        stdout
                                            .queue(MoveUp(lines_printed))
                                            .map_err(|e| DockerError::StreamError(e.to_string()))?;
                                        stdout
                                            .queue(Clear(ClearType::FromCursorDown))
                                            .map_err(|e| DockerError::StreamError(e.to_string()))?;
                                    }

                                    let text = String::from_utf8_lossy(&buffer);
                                    let lines = Self::extract_last_lines(&text, DISPLAY_LINES);

                                    if lines.is_empty() {
                                        println!("  [{}s, {} bytes received]", start.elapsed().as_secs(), buffer.len());
                                        lines_printed = 1;
                                    } else {
                                        println!("  [{}s elapsed]", start.elapsed().as_secs());
                                        lines_printed = u16::try_from(lines.len() + 1).unwrap_or(u16::MAX);
                                        for line in lines {
                                            println!("  {}", line.dimmed());
                                        }
                                    }

                                    stdout
                                        .flush()
                                        .map_err(|e| DockerError::StreamError(e.to_string()))?;
                                } else {
                                    println!("  [{}s elapsed]...", start.elapsed().as_secs());
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log::debug!("Log stream ended: {}", e);
                            break Ok(());
                        }
                        None => {
                            break Ok(());
                        }
                    }
                }
            }
        };

        // Normal completion - show final state and save
        if stream_result.is_ok() {
            // Always show final output with all collected data
            if !buffer.is_empty() && is_tty {
                // Clear previous output
                if lines_printed > 0 {
                    let _ = stdout.queue(MoveUp(lines_printed));
                    let _ = stdout.queue(Clear(ClearType::FromCursorDown));
                }

                let text = String::from_utf8_lossy(&buffer);
                let lines = Self::extract_last_lines(&text, DISPLAY_LINES);

                if lines.is_empty() {
                    println!("  [{}s, {} bytes]", start.elapsed().as_secs(), buffer.len());
                } else {
                    println!("  [{}s elapsed]", start.elapsed().as_secs());
                    for line in lines {
                        println!("  {}", line.dimmed());
                    }
                }
                stdout
                    .flush()
                    .map_err(|e| DockerError::StreamError(e.to_string()))?;
            }

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
        log::info!("Full output saved to: {}", log_path.display());
        Ok(())
    }

    fn extract_last_lines(text: &str, count: usize) -> Vec<String> {
        let clean = strip_ansi(text);
        let lines: Vec<&str> = clean
            .split(['\n', '\r'])
            .filter(|s| !s.trim().is_empty())
            .collect();

        let start = lines.len().saturating_sub(count);
        lines
            .get(start..)
            .unwrap_or(&[])
            .iter()
            .map(|s| {
                let trimmed = s.trim();
                if trimmed.len() > 80 {
                    let truncated: String = trimmed.chars().take(77).collect();
                    format!("{}...", truncated)
                } else {
                    trimmed.to_string()
                }
            })
            .collect()
    }
}
