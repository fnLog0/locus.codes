//! Inline error spans aligned with the transcript grid (no vertical rail).

use ratatui::text::{Line, Span};

use crate::layouts::danger_style;
use crate::theme::LocusPalette;
use crate::utils::{LEFT_PADDING, wrap_lines};

use super::common::{push_timestamp, push_wrapped_plain_continuations};

/// Inline error shown in chat (from SessionEvent::Error).
#[derive(Debug, Clone)]
pub struct ErrorMessage {
    pub text: String,
    pub timestamp: Option<String>,
}

const ERROR_INDICATOR: &str = "✗";

/// Build lines for an error message: indicator + text, following the grid indent.
pub fn error_message_lines(
    msg: &ErrorMessage,
    palette: &LocusPalette,
    width: usize,
) -> Vec<Line<'static>> {
    let indent_len = LEFT_PADDING.len();
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let wrapped = wrap_lines(msg.text.trim(), wrap_width);
    let style = danger_style(palette.danger);
    let indent_span = Span::raw(LEFT_PADDING);

    if wrapped.is_empty() {
        let mut spans = vec![
            indent_span.clone(),
            Span::styled(format!("{} ", ERROR_INDICATOR), style),
        ];
        push_timestamp(&mut spans, msg.timestamp.as_deref(), palette);
        return vec![Line::from(spans)];
    }

    let mut lines = Vec::with_capacity(wrapped.len());
    let first = &wrapped[0];
    let mut first_line = vec![
        indent_span.clone(),
        Span::styled(format!("{} ", ERROR_INDICATOR), style),
    ];
    push_timestamp(&mut first_line, msg.timestamp.as_deref(), palette);
    first_line.push(Span::styled(first.clone(), style));
    lines.push(Line::from(first_line));

    push_wrapped_plain_continuations(&mut lines, &indent_span, 3, &wrapped, style);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::LocusPalette;

    #[test]
    fn error_empty_text() {
        let msg = ErrorMessage {
            text: "".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
    }

    #[test]
    fn error_wraps_long_text() {
        let msg = ErrorMessage {
            text:
                "Connection refused: could not connect to provider endpoint after multiple retries"
                    .into(),
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
    fn error_has_indicator() {
        let msg = ErrorMessage {
            text: "fail".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✗")));
    }

    #[test]
    fn error_continuation_aligns_with_padding() {
        let msg = ErrorMessage {
            text: "one two three four five six seven eight".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 18);
        assert!(lines.len() > 1);
        assert_eq!(lines[1].spans[0].content.as_ref(), "  ");
    }
}
