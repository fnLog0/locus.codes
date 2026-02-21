//! Input bar layout: bottom strip for the command/input line.
//!
//! Uses [crate::utils] for padding and [crate::theme] for bar style. Does not
//! depend on locus_ui.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Padding},
};

use crate::theme::LocusPalette;
use crate::utils::{horizontal_padding, padding, HORIZONTAL_PADDING};
use super::style::{background_style, border_style};

/// Horizontal padding inside the input block (each side).
pub const INPUT_PADDING_H: u16 = 2;

/// Icon shown at the start of the input line (e.g. prompt).
pub const INPUT_ICON: &str = "▸ ";

/// Layout for the input bar: outer area and inner rect for cursor/content.
#[derive(Debug, Clone)]
pub struct InputLayout {
    /// Full footer strip (e.g. from [super::split::MainSplits::footer]).
    pub area: Rect,
    /// Inner rect with horizontal padding for the input line.
    pub inner: Rect,
}

impl InputLayout {
    /// Build from the footer [Rect]. Uses [crate::utils::horizontal_padding].
    pub fn new(area: Rect) -> Self {
        let inner = horizontal_padding(area);
        Self { area, inner }
    }

    /// With optional vertical padding (e.g. when bar is taller than 1 line).
    pub fn with_vertical_padding(area: Rect, v_pad: u16) -> Self {
        let inner = padding(area, HORIZONTAL_PADDING, v_pad);
        Self { area, inner }
    }
}

/// Block for the input bar: background and optional top border.
/// Draw this in [InputLayout::area], then render input in [InputLayout::inner].
pub fn block_for_input(_layout: &InputLayout, palette: &LocusPalette, with_border: bool) -> Block<'static> {
    let bg = background_style(palette.status_bar_background);
    let block = Block::default().style(bg);
    if with_border {
        block.borders(Borders::TOP).border_style(border_style(palette.border))
    } else {
        block
    }
}

/// Block for the input area with full rounded border and horizontal padding.
/// When focused is true, uses border_focused for glow. Focused is typically always true when input is active.
pub fn block_for_input_bordered(palette: &LocusPalette, focused: bool) -> Block<'static> {
    use ratatui::widgets::BorderType;
    let border_style = if focused {
        super::style::border_focused_style(palette.border_focused)
    } else {
        border_style(palette.border)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(background_style(palette.status_bar_background))
        .padding(Padding::new(INPUT_PADDING_H, INPUT_PADDING_H, 0, 0))
}
