//! Custom cliclack theme for ADI CLI.
//!
//! Provides a Blue/Cyan color scheme with custom Unicode symbols
//! for a polished, consistent visual identity.

use cliclack::{Theme, ThemeState};
use console::Style;

/// ADI CLI custom theme.
///
/// Colors:
/// - Active: Cyan
/// - Submit: Green
/// - Cancel: Yellow
/// - Error: Red
/// - Inactive: Blue (dimmed)
pub struct AdiTheme;

impl Theme for AdiTheme {
    /// Bar color based on state (always blue dim for consistency).
    fn bar_color(&self, state: &ThemeState) -> Style {
        match state {
            ThemeState::Error(_) => Style::new().red(),
            _ => Style::new().blue().dim(),
        }
    }

    /// State symbol color.
    fn state_symbol_color(&self, state: &ThemeState) -> Style {
        match state {
            ThemeState::Active => Style::new().cyan(),
            ThemeState::Submit => Style::new().green(),
            ThemeState::Cancel => Style::new().yellow(),
            ThemeState::Error(_) => Style::new().red(),
        }
    }

    /// State symbol (◆ active, ✔ submit, ◇ cancel, ✖ error).
    fn state_symbol(&self, state: &ThemeState) -> String {
        let color = self.state_symbol_color(state);
        match state {
            ThemeState::Active => color.apply_to("◆"),
            ThemeState::Submit => color.apply_to("✱"),
            ThemeState::Cancel => color.apply_to("◇"),
            ThemeState::Error(_) => color.apply_to("✖"),
        }
        .to_string()
    }

    /// Info symbol: ◈ (diamond with dot) - cyan.
    fn info_symbol(&self) -> String {
        Style::new().cyan().apply_to("◈").to_string()
    }

    /// Warning symbol: ⚠ (warning sign) - yellow.
    fn warning_symbol(&self) -> String {
        Style::new().yellow().apply_to("⚠").to_string()
    }

    /// Error symbol: ✖ (heavy X) - red.
    fn error_symbol(&self) -> String {
        Style::new().red().apply_to("✖").to_string()
    }

    /// Remark/debug symbol: ⚙ (gear) - blue dim.
    fn remark_symbol(&self) -> String {
        Style::new().blue().dim().apply_to("⚙").to_string()
    }

    /// Submit symbol: ✱ (asterisk) - green.
    /// Used for note boxes and submitted prompts.
    fn submit_symbol(&self) -> String {
        Style::new().green().apply_to("✱").to_string()
    }

    /// Intro with cyan background badge.
    fn format_intro(&self, title: &str) -> String {
        let badge = Style::new().on_cyan().black().bold();
        let bar = Style::new().blue().dim();
        format!(
            "{}  {}\n{}\n",
            bar.apply_to("┌"),
            badge.apply_to(format!(" {} ", title)),
            bar.apply_to("│")
        )
    }

    /// Outro with green background badge.
    fn format_outro(&self, message: &str) -> String {
        let badge = Style::new().on_green().black().bold();
        let bar = Style::new().blue().dim();
        format!(
            "{}  {}\n",
            bar.apply_to("└"),
            badge.apply_to(format!(" {} ", message))
        )
    }

    /// Cancel outro with yellow background badge.
    fn format_outro_cancel(&self, message: &str) -> String {
        let badge = Style::new().on_yellow().black().bold();
        let bar = Style::new().blue().dim();
        format!(
            "{}  {}\n",
            bar.apply_to("└"),
            badge.apply_to(format!(" {} ", message))
        )
    }
}
