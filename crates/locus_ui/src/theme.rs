//! Runtime theme with light/dark toggle.
//!
//! Wraps `locus_constant::theme` constants with a switchable theme mode.
//! Components should use this rather than accessing constants directly.

use ratatui::style::Color;

/// Theme mode (light or dark).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

/// Runtime theme with all colors cached for the current mode.
#[derive(Debug, Clone)]
pub struct Theme {
    mode: ThemeMode,
    // Cached colors for current mode
    pub bg: Color,
    pub fg: Color,
    pub card: Color,
    pub primary: Color,
    pub primary_fg: Color,
    pub secondary: Color,
    pub muted: Color,
    pub muted_fg: Color,
    pub faint: Color,
    pub accent: Color,
    pub danger: Color,
    pub success: Color,
    pub warning: Color,
    pub info: Color,
    pub purple: Color,
    pub border: Color,
    pub input: Color,
    pub ring: Color,
    pub code_bg: Color,
    pub tool_bg: Color,
    pub bash_bg: Color,
    pub think_bg: Color,
    pub file_path: Color,
    pub tool_name: Color,
    pub timestamp: Color,
}

impl Theme {
    /// Create a dark theme.
    pub fn dark() -> Self {
        use locus_constant::theme::dark as c;
        Self {
            mode: ThemeMode::Dark,
            bg: rgb(c::BACKGROUND),
            fg: rgb(c::FOREGROUND),
            card: rgb(c::CARD),
            primary: rgb(c::PRIMARY),
            primary_fg: rgb(c::PRIMARY_FG),
            secondary: rgb(c::SECONDARY),
            muted: rgb(c::MUTED),
            muted_fg: rgb(c::MUTED_FG),
            faint: rgb(c::FAINT),
            accent: rgb(c::ACCENT),
            danger: rgb(c::DANGER),
            success: rgb(c::SUCCESS),
            warning: rgb(c::WARNING),
            info: rgb(c::INFO),
            purple: rgb(c::PURPLE),
            border: rgb(c::BORDER),
            input: rgb(c::INPUT),
            ring: rgb(c::RING),
            code_bg: rgb(c::CODE_BG),
            tool_bg: rgb(c::TOOL_BG),
            bash_bg: rgb(c::BASH_BG),
            think_bg: rgb(c::THINK_BG),
            file_path: rgb(c::FILE_PATH),
            tool_name: rgb(c::TOOL_NAME),
            timestamp: rgb(c::TIMESTAMP),
        }
    }

    /// Create a light theme.
    pub fn light() -> Self {
        use locus_constant::theme::light as c;
        Self {
            mode: ThemeMode::Light,
            bg: rgb(c::BACKGROUND),
            fg: rgb(c::FOREGROUND),
            card: rgb(c::CARD),
            primary: rgb(c::PRIMARY),
            primary_fg: rgb(c::PRIMARY_FG),
            secondary: rgb(c::SECONDARY),
            muted: rgb(c::MUTED),
            muted_fg: rgb(c::MUTED_FG),
            faint: rgb(c::FAINT),
            accent: rgb(c::ACCENT),
            danger: rgb(c::DANGER),
            success: rgb(c::SUCCESS),
            warning: rgb(c::WARNING),
            info: rgb(c::INFO),
            purple: rgb(c::INFO), // Light theme doesn't have purple, fallback to info
            border: rgb(c::BORDER),
            input: rgb(c::INPUT),
            ring: rgb(c::RING),
            code_bg: rgb(c::CODE_BG),
            tool_bg: rgb(c::TOOL_BG),
            bash_bg: rgb(c::BASH_BG),
            think_bg: rgb(c::THINK_BG),
            file_path: rgb(c::FILE_PATH),
            tool_name: rgb(c::TOOL_NAME),
            timestamp: rgb(c::TIMESTAMP),
        }
    }

    /// Toggle between light and dark mode.
    pub fn toggle(&mut self) {
        *self = match self.mode {
            ThemeMode::Dark => Self::light(),
            ThemeMode::Light => Self::dark(),
        };
    }

    /// Get the current theme mode.
    pub fn mode(&self) -> ThemeMode {
        self.mode
    }

    /// Create theme from environment variables.
    ///
    /// Checks `NO_COLOR` and `COLOR_TERM`/`TERM` for hints.
    /// Defaults to dark theme.
    pub fn from_env() -> Self {
        // NO_COLOR disables colors (but we still need a theme)
        if std::env::var("NO_COLOR").is_ok() {
            return Self::dark();
        }

        // Check for light theme hint in terminal
        let term = std::env::var("TERM").unwrap_or_default();
        let color_term = std::env::var("COLOR_TERM").unwrap_or_default();

        // Some terminals set TERM_PROGRAM or similar for light mode
        // For now, default to dark as it's the intended aesthetic
        let _ = (term, color_term);
        Self::dark()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Convert RGB tuple to ratatui Color.
fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_toggle_switches_mode() {
        let mut theme = Theme::dark();
        assert_eq!(theme.mode(), ThemeMode::Dark);

        theme.toggle();
        assert_eq!(theme.mode(), ThemeMode::Light);

        theme.toggle();
        assert_eq!(theme.mode(), ThemeMode::Dark);
    }

    #[test]
    fn dark_theme_has_expected_colors() {
        let theme = Theme::dark();
        // Verify dark background
        assert_eq!(theme.bg, Color::Rgb(10, 10, 15));
        // Verify foreground
        assert_eq!(theme.fg, Color::Rgb(192, 202, 245));
    }
}
