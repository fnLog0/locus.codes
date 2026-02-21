//! Map theme palette to ratatui styles for use in layout components.
//!
//! All colors come from [LocusPalette]; use these helpers so layout chrome
//! (borders, backgrounds, text) stays consistent with the theme.

use ratatui::style::{Color, Style};

use crate::theme::Rgb;

/// Convert theme [Rgb] to ratatui [Color].
#[inline]
pub fn rgb_to_color(rgb: Rgb) -> Color {
    let (r, g, b) = rgb.tuple();
    Color::Rgb(r, g, b)
}

/// Style for panel borders (border color, no fill).
pub fn border_style(border_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(border_rgb))
}

/// Style for panel background only (e.g. inner fill).
pub fn background_style(bg_rgb: Rgb) -> Style {
    Style::default().bg(rgb_to_color(bg_rgb))
}

/// Style for focused panel border (e.g. accent).
pub fn border_focused_style(border_focused_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(border_focused_rgb))
}

/// Style for primary text on a panel (e.g. palette.text).
pub fn text_style(text_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(text_rgb))
}

/// Style for muted/secondary text (e.g. palette.text_muted).
pub fn text_muted_style(text_muted_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(text_muted_rgb))
}

/// Style for success state (e.g. tool done, success).
pub fn success_style(success_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(success_rgb))
}

/// Style for error/danger state (e.g. tool failed).
pub fn danger_style(danger_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(danger_rgb))
}

/// Style for warning state (e.g. streaming indicator).
pub fn warning_style(warning_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(warning_rgb))
}

/// Style for info (e.g. thinking indicator).
pub fn info_style(info_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(info_rgb))
}
