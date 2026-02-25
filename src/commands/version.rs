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

/// Print the ASCII logo with gradient and version info.
fn print_logo() {
    let subtitle = build_subtitle();
    let centered_subtitle = center(&subtitle, 78);

    // Logo lines (will be padded to 78 chars)
    // Symmetric diamond with triangle + dot inside
    let logo_lines = [
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
        "",
        "                 █████╗ ██████╗ ██╗      ██████╗██╗     ██╗",
        "                ██╔══██╗██╔══██╗██║     ██╔════╝██║     ██║",
        "                ███████║██║  ██║██║     ██║     ██║     ██║",
        "                ██╔══██║██║  ██║██║     ██║     ██║     ██║",
        "                ██║  ██║██████╔╝██║     ╚██████╗███████╗██║",
        "                ╚═╝  ╚═╝╚═════╝ ╚═╝      ╚═════╝╚══════╝╚═╝",
    ];

    // Apply vertical gradient (orange top -> blue bottom)
    let line_count = logo_lines.len();
    let gradient_lines: Vec<String> = logo_lines
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
