//! Bordered panel layout: outer area, inner padded content area, and theme-backed block.
//!
//! Uses [utils] for padding and [theme::LocusPalette] for border and background colors.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
};

use crate::theme::LocusPalette;
use crate::utils::{padding, HORIZONTAL_PADDING};
use super::style::{background_style, border_focused_style, border_style};

/// Configurable bordered panel: computes inner [Rect] and a [Block] to render.
#[derive(Debug, Clone)]
pub struct PanelLayout {
    /// Full area of the panel (including border).
    pub outer: Rect,
    /// Inner area after border and padding (where content goes).
    pub inner: Rect,
    /// Horizontal padding applied inside the border (each side).
    pub padding_h: u16,
    /// Vertical padding applied inside the border (each side).
    pub padding_v: u16,
}

impl PanelLayout {
    /// Build panel layout for `area` with optional border and inner padding.
    ///
    /// - If `bordered` is true, the block will have borders and the inner rect is inset by 1 on each side, then by padding.
    /// - Uses `palette.surface_background` for block background and `palette.border` for border.
    pub fn new(
        area: Rect,
        bordered: bool,
        padding_h: u16,
        padding_v: u16,
    ) -> Self {
        let (outer, after_border) = if bordered {
            let inner_w = area.width.saturating_sub(2);
            let inner_h = area.height.saturating_sub(2);
            (
                area,
                Rect {
                    x: area.x + 1,
                    y: area.y + 1,
                    width: inner_w,
                    height: inner_h,
                },
            )
        } else {
            (area, area)
        };
        let inner = padding(after_border, padding_h, padding_v);
        Self {
            outer,
            inner,
            padding_h,
            padding_v,
        }
    }

    /// Panel with default horizontal padding from utils, no vertical padding, bordered.
    pub fn bordered(area: Rect) -> Self {
        Self::new(area, true, HORIZONTAL_PADDING, 0)
    }

    /// Panel with default horizontal padding, no border.
    pub fn plain(area: Rect) -> Self {
        Self::new(area, false, HORIZONTAL_PADDING, 0)
    }
}

/// Build a [Block] for the given [PanelLayout] and palette.
/// Draw this block in `layout.outer`, then render content in `layout.inner`.
pub fn block_for_panel(_layout: &PanelLayout, palette: &LocusPalette, focused: bool) -> Block<'static> {
    let border_color = if focused {
        border_focused_style(palette.pane_focused_border)
    } else {
        border_style(palette.border)
    };
    let bg = background_style(palette.surface_background);
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_color)
        .style(bg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_bordered_inner_smaller() {
        let area = Rect::new(0, 0, 20, 10);
        let layout = PanelLayout::bordered(area);
        assert_eq!(layout.outer, area);
        assert!(layout.inner.width <= area.width);
        assert!(layout.inner.height <= area.height);
        // Inner is inset by border (1) then horizontal padding (HORIZONTAL_PADDING)
        assert_eq!(layout.inner.x, 1 + HORIZONTAL_PADDING);
        assert_eq!(layout.inner.y, 1);
    }

    #[test]
    fn panel_plain_uses_padding() {
        let area = Rect::new(0, 0, 20, 10);
        let layout = PanelLayout::plain(area);
        assert_eq!(layout.outer, area);
        assert_eq!(layout.inner.x, HORIZONTAL_PADDING);
        assert_eq!(layout.inner.width, area.width.saturating_sub(HORIZONTAL_PADDING * 2));
    }
}
