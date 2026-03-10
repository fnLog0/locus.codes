//! Inline error message rendering (✗ icon, danger style).

use ratatui::text::{Line, Span};

use crate::layouts::danger_style;
use crate::theme::LocusPalette;
use crate::utils::wrap_lines;

use super::common::{push_timestamp, push_wrapped_plain_continuations, rail_span};

/// Inline error shown in chat (from SessionEvent::Error).
#[derive(Debug, Clone)]
pub struct ErrorMessage {
    pub text: String,
    pub timestamp: Option<String>,
}

const ERROR_LEFT_BORDER: &str = "█ ";
const ERROR_INDICATOR: &str = "error";

/// Build lines for an error message: ✗ icon in danger, text wrapped like AI message.
pub fn error_message_lines(msg: &ErrorMessage, palette: &LocusPalette, width: usize) -> Vec<Line<'static>> {
    let meta_prefix = format!("{}  ", ERROR_INDICATOR);
    let indent_len = ERROR_LEFT_BORDER.len() + meta_prefix.len();
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let wrapped = wrap_lines(msg.text.trim(), wrap_width);
    let style = danger_style(palette.danger);
    let rail = rail_span(ERROR_LEFT_BORDER, style);

    if wrapped.is_empty() {
        let mut spans = vec![
            rail,
            Span::styled(ERROR_INDICATOR.to_string(), style),
            Span::raw("  "),
        ];
        push_timestamp(&mut spans, msg.timestamp.as_deref(), palette);
        return vec![Line::from(spans)];
    }

    let mut lines = Vec::with_capacity(wrapped.len());
    let first = &wrapped[0];
    let mut first_line = vec![
        rail.clone(),
        Span::styled(ERROR_INDICATOR.to_string(), style),
        Span::raw("  "),
    ];
    push_timestamp(&mut first_line, msg.timestamp.as_deref(), palette);
    first_line.push(Span::styled(first.clone(), style));
    lines.push(Line::from(first_line));

    push_wrapped_plain_continuations(&mut lines, &rail, meta_prefix.len(), &wrapped, style);
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
        assert!(lines[0].spans.iter().any(|s| s.content.contains("error")));
    }

    #[test]
    fn error_continuation_aligns_after_indicator() {
        let msg = ErrorMessage {
            text: "one two three four five six seven eight".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 18);
        assert!(lines.len() > 1);
        assert_eq!(lines[1].spans[1].content.as_ref(), "       ");
    }
}
