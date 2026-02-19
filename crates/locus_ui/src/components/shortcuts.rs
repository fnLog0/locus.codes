//! Shortcuts bar component - bottom bar showing keyboard shortcuts.
//!
//! Layout:
//! ```text
//! Enter: send  Ctrl+C: quit  Ctrl+L: theme  ?: help  ↑↓: scroll
//! ```

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme::Theme;

/// Shortcuts bar showing keyboard hints.
#[derive(Debug, Clone)]
pub struct ShortcutsBar {
    shortcuts: Vec<(String, String)>,
}

impl ShortcutsBar {
    /// Create shortcuts bar with default shortcuts.
    pub fn new() -> Self {
        Self {
            shortcuts: vec![
                ("Enter".into(), "send".into()),
                ("Ctrl+C".into(), "quit".into()),
                ("Ctrl+L".into(), "theme".into()),
                ("?".into(), "help".into()),
                ("\u{2191}\u{2193}".into(), "scroll".into()), // ↑↓
            ],
        }
    }

    /// Create with custom shortcuts.
    pub fn with_shortcuts(shortcuts: Vec<(String, String)>) -> Self {
        Self { shortcuts }
    }

    /// Render the shortcuts bar into the frame.
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let mut spans: Vec<Span> = Vec::new();

        for (i, (key, action)) in self.shortcuts.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  ")); // Two spaces between shortcuts
            }
            spans.push(Span::styled(
                key,
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(": ", Style::default().fg(theme.muted_fg)));
            spans.push(Span::styled(action, Style::default().fg(theme.muted_fg)));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).style(Style::default().bg(theme.secondary));
        f.render_widget(paragraph, area);
    }
}

impl Default for ShortcutsBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_bar_new() {
        let bar = ShortcutsBar::new();
        assert_eq!(bar.shortcuts.len(), 5);
    }
}
