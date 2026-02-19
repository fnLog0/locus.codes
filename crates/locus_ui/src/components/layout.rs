//! Layout utilities for consistent spacing and sizing.
//!
//! Provides helper functions for common layout patterns:
//! - Horizontal/vertical padding
//! - Right-aligned value rows
//! - Dynamic height calculation
//! - Spacing marker handling

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
};

use super::constants::*;

/// Apply horizontal padding to a Rect (symmetric left/right).
pub fn horizontal_padding(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(HORIZONTAL_PADDING),
        y: area.y,
        width: area.width.saturating_sub(HORIZONTAL_PADDING.saturating_mul(2)),
        height: area.height,
    }
}

/// Apply horizontal padding with custom amount.
pub fn horizontal_padding_with(area: Rect, padding: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(padding),
        y: area.y,
        width: area.width.saturating_sub(padding.saturating_mul(2)),
        height: area.height,
    }
}

/// Apply vertical padding to a Rect (symmetric top/bottom).
pub fn vertical_padding(area: Rect, padding: u16) -> Rect {
    Rect {
        x: area.x,
        y: area.y.saturating_add(padding),
        width: area.width,
        height: area.height.saturating_sub(padding.saturating_mul(2)),
    }
}

/// Apply padding to all sides of a Rect.
pub fn padding(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

/// Calculate dynamic height for popups/dropdowns with clamping.
///
/// # Arguments
/// * `desired_height` - Ideal height based on content
/// * `terminal_height` - Available vertical space
///
/// # Returns
/// Clamped height between MIN_COMPONENT_HEIGHT and 60% of terminal height
pub fn dynamic_height(desired_height: u16, terminal_height: u16) -> u16 {
    let max_height = (terminal_height as f32 * POPUP_MAX_HEIGHT_PERCENT) as u16;
    desired_height.clamp(MIN_COMPONENT_HEIGHT, max_height.max(MIN_COMPONENT_HEIGHT))
}

/// Create a right-aligned key-value row.
///
/// # Arguments
/// * `label` - Left-side label text
/// * `value` - Right-side value text
/// * `width` - Available width for the row
/// * `label_style` - Style for the label
/// * `value_style` - Style for the value
///
/// # Returns
/// A Line with label, spacing, and right-aligned value
pub fn right_aligned_row(
    label: &str,
    value: &str,
    width: u16,
    label_style: Style,
    value_style: Style,
) -> Line<'static> {
    let label_len = label.len() as u16;
    let value_len = value.len() as u16;
    let right_padding = HORIZONTAL_PADDING;
    let spacing = width.saturating_sub(label_len + value_len + right_padding);

    Line::from(vec![
        Span::styled(label.to_string(), label_style),
        Span::raw(" ".repeat(spacing as usize)),
        Span::styled(value.to_string(), value_style),
    ])
}

/// Check if a line is a spacing marker.
pub fn is_spacing_marker(line: &str) -> bool {
    line.trim() == SPACING_MARKER
}

/// Convert spacing markers in a vector of lines to empty lines.
///
/// This allows content generators to insert `SPACING_MARKER` strings
/// which get converted to actual empty lines during rendering.
pub fn process_spacing_markers(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| {
            let line_text: String = line
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect();
            if is_spacing_marker(&line_text) {
                Line::raw("")
            } else {
                line
            }
        })
        .collect()
}

/// Collapse consecutive empty lines to a maximum count.
///
/// Prevents huge vertical gaps from malformed content.
pub fn collapse_empty_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    let mut consecutive_empty = 0;

    for line in lines {
        let is_empty = line.spans.iter().all(|s| s.content.trim().is_empty());

        if is_empty {
            consecutive_empty += 1;
            if consecutive_empty <= MAX_CONSECUTIVE_EMPTY_LINES {
                result.push(line);
            }
        } else {
            consecutive_empty = 0;
            result.push(line);
        }
    }

    result
}

/// Calculate scroll offset with buffer to keep content visible at edges.
pub fn scroll_with_buffer(
    offset: usize,
    content_height: usize,
    viewport_height: usize,
) -> usize {
    let max_offset = content_height.saturating_sub(viewport_height.saturating_sub(SCROLL_BUFFER_LINES));
    offset.min(max_offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn horizontal_padding_reduces_width() {
        let area = Rect::new(0, 0, 80, 24);
        let padded = horizontal_padding(area);
        assert_eq!(padded.x, 2);
        assert_eq!(padded.width, 76);
        assert_eq!(padded.y, 0);
        assert_eq!(padded.height, 24);
    }

    #[test]
    fn horizontal_padding_custom_amount() {
        let area = Rect::new(0, 0, 80, 24);
        let padded = horizontal_padding_with(area, 4);
        assert_eq!(padded.x, 4);
        assert_eq!(padded.width, 72);
    }

    #[test]
    fn vertical_padding_reduces_height() {
        let area = Rect::new(0, 0, 80, 24);
        let padded = vertical_padding(area, 2);
        assert_eq!(padded.y, 2);
        assert_eq!(padded.height, 20);
        assert_eq!(padded.x, 0);
        assert_eq!(padded.width, 80);
    }

    #[test]
    fn padding_all_sides() {
        let area = Rect::new(0, 0, 80, 24);
        let padded = padding(area, 2, 1);
        assert_eq!(padded.x, 2);
        assert_eq!(padded.y, 1);
        assert_eq!(padded.width, 76);
        assert_eq!(padded.height, 22);
    }

    #[test]
    fn dynamic_height_clamps_to_max() {
        let height = dynamic_height(50, 24);
        let max_expected = (24.0 * 0.6) as u16;
        assert_eq!(height, max_expected);
    }

    #[test]
    fn dynamic_height_clamps_to_min() {
        let height = dynamic_height(1, 24);
        assert_eq!(height, MIN_COMPONENT_HEIGHT);
    }

    #[test]
    fn right_aligned_row_basic() {
        let line = right_aligned_row(
            "Label",
            "Value",
            20,
            Style::default().fg(Color::White),
            Style::default().fg(Color::Gray),
        );
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[0].content, "Label");
        assert_eq!(line.spans[2].content, "Value");
    }

    #[test]
    fn is_spacing_marker_detects_marker() {
        assert!(is_spacing_marker("SPACING_MARKER"));
        assert!(is_spacing_marker("  SPACING_MARKER  "));
        assert!(!is_spacing_marker("Some text"));
    }

    #[test]
    fn collapse_empty_lines_limits_gaps() {
        let lines = vec![
            Line::raw("text"),
            Line::raw(""),
            Line::raw(""),
            Line::raw(""),
            Line::raw(""),
            Line::raw("more text"),
        ];
        let collapsed = collapse_empty_lines(lines);
        // Should have max 2 consecutive empty lines (4 input empties -> 2 kept)
        assert_eq!(collapsed.len(), 4); // text, "", "", "more text"
    }

    #[test]
    fn scroll_with_buffer_limits_offset() {
        let offset = scroll_with_buffer(100, 50, 24);
        // max_offset = 50 - (24 - 2) = 28
        assert_eq!(offset, 28);
    }
}
