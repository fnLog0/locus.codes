//! Shortcut hint layout: fixed line below input (muted style), context-aware hints.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::input::INPUT_PADDING_H;
use super::style::text_muted_style;
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
/// - When streaming: "Streaming…  Ctrl+C: cancel"
/// - When input has text: "Enter: send  Ctrl+U: clear  Ctrl+C: quit"
/// - When input empty: "↑↓: scroll  t: thinking  Ctrl+N: new session  q: quit  Ctrl+C: quit"
pub fn shortcut_line(palette: &LocusPalette, is_streaming: bool, input_has_text: bool) -> Line<'static> {
    let hint = if is_streaming {
        "Streaming…  ·  Ctrl+C: cancel (again to quit)"
    } else if input_has_text {
        "Enter: send  ·  Ctrl+U: clear  ·  Ctrl+C: quit"
    } else {
        "↑↓: scroll  ·  t: thinking  ·  Ctrl+N: new session  ·  q: quit  ·  Ctrl+C: quit"
    };
    Line::from(vec![
        Span::styled(hint.to_string(), text_muted_style(palette.text_muted)),
    ])
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
        let line = shortcut_line(&palette, true, false);
        assert!(line.spans.iter().any(|s| s.content.contains("Streaming")));
    }

    #[test]
    fn shortcut_line_typing() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, false, true);
        assert!(line.spans.iter().any(|s| s.content.contains("Enter")));
    }

    #[test]
    fn shortcut_line_idle() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, false, false);
        assert!(line.spans.iter().any(|s| s.content.contains("scroll")));
    }
}
