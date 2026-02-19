//! Main view layout for the terminal UI.
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │                                                            │
//! │  [Messages - scrollable, takes all remaining space]       │  <- chat
//! │                                                            │
//! ├────────────────────────────────────────────────────────────┤
//! │  ●                          $ shell | / commands           │  <- status
//! ├────────────────────────────────────────────────────────────┤
//! │  > _                                                       │  <- input
//! ├────────────────────────────────────────────────────────────┤
//! │  Enter: send  Ctrl+C: quit  Ctrl+L: theme  ?: help         │  <- shortcuts
//! └────────────────────────────────────────────────────────────┘
//! ```

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app_state::AppState;

/// Height of the status line (1 line).
const STATUS_HEIGHT: u16 = 1;

/// Height of the shortcuts bar (1 line).
const SHORTCUTS_HEIGHT: u16 = 1;

/// Height of the input area (3 lines for padding + content).
const INPUT_HEIGHT: u16 = 3;

/// Render the main view into the frame.
///
/// Layout (bottom-fixed):
/// - Chat (flexible, scrollable)
/// - Status (fixed)
/// - Input (fixed)
/// - Shortcuts (fixed)
pub fn view(f: &mut Frame, state: &mut AppState) {
    let area = f.area();
    let theme = &state.theme;

    // Fill entire frame with theme background first (prevents edge mismatches)
    let bg_block = Block::default().style(Style::default().bg(theme.bg));
    f.render_widget(bg_block, area);

    // Vertical layout: chat takes remaining space, bottom is fixed
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1), // chat takes all remaining space
            Constraint::Length(STATUS_HEIGHT),
            Constraint::Length(INPUT_HEIGHT),
            Constraint::Length(SHORTCUTS_HEIGHT),
        ])
        .split(area);

    let chat_area = vertical[0];
    let status_area = vertical[1];
    let input_area = vertical[2];
    let shortcuts_area = vertical[3];

    // Render chat (scrollable)
    state.chat.render(f, chat_area, theme);

    // Render status line
    render_status(f, state, status_area);

    // Render input
    state.input.render(f, input_area, theme);

    // Render shortcuts
    state.shortcuts.render(f, shortcuts_area, theme);
}

/// Render the status line with loading indicator and mode hints.
fn render_status(f: &mut Frame, state: &AppState, area: Rect) {
    let theme = &state.theme;

    // Fill background for entire status area first
    let bg_block = Block::default().style(Style::default().bg(theme.bg));
    f.render_widget(bg_block, area);

    // Apply horizontal padding for content (no vertical padding for 1-line status)
    const PADDING: u16 = 2;
    let inner = Rect {
        x: area.x.saturating_add(PADDING),
        y: area.y,
        width: area.width.saturating_sub(PADDING.saturating_mul(2)),
        height: area.height,
    };

    let mut spans: Vec<Span> = Vec::new();

    // Left side: loading indicator or idle
    if state.loading {
        spans.push(Span::styled(
            "⟳ thinking...",
            Style::default().fg(theme.warning),
        ));
    } else {
        spans.push(Span::styled("●", Style::default().fg(theme.success)));
    }

    // Right side: mode hints
    let right_hints = if state.shell_mode {
        "$ shell mode"
    } else {
        "$ shell | / commands"
    };

    // Calculate spacing for right alignment
    let left_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let right_len = right_hints.chars().count();
    let spacing = inner.width.saturating_sub((left_len + right_len) as u16);

    spans.push(Span::raw(" ".repeat(spacing as usize)));
    spans.push(Span::styled(right_hints, Style::default().fg(theme.faint)));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_sensible() {
        assert_eq!(STATUS_HEIGHT, 1);
        assert_eq!(SHORTCUTS_HEIGHT, 1);
        assert!(INPUT_HEIGHT >= 3);
    }
}
