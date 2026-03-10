//! User message rendering.
//!
//! Layout (see docs/user-message-plan.md):
//! - First line: accent rail + `you` label + optional timestamp + text start
//! - Continuation: accent rail + wrapped content
//! - Colors from crate::theme: accent rail, muted metadata, primary body

use ratatui::text::{Line, Span};

use crate::layouts::text_style;
use crate::theme::LocusPalette;
use crate::utils::wrap_lines;

use super::common::{
    push_timestamp, push_wrapped_plain_continuations, rail_span,
};

/// User message for display. No dependency on other crates.
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub text: String,
    /// Optional short timestamp (e.g. "10:32"). Shown in muted style.
    pub timestamp: Option<String>,
}

/// Label shown before user message metadata.
pub const USER_INDICATOR: &str = "you";

/// Left rail for user messages.
const USER_LEFT_BORDER: &str = "┃ ";

/// Build lines for a user message: left border (│) in accent, then indicator + optional timestamp + text;
/// continuation lines with same left border + 2-space indent.
pub fn user_message_lines(msg: &UserMessage, palette: &LocusPalette, width: usize) -> Vec<Line<'static>> {
    let meta_prefix = format!("{}  ", USER_INDICATOR);
    let indent_len = USER_LEFT_BORDER.len() + meta_prefix.len();
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let wrapped = wrap_lines(msg.text.trim(), wrap_width);
    let border_span = rail_span(USER_LEFT_BORDER, text_style(palette.accent));
    let label_span = Span::styled(
        USER_INDICATOR.to_string(),
        text_style(palette.accent),
    );
    let meta_gap = Span::raw("  ");

    if wrapped.is_empty() {
        let mut spans = vec![
            border_span,
            label_span,
            meta_gap,
        ];
        push_timestamp(&mut spans, msg.timestamp.as_deref(), palette);
        return vec![Line::from(spans)];
    }

    let mut lines = Vec::with_capacity(wrapped.len());
    let first = &wrapped[0];
    let mut first_line = vec![
        border_span.clone(),
        label_span,
        meta_gap.clone(),
    ];
    push_timestamp(&mut first_line, msg.timestamp.as_deref(), palette);
    first_line.push(Span::styled(first.clone(), text_style(palette.text)));
    lines.push(Line::from(first_line));

    push_wrapped_plain_continuations(
        &mut lines,
        &border_span,
        meta_prefix.len(),
        &wrapped,
        text_style(palette.text),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_lines_first_line_has_indicator() {
        let msg = UserMessage {
            text: "Hello world".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
        assert!(lines[0].spans.iter().any(|s| s.content.as_ref() == USER_INDICATOR));
    }

    #[test]
    fn user_message_lines_wraps_long_text() {
        let msg = UserMessage {
            text: "one two three four five six seven".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 12);
        assert!(lines.len() > 1);
    }

    #[test]
    fn user_message_empty_text() {
        let msg = UserMessage { text: "".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
    }

    #[test]
    fn user_message_with_timestamp() {
        let msg = UserMessage { text: "hi".into(), timestamp: Some("09:15".into()) };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("09:15")));
    }

    #[test]
    fn user_message_emoji() {
        let msg = UserMessage { text: "Hello 🌍🎉".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
    }

    #[test]
    fn user_message_has_left_border() {
        let msg = UserMessage { text: "hi".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("┃")));
    }
}
