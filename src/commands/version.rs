//! Version command implementation.

use crate::error::Result;

include!(concat!(env!("OUT_DIR"), "/built.rs"));

/// ANSI reset code.
const RESET: &str = "\x1b[0m";

/// Gradient start color: Blue (#5B8DEE).
const START_RGB: (u8, u8, u8) = (91, 141, 238);

/// Gradient end color: Purple (#A855F7).
const END_RGB: (u8, u8, u8) = (168, 85, 247);

/// Build the version subtitle with version and commit.
fn build_subtitle() -> String {
    let version = PKG_VERSION;
    let git_commit = GIT_COMMIT_HASH.unwrap_or("unknown");
    let git_commit_short = git_commit.get(..8).unwrap_or(git_commit);

    format!("v{version} • {git_commit_short}")
}

/// Center a string within a given width.
fn center(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        return text.to_string();
    }
    let padding = (width - text_len) / 2;
    format!("{:>width$}", text, width = padding + text_len)
}

/// Interpolate between two color values (0-255).
fn lerp(start: u8, end: u8, t: f32) -> u8 {
    let start = f32::from(start);
    let end = f32::from(end);
    let value = (start + (end - start) * t).round().clamp(0.0, 255.0);
    // SAFETY: value is clamped to 0-255 range
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = value as u8;
    result
}

/// Apply horizontal gradient to a line of text.
fn gradient_line(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return String::new();
    }

    let mut result = String::with_capacity(len * 20);
    for (i, ch) in chars.iter().enumerate() {
        let t = if len > 1 {
            i as f32 / (len - 1) as f32
        } else {
            0.0
        };
        let r = lerp(START_RGB.0, END_RGB.0, t);
        let g = lerp(START_RGB.1, END_RGB.1, t);
        let b = lerp(START_RGB.2, END_RGB.2, t);
        result.push_str(&format!("\x1b[38;2;{r};{g};{b}m{ch}"));
    }
    result.push_str(RESET);
    result
}

/// Print the ASCII logo with gradient and version info.
fn print_logo() {
    let subtitle = build_subtitle();
    let centered_subtitle = center(&subtitle, 78);

    // Logo lines (will be padded to 78 chars)
    let logo_lines = [
        "                 █████╗ ██████╗ ██╗      ██████╗██╗     ██╗",
        "                ██╔══██╗██╔══██╗██║     ██╔════╝██║     ██║",
        "                ███████║██║  ██║██║     ██║     ██║     ██║",
        "                ██╔══██║██║  ██║██║     ██║     ██║     ██║",
        "                ██║  ██║██████╔╝██║     ╚██████╗███████╗██║",
        "                ╚═╝  ╚═╝╚═════╝ ╚═╝      ╚═════╝╚══════╝╚═╝",
    ];

    // Apply gradient and pad each line to exactly 78 chars
    let gradient_lines: Vec<String> = logo_lines
        .iter()
        .map(|l| {
            let padded = format!("{:<78}", l);
            gradient_line(&padded)
        })
        .collect();

    println!();
    println!("╭──────────────────────────────────────────────────────────────────────────────╮");
    println!("│                                                                              │");
    for line in &gradient_lines {
        println!("│{line}│");
    }
    println!("│                                                                              │");
    println!("│{centered_subtitle:<78}│");
    println!("│                                                                              │");
    println!("╰──────────────────────────────────────────────────────────────────────────────╯");
    println!();
}

/// Execute the version command.
pub async fn run() -> Result<()> {
    print_logo();
    Ok(())
}
