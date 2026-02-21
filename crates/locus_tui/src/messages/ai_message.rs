//! AI / assistant message rendering.
//!
//! Layout: indicator + body text; continuation lines with 2-space indent.
//! Colors from crate::theme: accent (indicator), text (body).

use ratatui::text::{Line, Span};

use crate::layouts::text_style;
use super::markdown::{
    has_block_markdown, has_inline_markdown, parse_blocks, parse_inline_markdown, render_blocks_to_lines,
};
use crate::theme::LocusPalette;
use crate::utils::{wrap_lines, LEFT_PADDING};

/// AI/assistant message for display. No dependency on other crates.
#[derive(Debug, Clone)]
pub struct AiMessage {
    pub text: String,
    /// Optional short timestamp (e.g. "10:32"). Shown in muted style.
    pub timestamp: Option<String>,
}

/// Indicator shown before AI message (accent color).
pub const AI_INDICATOR: &str = "▸";

/// Left border (2-char) for AI messages (muted).
const AI_LEFT_BORDER: &str = "│ ";

/// Cursor shown at the end of streaming (in-progress) AI output.
pub const STREAMING_CURSOR: &str = "▌";

/// Build lines for an AI message: first line with indicator + optional timestamp + text start;
/// continuation lines with 2-space indent. Wrap at `width - 2` for body.
/// When `streaming` is true, a cursor is drawn after the last line to show output is in progress.
/// When `cursor_visible` is true (and streaming), show the blinking cursor.
pub fn ai_message_lines(
    msg: &AiMessage,
    palette: &LocusPalette,
    width: usize,
    streaming: bool,
    cursor_visible: bool,
) -> Vec<Line<'static>> {
    use crate::layouts::text_muted_style;
    let border_span = Span::styled(AI_LEFT_BORDER.to_string(), text_muted_style(palette.text_muted));
    let indent_len = LEFT_PADDING.len() + AI_LEFT_BORDER.len();

    let mut first_prefix = vec![
        Span::styled(AI_INDICATOR.to_string(), text_style(palette.accent)),
        Span::raw(" "),
    ];
    if let Some(t) = &msg.timestamp {
        first_prefix.push(Span::styled(
            format!("{} ", t),
            text_muted_style(palette.text_muted),
        ));
    }

    // During streaming, skip block-level markdown to avoid parse_blocks/render_blocks every frame (prevents TUI hang).
    if !streaming && has_block_markdown(&msg.text) {
        let blocks = parse_blocks(msg.text.trim());
        let mut lines = render_blocks_to_lines(
            &blocks,
            palette,
            width,
            indent_len,
            &border_span,
            Some(first_prefix),
        );
        if streaming && cursor_visible && !lines.is_empty() {
            let last = lines.len() - 1;
            let mut last_line = std::mem::take(&mut lines[last]);
            last_line.spans.push(Span::styled(
                STREAMING_CURSOR.to_string(),
                text_style(palette.accent),
            ));
            lines[last] = last_line;
        }
        return lines;
    }

    let wrap_width = width.saturating_sub(indent_len).max(1);
    let wrapped = wrap_lines(msg.text.trim(), wrap_width);
    if wrapped.is_empty() {
        let mut line = vec![border_span];
        line.extend(first_prefix);
        if streaming && cursor_visible {
            line.push(Span::styled(
                STREAMING_CURSOR.to_string(),
                text_style(palette.accent),
            ));
        }
        return vec![Line::from(line)];
    }

    let mut lines = Vec::with_capacity(wrapped.len());
    let first = &wrapped[0];
    let mut first_spans = vec![border_span.clone()];
    first_spans.extend(first_prefix);
    if streaming {
        first_spans.push(Span::styled(first.clone(), text_style(palette.text)));
    } else if has_inline_markdown(first) {
        first_spans.extend(parse_inline_markdown(first, palette));
    } else {
        first_spans.push(Span::styled(first.clone(), text_style(palette.text)));
    }
    if streaming && wrapped.len() == 1 && cursor_visible {
        first_spans.push(Span::styled(
            STREAMING_CURSOR.to_string(),
            text_style(palette.accent),
        ));
    }
    lines.push(Line::from(first_spans));

    for (i, seg) in wrapped.iter().skip(1).enumerate() {
        let is_last = i == wrapped.len().saturating_sub(2);
        let mut seg_spans = vec![border_span.clone(), Span::raw(LEFT_PADDING)];
        if streaming {
            seg_spans.push(Span::styled(seg.clone(), text_style(palette.text)));
        } else if has_inline_markdown(seg) {
            seg_spans.extend(parse_inline_markdown(seg, palette));
        } else {
            seg_spans.push(Span::styled(seg.clone(), text_style(palette.text)));
        }
        if streaming && is_last && cursor_visible {
            seg_spans.push(Span::styled(
                STREAMING_CURSOR.to_string(),
                text_style(palette.accent),
            ));
        }
        lines.push(Line::from(seg_spans));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_message_lines_first_line_has_indicator() {
        let msg = AiMessage {
            text: "Here is the fix.".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 40, false, true);
        assert!(!lines.is_empty());
        assert!(lines[0].spans.iter().any(|s| s.content.as_ref() == AI_INDICATOR));
    }

    #[test]
    fn ai_message_lines_wraps() {
        let msg = AiMessage {
            text: "First line. Second line with more words.".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 15, false, true);
        assert!(lines.len() > 1);
    }

    #[test]
    fn ai_message_empty_text() {
        let msg = AiMessage { text: "".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 40, false, true);
        assert!(!lines.is_empty());
    }

    #[test]
    fn ai_message_unicode_emoji() {
        let msg = AiMessage { text: "Hello 🎉 世界 done".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 40, false, true);
        assert!(!lines.is_empty());
    }

    #[test]
    fn ai_message_streaming_cursor_shown() {
        let msg = AiMessage { text: "partial".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 40, true, true);
        let has_cursor = lines.iter().any(|l| {
            l.spans.iter().any(|s| s.content.as_ref() == STREAMING_CURSOR)
        });
        assert!(has_cursor);
    }

    #[test]
    fn ai_message_no_cursor_when_not_streaming() {
        let msg = AiMessage { text: "done".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 40, false, true);
        let has_cursor = lines.iter().any(|l| {
            l.spans.iter().any(|s| s.content.as_ref() == STREAMING_CURSOR)
        });
        assert!(!has_cursor);
    }

    #[test]
    fn ai_message_with_timestamp() {
        let msg = AiMessage { text: "hi".into(), timestamp: Some("10:30".into()) };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 40, false, true);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("10:30")));
    }

    #[test]
    fn ai_message_long_single_word() {
        let msg = AiMessage { text: "a".repeat(500), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = ai_message_lines(&msg, &palette, 20, false, true);
        assert!(!lines.is_empty());
    }
}
