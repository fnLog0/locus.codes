//! Shortcut hint layout: fixed line below input (muted style), context-aware hints.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::input::INPUT_PADDING_H;
use super::style::{text_muted_style, text_style};
use crate::theme::LocusPalette;

/// Horizontal inset so shortcut aligns with input content (input border + input padding).
const SHORTCUT_INSET_H: u16 = 1 + INPUT_PADDING_H;

/// Rect for the shortcut line with horizontal padding so it aligns with the input content above.
pub fn shortcut_inner_rect(area: Rect) -> Rect {
    let inset = SHORTCUT_INSET_H;
    let w = area.width.saturating_sub(inset.saturating_mul(2));
    Rect {
        x: area.x.saturating_add(inset),
        y: area.y,
        width: w,
        height: area.height,
    }
}

/// Build the shortcut line for the footer. Dynamic based on state:
/// - When active: "<phase>  Ctrl+C: cancel  Ctrl+D: logs"
/// - When input has text: "Enter: send  Ctrl+U: clear  Ctrl+K: kill  Ctrl+C: quit"
/// - When input empty: "↑↓: scroll  PgUp/PgDn: faster  Ctrl+N: new  Ctrl+D: logs"
pub fn shortcut_line(
    palette: &LocusPalette,
    active_label: Option<&str>,
    active_glyph: Option<&str>,
    input_has_text: bool,
    has_diff_pager: bool,
    has_ai_history: bool,
) -> Line<'static> {
    let key_style = text_style(palette.text);
    let desc_style = text_muted_style(palette.text_muted);
    let sep_style = text_muted_style(palette.text_disabled);
    let streaming_style = text_style(palette.warning);

    let mut spans = Vec::new();

    if let Some(label) = active_label {
        spans.push(Span::styled(
            format!("{} ", active_glyph.unwrap_or("●")),
            streaming_style,
        ));
        spans.push(Span::styled(label.to_string(), streaming_style));
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        push_shortcut(&mut spans, "Ctrl+C", "cancel", key_style, desc_style);
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        push_shortcut(&mut spans, "Ctrl+D", "logs", key_style, desc_style);
    } else if input_has_text {
        push_shortcut(&mut spans, "Enter", "send", key_style, desc_style);
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        push_shortcut(&mut spans, "Ctrl+U", "clear", key_style, desc_style);
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        push_shortcut(&mut spans, "Ctrl+K", "kill", key_style, desc_style);
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        push_shortcut(&mut spans, "Ctrl+C", "quit", key_style, desc_style);
    } else {
        push_shortcut(&mut spans, "↑↓", "scroll", key_style, desc_style);
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        push_shortcut(&mut spans, "PgUp/PgDn", "faster", key_style, desc_style);
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        if has_diff_pager {
            push_shortcut(&mut spans, "d", "next diff", key_style, desc_style);
        } else if has_ai_history {
            push_shortcut(&mut spans, "Ctrl+Y", "copy reply", key_style, desc_style);
        } else {
            push_shortcut(&mut spans, "Ctrl+N", "new", key_style, desc_style);
        }
        spans.push(Span::styled("  ·  ".to_string(), sep_style));
        push_shortcut(&mut spans, "Ctrl+D", "logs", key_style, desc_style);
    }

    Line::from(spans)
}

fn push_shortcut(
    spans: &mut Vec<Span<'static>>,
    key: &str,
    description: &str,
    key_style: ratatui::style::Style,
    desc_style: ratatui::style::Style,
) {
    spans.push(Span::styled(key.to_string(), key_style));
    spans.push(Span::styled(
        format!(": {}", description),
        desc_style,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_inner_rect_zero_width() {
        let area = Rect::new(0, 0, 0, 1);
        let inner = shortcut_inner_rect(area);
        assert_eq!(inner.width, 0);
    }

    #[test]
    fn shortcut_inner_rect_small_width() {
        let area = Rect::new(0, 0, 4, 1);
        let inner = shortcut_inner_rect(area);
        assert!(inner.width <= area.width);
    }

    #[test]
    fn shortcut_line_streaming() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, Some("Responding"), Some("◔"), false, false, false);
        assert!(line.spans.iter().any(|s| s.content.contains("Responding")));
    }

    #[test]
    fn shortcut_line_typing() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, None, None, true, false, false);
        assert!(line.spans.iter().any(|s| s.content.contains("Enter")));
        assert!(line.spans.iter().any(|s| s.content.contains("Ctrl+K")));
    }

    #[test]
    fn shortcut_line_idle() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, None, None, false, false, true);
        assert!(line.spans.iter().any(|s| s.content.contains("copy reply")));
    }
}
