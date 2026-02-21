//! Header strip layout: top bar with optional title and right-aligned status (with colored dot).
//!
//! Uses [crate::utils] for padding and [crate::theme] for colors. Does not
//! depend on locus_ui.

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use ratatui::Frame;
use ratatui::style::Modifier;
use ratatui::text::Span;

use crate::theme::LocusPalette;
use crate::utils::horizontal_padding;
use super::style::{background_style, border_style, text_muted_style, text_style, danger_style, warning_style, success_style};

/// Layout for the main app header: outer area and padded inner rect for content.
#[derive(Debug, Clone)]
pub struct HeadLayout {
    /// Full header strip (e.g. from [super::split::MainSplits::header]).
    pub area: Rect,
    /// Inner rect with horizontal padding for title and right text.
    pub inner: Rect,
}

impl HeadLayout {
    /// Build from the header [Rect]. Uses [crate::utils::horizontal_padding].
    pub fn new(area: Rect) -> Self {
        let inner = horizontal_padding(area);
        Self { area, inner }
    }
}

/// Build first header line: title (bold) left, then right-aligned status with colored dot.
/// is_streaming: yellow dot; has_error: red dot; else green dot.
pub fn header_line(
    title: &str,
    right: &str,
    is_streaming: bool,
    has_error: bool,
    palette: &LocusPalette,
    width: u16,
) -> Line<'static> {
    let title_style = text_style(palette.text).add_modifier(Modifier::BOLD);
    let dot_style = if has_error {
        danger_style(palette.danger)
    } else if is_streaming {
        warning_style(palette.warning)
    } else {
        success_style(palette.success)
    };
    let right_style = text_muted_style(palette.text_muted);
    let left_len = title.len() + 1;
    let right_len = 2 + right.len(); // "● " + status
    let gap = (width as usize).saturating_sub(left_len + right_len).max(0);
    Line::from(vec![
        Span::styled(title.to_string(), title_style),
        Span::raw(" ".repeat(gap)),
        Span::styled("● ".to_string(), dot_style),
        Span::styled(right.to_string(), right_style),
    ])
}

/// Block for the header bar: full-width background, bottom border on second line.
pub fn block_for_head(_layout: &HeadLayout, palette: &LocusPalette) -> Block<'static> {
    Block::default()
        .borders(Borders::BOTTOM)
        .border_style(border_style(palette.border))
        .style(background_style(palette.status_bar_background))
}

/// Default title shown in the header.
pub const HEADER_TITLE: &str = "locus.codes";

/// Default status when none is set.
pub const HEADER_STATUS_READY: &str = "Ready";

/// Draw the header: two-line block (title line, then border), status with colored dot.
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    palette: &LocusPalette,
    title: &str,
    status: &str,
    is_streaming: bool,
    has_error: bool,
) {
    let layout = HeadLayout::new(area);
    let block = block_for_head(&layout, palette);
    let line = header_line(title, status, is_streaming, has_error, palette, layout.inner.width);
    let bg = background_style(palette.status_bar_background);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(line).style(bg), layout.inner);
}
