//! User message rendering.
//!
//! Layout:
//! - left padding aligned with tool/channel grid
//! - primary-colored body text per line
//! - timestamps kept but hidden

use ratatui::text::{Line, Span};

use crate::layouts::text_style;
use crate::theme::LocusPalette;
use crate::utils::{LEFT_PADDING, wrap_lines};

use super::common::push_wrapped_plain_continuations;

/// User message for display. No dependency on other crates.
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub text: String,
    /// Optional short timestamp (e.g. "10:32"). Shown in muted style.
    pub timestamp: Option<String>,
}

/// Build lines for a user message: aligned with the tool layout indent and accent body text.
pub fn user_message_lines(
    msg: &UserMessage,
    palette: &LocusPalette,
    width: usize,
) -> Vec<Line<'static>> {
    let indent_len = LEFT_PADDING.len();
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let wrapped = wrap_lines(msg.text.trim(), wrap_width);
    let body_style = text_style(palette.text_accent);

    if wrapped.is_empty() {
        return vec![Line::from(Span::raw(LEFT_PADDING.to_string()))];
    }

    let mut lines = Vec::with_capacity(wrapped.len());
    let first = &wrapped[0];
    let indent_span = Span::raw(LEFT_PADDING);
    let first_line = vec![indent_span.clone(), Span::styled(first.clone(), body_style)];
    lines.push(Line::from(first_line));

    push_wrapped_plain_continuations(
        &mut lines,
        &indent_span,
        LEFT_PADDING.len(),
        &wrapped,
        body_style,
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_lines_first_line_has_primary_body() {
        let msg = UserMessage {
            text: "Hello world".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.content.as_ref() == "Hello world")
        );
        assert!(!lines[0].spans.iter().any(|s| s.content.as_ref() == "you"));
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
        let msg = UserMessage {
            text: "".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
    }

    #[test]
    fn user_message_hides_timestamp() {
        let msg = UserMessage {
            text: "hi".into(),
            timestamp: Some("09:15".into()),
        };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(!lines[0].spans.iter().any(|s| s.content.contains("09:15")));
    }

    #[test]
    fn user_message_emoji() {
        let msg = UserMessage {
            text: "Hello 🌍🎉".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = user_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
    }
}
