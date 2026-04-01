//! Version command implementation.

use crate::error::Result;

include!(concat!(env!("OUT_DIR"), "/built.rs"));

/// ANSI reset code.
const RESET: &str = "\x1b[0m";

/// Gradient top color: Orange (#FF6B35).
const TOP_RGB: (u8, u8, u8) = (255, 107, 53);

/// Gradient bottom color: Blue (#3B82F6).
const BOTTOM_RGB: (u8, u8, u8) = (59, 130, 246);

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

/// Apply a single color to a line of text.
fn color_line(text: &str, rgb: (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{};{};{}m{}{}", rgb.0, rgb.1, rgb.2, text, RESET)
}

/// Get interpolated color for vertical gradient (0.0 = top, 1.0 = bottom).
fn vertical_gradient_color(t: f32) -> (u8, u8, u8) {
    (
        lerp(TOP_RGB.0, BOTTOM_RGB.0, t),
        lerp(TOP_RGB.1, BOTTOM_RGB.1, t),
        lerp(TOP_RGB.2, BOTTOM_RGB.2, t),
    )
}

/// Apply gradient to a set of lines.
fn apply_gradient(lines: &[&str]) -> Vec<String> {
    let line_count = lines.len();
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let t = if line_count > 1 {
                i as f32 / (line_count - 1) as f32
            } else {
                0.0
            };
            let padded = format!("{:<78}", l);
            color_line(&padded, vertical_gradient_color(t))
        })
        .collect()
}

/// Print the ASCII logo with gradient and version info.
fn print_logo() {
    let subtitle = build_subtitle();
    let centered_subtitle = center(&subtitle, 78);

    // Diamond logo (separate gradient)
    let diamond_lines = [
        "                                    ▄",
        "                                   ▄█▄",
        "                                  ▄███▄",
        "                                 ▄█████▄",
        "                                ▄███████▄",
        "                               ▄████ ████▄",
        "                              ▄████   ████▄",
        "                             ▄████     ████▄",
        "                            ▄████ ▄███▄ ████▄",
        "                           ▄████  ▀███▀  ████▄",
        "                          ▄████           ████▄",
        "                          ▀███████████████████▀",
        "                           ▀█████████████████▀",
        "                            ▀███████████████▀",
        "                             ▀█████████████▀",
        "                              ▀███████████▀",
        "                               ▀█████████▀",
        "                                ▀███████▀",
        "                                 ▀█████▀",
        "                                  ▀███▀",
        "                                   ▀█▀",
        "                                    ▀",
    ];

    // Text logo (separate gradient)
    let text_lines = [
        "                 █████╗ ██████╗ ██╗      ██████╗██╗     ██╗",
        "                ██╔══██╗██╔══██╗██║     ██╔════╝██║     ██║",
        "                ███████║██║  ██║██║     ██║     ██║     ██║",
        "                ██╔══██║██║  ██║██║     ██║     ██║     ██║",
        "                ██║  ██║██████╔╝██║     ╚██████╗███████╗██║",
        "                ╚═╝  ╚═╝╚═════╝ ╚═╝      ╚═════╝╚══════╝╚═╝",
    ];

    let diamond_gradient = apply_gradient(&diamond_lines);
    let text_gradient = apply_gradient(&text_lines);

    println!();
    println!("╭──────────────────────────────────────────────────────────────────────────────╮");
    println!("│                                                                              │");
    for line in &diamond_gradient {
        println!("│{line}│");
    }
    println!("│{:78}│", "");
    for line in &text_gradient {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn lerp_at_zero_returns_start() {
        assert_eq!(lerp(0, 255, 0.0), 0);
    }

    #[test]
    fn lerp_at_one_returns_end() {
        assert_eq!(lerp(0, 255, 1.0), 255);
    }

    #[test]
    fn lerp_at_half_returns_midpoint() {
        assert_eq!(lerp(0, 255, 0.5), 128);
    }

    #[test]
    fn lerp_same_start_end() {
        assert_eq!(lerp(100, 100, 0.5), 100);
    }

    #[test]
    fn center_shorter_than_width() {
        let result = center("hi", 10);
        // center pads left only via format width
        assert!(result.contains("hi"));
        assert!(result.starts_with(' '));
    }

    #[test]
    fn center_equal_to_width() {
        let result = center("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn center_longer_than_width() {
        let result = center("hello world", 5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn build_subtitle_contains_version() {
        let subtitle = build_subtitle();
        assert!(subtitle.starts_with('v'));
        assert!(subtitle.contains('•'));
    }
}
