//! AI "thinking" / reasoning message rendering.
//!
//! Shown in muted style to distinguish from main assistant output.
//! Layout: optional indicator + wrapped content, all muted.

use ratatui::text::{Line, Span};

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;
use crate::utils::{wrap_lines, LEFT_PADDING};

/// AI thinking/reasoning content for display. No dependency on other crates.
#[derive(Debug, Clone)]
pub struct AiThinkMessage {
    pub text: String,
    /// When true, show single line "⋯ Thinking (N lines)".
    pub collapsed: bool,
}

/// Indicator for thinking block (muted).
pub const THINK_INDICATOR: &str = "⋯";

/// Cursor shown at the end of streaming (in-progress) thinking output.
pub const STREAMING_CURSOR: &str = "▌";

/// Build lines for a thinking message: optional indicator, then wrapped text.
/// All in text_muted style (indicator in palette.info when not collapsed). Continuation lines use 2-space indent.
/// When `streaming` is true, a cursor is drawn after the last line. When `cursor_visible` is true, show blinking cursor.
/// When `streaming_truncate_last_n` is Some(n) and streaming, show only the last n logical lines and a "…" line above.
/// When collapsed, single line: "⋯ Thinking (N lines)".
pub fn think_message_lines(
    msg: &AiThinkMessage,
    palette: &LocusPalette,
    width: usize,
    streaming: bool,
    cursor_visible: bool,
    streaming_truncate_last_n: Option<usize>,
) -> Vec<Line<'static>> {
    use crate::layouts::text_style;
    let indent_len = LEFT_PADDING.len();
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let info_style = text_style(palette.info);
    let muted = text_muted_style(palette.text_muted);

    if msg.collapsed {
        let n = msg.text.lines().filter(|l| !l.trim().is_empty()).count().max(1);
        let line = Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(THINK_INDICATOR.to_string(), info_style),
            Span::raw(" "),
            Span::styled(format!("Thinking ({} lines)", n), muted),
        ]);
        return vec![line];
    }

    let effective_text = if streaming && streaming_truncate_last_n.is_some() {
        let n = streaming_truncate_last_n.unwrap_or(3);
        let text_lines: Vec<&str> = msg.text.lines().collect();
        if text_lines.len() > n {
            text_lines[text_lines.len().saturating_sub(n)..].join("\n")
        } else {
            msg.text.trim().to_string()
        }
    } else {
        msg.text.trim().to_string()
    };

    let add_ellipsis_line = streaming
        && streaming_truncate_last_n.is_some()
        && msg.text.lines().count() > streaming_truncate_last_n.unwrap_or(3);

    let wrapped = wrap_lines(&effective_text, wrap_width);
    if wrapped.is_empty() {
        let mut out = Vec::new();
        if add_ellipsis_line {
            out.push(Line::from(vec![
                Span::raw(LEFT_PADDING),
                Span::styled("…", muted),
            ]));
        }
        let mut line = vec![
            Span::raw(LEFT_PADDING),
            Span::styled(THINK_INDICATOR.to_string(), info_style),
            Span::raw(" "),
            Span::styled("Thinking…".to_string(), muted),
        ];
        if streaming && cursor_visible {
            line.push(Span::styled(STREAMING_CURSOR.to_string(), muted));
        }
        out.push(Line::from(line));
        return out;
    }

    let mut lines = Vec::with_capacity(wrapped.len() + if add_ellipsis_line { 1 } else { 0 });
    if add_ellipsis_line {
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled("…", muted),
        ]));
    }
    let mut first_spans = vec![
        Span::raw(LEFT_PADDING),
        Span::styled(THINK_INDICATOR.to_string(), info_style),
        Span::raw(" "),
        Span::styled(wrapped[0].clone(), muted),
    ];
    if streaming && wrapped.len() == 1 && cursor_visible {
        first_spans.push(Span::styled(STREAMING_CURSOR.to_string(), muted));
    }
    lines.push(Line::from(first_spans));

    for (i, seg) in wrapped.iter().skip(1).enumerate() {
        let is_last = i == wrapped.len().saturating_sub(2);
        let mut seg_spans = vec![
            Span::raw(LEFT_PADDING),
            Span::styled(seg.clone(), muted),
        ];
        if streaming && is_last && cursor_visible {
            seg_spans.push(Span::styled(STREAMING_CURSOR.to_string(), muted));
        }
        lines.push(Line::from(seg_spans));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn think_message_lines_has_indicator() {
        let msg = AiThinkMessage {
            text: "Considering the best approach…".into(),
            collapsed: false,
        };
        let palette = LocusPalette::locus_dark();
        let lines = think_message_lines(&msg, &palette, 40, false, true, None);
        assert!(!lines.is_empty());
        assert!(lines[0].spans.iter().any(|s| s.content.as_ref() == THINK_INDICATOR));
    }

    #[test]
    fn think_message_lines_wraps() {
        let msg = AiThinkMessage {
            text: "Step one. Step two with more content here.".into(),
            collapsed: false,
        };
        let palette = LocusPalette::locus_dark();
        let lines = think_message_lines(&msg, &palette, 14, false, true, None);
        assert!(lines.len() > 1);
    }

    #[test]
    fn think_empty_text() {
        let msg = AiThinkMessage { text: "".into(), collapsed: false };
        let palette = LocusPalette::locus_dark();
        let lines = think_message_lines(&msg, &palette, 40, false, true, None);
        assert!(!lines.is_empty());
    }

    #[test]
    fn think_collapsed_shows_line_count() {
        let msg = AiThinkMessage {
            text: "line 1\nline 2\nline 3".into(),
            collapsed: true,
        };
        let palette = LocusPalette::locus_dark();
        let lines = think_message_lines(&msg, &palette, 40, false, true, None);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("3 lines")));
    }

    #[test]
    fn think_streaming_cursor_shown() {
        let msg = AiThinkMessage { text: "thinking".into(), collapsed: false };
        let palette = LocusPalette::locus_dark();
        let lines = think_message_lines(&msg, &palette, 40, true, true, None);
        let has_cursor = lines.iter().any(|l| {
            l.spans.iter().any(|s| s.content.as_ref() == STREAMING_CURSOR)
        });
        assert!(has_cursor);
    }

    #[test]
    fn think_streaming_truncated_shows_ellipsis() {
        let text = (0..20).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let msg = AiThinkMessage { text, collapsed: false };
        let palette = LocusPalette::locus_dark();
        let lines = think_message_lines(&msg, &palette, 40, true, true, Some(3));
        assert!(lines[0].spans.iter().any(|s| s.content.as_ref() == "…"));
    }

    #[test]
    fn think_unicode() {
        let msg = AiThinkMessage { text: "考虑方案 🤔 思考中".into(), collapsed: false };
        let palette = LocusPalette::locus_dark();
        let lines = think_message_lines(&msg, &palette, 40, false, true, None);
        assert!(!lines.is_empty());
    }
}
