//! Popup component with dynamic sizing and centering.
//!
//! Based on stakpak/agent patterns:
//! - Dynamic height clamped between min and percentage of terminal
//! - Centered positioning
//! - Rounded borders by default

use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;
use super::{dynamic_height, MIN_COMPONENT_HEIGHT};

/// A popup with dynamic sizing.
#[derive(Debug, Clone)]
pub struct Popup {
    /// Title shown in top border.
    pub title: String,
    /// Content lines.
    pub content: Vec<String>,
    /// Desired height (will be clamped).
    pub desired_height: u16,
    /// Width percentage of terminal (0.0 - 1.0).
    pub width_percent: f32,
}

impl Popup {
    /// Create a new popup with title and content.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
            desired_height: MIN_COMPONENT_HEIGHT,
            width_percent: 0.5,
        }
    }

    /// Add a content line.
    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.content.push(line.into());
        self.desired_height = (self.content.len() + 2) as u16; // +2 for borders
        self
    }

    /// Set multiple content lines.
    pub fn content(mut self, lines: Vec<String>) -> Self {
        self.content = lines;
        self.desired_height = (self.content.len() + 2) as u16;
        self
    }

    /// Set width as percentage of terminal (0.0 - 1.0).
    pub fn width_percent(mut self, percent: f32) -> Self {
        self.width_percent = percent.clamp(0.1, 0.9);
        self
    }

    /// Calculate the popup area centered in the terminal.
    pub fn area(&self, terminal: Rect) -> Rect {
        let height = dynamic_height(self.desired_height, terminal.height);
        let width = (terminal.width as f32 * self.width_percent) as u16;
        let width = width.max(20).min(terminal.width.saturating_sub(4));

        Rect {
            x: (terminal.width.saturating_sub(width)) / 2,
            y: (terminal.height.saturating_sub(height)) / 2,
            width,
            height,
        }
    }

    /// Render the popup.
    pub fn render(&self, f: &mut Frame, terminal: Rect, theme: &Theme) {
        let area = self.area(terminal);

        // Clear the popup area
        f.render_widget(Clear, area);

        // Build the popup block with rounded borders
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.card));

        let _inner = block.inner(area);

        // Build content lines
        let lines: Vec<Line> = self
            .content
            .iter()
            .map(|line| Line::styled(line.as_str(), Style::default().fg(theme.fg)))
            .collect();

        let paragraph = Paragraph::new(lines).block(block);

        f.render_widget(paragraph, area);
    }
}

/// Shell command popup with prefix handling.
#[derive(Debug, Clone)]
pub struct ShellPopup {
    /// Base popup.
    pub popup: Popup,
    /// Shell mode prefix.
    pub prefix: &'static str,
}

impl ShellPopup {
    /// Create a new shell popup.
    pub fn new() -> Self {
        Self {
            popup: Popup::new("Shell")
                .width_percent(0.6),
            prefix: "$ ",
        }
    }

    /// Set content from command output.
    pub fn output(mut self, output: &str) -> Self {
        let lines: Vec<String> = output.lines().map(|l| format!("{}{}", self.prefix, l)).collect();
        self.popup = self.popup.content(lines);
        self
    }

    /// Render the shell popup.
    pub fn render(&self, f: &mut Frame, terminal: Rect, theme: &Theme) {
        self.popup.render(f, terminal, theme);
    }
}

impl Default for ShellPopup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::POPUP_MAX_HEIGHT_PERCENT;
    use ratatui::layout::Rect;

    #[test]
    fn popup_new() {
        let popup = Popup::new("Test");
        assert_eq!(popup.title, "Test");
        assert!(popup.content.is_empty());
    }

    #[test]
    fn popup_with_lines() {
        let popup = Popup::new("Test")
            .line("Line 1")
            .line("Line 2");
        assert_eq!(popup.content.len(), 2);
    }

    #[test]
    fn popup_area_centering() {
        let popup = Popup::new("Test").width_percent(0.5);
        let terminal = Rect::new(0, 0, 80, 24);
        let area = popup.area(terminal);

        // Should be centered
        assert!(area.x > 0);
        assert!(area.y > 0);
        // Width should be ~40 (50% of 80)
        assert!(area.width >= 20 && area.width <= 40);
    }

    #[test]
    fn dynamic_height_clamping() {
        let popup = Popup::new("Test")
            .content(vec!["a".to_string(); 100].clone());
        let terminal = Rect::new(0, 0, 80, 24);
        let area = popup.area(terminal);

        // Should clamp to max percentage
        let max_height = (24.0 * POPUP_MAX_HEIGHT_PERCENT) as u16;
        assert!(area.height <= max_height);
    }

    #[test]
    fn shell_popup_prefix() {
        let popup = ShellPopup::new();
        assert_eq!(popup.prefix, "$ ");
    }
}
