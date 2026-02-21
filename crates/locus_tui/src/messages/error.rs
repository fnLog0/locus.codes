//! Inline error message rendering (✗ icon, danger style).

use ratatui::text::{Line, Span};

use crate::layouts::danger_style;
use crate::theme::LocusPalette;
use crate::utils::{wrap_lines, LEFT_PADDING};

/// Inline error shown in chat (from SessionEvent::Error).
#[derive(Debug, Clone)]
pub struct ErrorMessage {
    pub text: String,
    pub timestamp: Option<String>,
}

/// Build lines for an error message: ✗ icon in danger, text wrapped like AI message.
pub fn error_message_lines(msg: &ErrorMessage, palette: &LocusPalette, width: usize) -> Vec<Line<'static>> {
    let indent_len = LEFT_PADDING.len();
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let wrapped = wrap_lines(msg.text.trim(), wrap_width);
    let style = danger_style(palette.danger);

    if wrapped.is_empty() {
        let mut spans = vec![
            Span::styled("✗ ", style),
            Span::raw(" "),
        ];
        if let Some(t) = &msg.timestamp {
            spans.push(Span::styled(format!("{} ", t), style));
        }
        return vec![Line::from(spans)];
    }

    let mut lines = Vec::with_capacity(wrapped.len());
    let first = &wrapped[0];
    let mut first_line = vec![
        Span::styled("✗ ", style),
        Span::raw(" "),
    ];
    if let Some(t) = &msg.timestamp {
        first_line.push(Span::styled(format!("{} ", t), style));
    }
    first_line.push(Span::styled(first.clone(), style));
    lines.push(Line::from(first_line));

    for seg in wrapped.iter().skip(1) {
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(seg.clone(), style),
        ]));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_empty_text() {
        let msg = ErrorMessage { text: "".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
    }

    #[test]
    fn error_wraps_long_text() {
        let msg = ErrorMessage {
            text: "Connection refused: could not connect to provider endpoint after multiple retries".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 30);
        assert!(lines.len() > 1);
    }

    #[test]
    fn error_with_timestamp() {
        let msg = ErrorMessage {
            text: "timeout".into(),
            timestamp: Some("14:30".into()),
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("14:30")));
    }

    #[test]
    fn error_has_danger_icon() {
        let msg = ErrorMessage { text: "fail".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✗")));
    }
}
