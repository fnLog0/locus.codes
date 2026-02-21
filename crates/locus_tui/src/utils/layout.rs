//! Layout helpers for Rects and lines.
//!
//! Use these with [ratatui::layout::Rect] to apply padding and compute
//! dynamic heights. Spacing markers are for message content that gets
//! converted to empty lines when rendering.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
};

use crate::utils::constants::*;

/// Apply horizontal padding to a Rect (symmetric left/right).
#[inline]
pub fn horizontal_padding(area: Rect) -> Rect {
    horizontal_padding_with(area, HORIZONTAL_PADDING)
}

/// Apply horizontal padding with a custom amount.
#[inline]
pub fn horizontal_padding_with(area: Rect, pad: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(pad),
        y: area.y,
        width: area.width.saturating_sub(pad.saturating_mul(2)),
        height: area.height,
    }
}

/// Apply vertical padding to a Rect (symmetric top/bottom).
#[inline]
pub fn vertical_padding(area: Rect, pad: u16) -> Rect {
    Rect {
        x: area.x,
        y: area.y.saturating_add(pad),
        width: area.width,
        height: area.height.saturating_sub(pad.saturating_mul(2)),
    }
}

/// Apply padding on all four sides.
#[inline]
pub fn padding(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

/// Clamp height for popups/dropdowns between [MIN_COMPONENT_HEIGHT] and
/// [POPUP_MAX_HEIGHT_PERCENT] of `terminal_height`.
pub fn dynamic_height(desired_height: u16, terminal_height: u16) -> u16 {
    let max_h = (terminal_height as f32 * POPUP_MAX_HEIGHT_PERCENT) as u16;
    let max_h = max_h.max(MIN_COMPONENT_HEIGHT);
    desired_height.clamp(MIN_COMPONENT_HEIGHT, max_h)
}

/// Build a single line with left label, flexible spacing, and right-aligned value.
pub fn right_aligned_row(
    label: &str,
    value: &str,
    width: u16,
    label_style: Style,
    value_style: Style,
) -> Line<'static> {
    let label_len = label.len() as u16;
    let value_len = value.len() as u16;
    let right_pad = HORIZONTAL_PADDING;
    let gap = width.saturating_sub(label_len + value_len + right_pad);

    Line::from(vec![
        Span::styled(label.to_string(), label_style),
        Span::raw(" ".repeat(gap as usize)),
        Span::styled(value.to_string(), value_style),
    ])
}

/// True if the line content is the spacing marker (after trim).
pub fn is_spacing_marker(line: &str) -> bool {
    line.trim() == SPACING_MARKER
}

/// Replace spacing-marker lines with empty lines in the given vector.
pub fn process_spacing_markers(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if is_spacing_marker(&text) {
                Line::raw("")
            } else {
                line
            }
        })
        .collect()
}

/// Collapse runs of empty lines to at most [MAX_CONSECUTIVE_EMPTY_LINES].
pub fn collapse_empty_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut consecutive_empty = 0;

    for line in lines {
        let empty = line.spans.iter().all(|s| s.content.trim().is_empty());
        if empty {
            consecutive_empty += 1;
            if consecutive_empty <= MAX_CONSECUTIVE_EMPTY_LINES {
                out.push(line);
            }
        } else {
            consecutive_empty = 0;
            out.push(line);
        }
    }
    out
}

/// Compute scroll offset: clamp so we never skip past the end of content.
/// Max offset is content_height - viewport_height so the last line of content can be at the bottom of the viewport.
pub fn scroll_with_buffer(
    offset: usize,
    content_height: usize,
    viewport_height: usize,
) -> usize {
    let max_offset = content_height.saturating_sub(viewport_height);
    offset.min(max_offset)
}
