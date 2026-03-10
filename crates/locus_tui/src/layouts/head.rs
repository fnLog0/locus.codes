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
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use crate::theme::LocusPalette;
use crate::utils::horizontal_padding;
use super::style::{background_style, border_style, text_muted_style, text_style};

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

/// Short descriptor shown beside the app title in the header.
pub const HEADER_TAGLINE: &str = "terminal workspace";

/// Build the top header line: app title, accent dot, and muted tagline.
pub fn header_title_line(
    title: &str,
    palette: &LocusPalette,
    width: u16,
) -> Line<'static> {
    let title_style = text_style(palette.text).add_modifier(Modifier::BOLD);
    let accent_style = text_style(palette.accent);
    let tagline_style = text_muted_style(palette.text_muted);
    let title_width = UnicodeWidthStr::width(title);
    let tagline_width = UnicodeWidthStr::width(HEADER_TAGLINE);
    let can_show_tagline = width as usize > title_width + tagline_width + 8;

    let mut spans = vec![
        Span::styled("●".to_string(), accent_style),
        Span::raw(" "),
        Span::styled(title.to_string(), title_style),
    ];

    if can_show_tagline {
        spans.push(Span::styled("  ·  ".to_string(), tagline_style));
        spans.push(Span::styled(HEADER_TAGLINE.to_string(), tagline_style));
    }

    Line::from(spans)
}

fn status_badge_style(palette: &LocusPalette, is_streaming: bool, has_error: bool) -> Style {
    let fg = if has_error {
        palette.danger
    } else if is_streaming {
        palette.warning
    } else {
        palette.success
    };

    Style::default()
        .fg(super::style::rgb_to_color(fg))
        .bg(super::style::rgb_to_color(palette.element_background))
        .add_modifier(Modifier::BOLD)
}

/// Build second header line: muted section label on the left and right-aligned status badge.
pub fn header_status_line(
    section: &str,
    status: &str,
    is_streaming: bool,
    has_error: bool,
    palette: &LocusPalette,
    width: u16,
) -> Line<'static> {
    let left_style = text_muted_style(palette.text_muted);
    let left = section.to_string();
    let badge_text = format!(" ● {} ", status);
    let left_width = UnicodeWidthStr::width(left.as_str());
    let badge_width = UnicodeWidthStr::width(badge_text.as_str());
    let gap = (width as usize).saturating_sub(left_width + badge_width);

    Line::from(vec![
        Span::styled(left, left_style),
        Span::raw(" ".repeat(gap)),
        Span::styled(
            badge_text,
            status_badge_style(palette, is_streaming, has_error),
        ),
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
    section: &str,
    status: &str,
    is_streaming: bool,
    has_error: bool,
) {
    let layout = HeadLayout::new(area);
    let block = block_for_head(&layout, palette);
    let bg = background_style(palette.status_bar_background);

    frame.render_widget(block, area);

    if layout.inner.height == 0 {
        return;
    }

    let title_rect = Rect {
        x: layout.inner.x,
        y: layout.inner.y,
        width: layout.inner.width,
        height: 1,
    };
    let title_line = header_title_line(title, palette, layout.inner.width);
    frame.render_widget(Paragraph::new(title_line).style(bg), title_rect);

    if layout.inner.height > 1 {
        let status_rect = Rect {
            x: layout.inner.x,
            y: layout.inner.y.saturating_add(1),
            width: layout.inner.width,
            height: 1,
        };
        let status_line = header_status_line(
            section,
            status,
            is_streaming,
            has_error,
            palette,
            layout.inner.width,
        );
        frame.render_widget(Paragraph::new(status_line).style(bg), status_rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_title_line_contains_title() {
        let palette = LocusPalette::locus_dark();
        let line = header_title_line("locus.codes", &palette, 80);
        assert!(line.spans.iter().any(|s| s.content.contains("locus.codes")));
        assert!(line.spans.iter().any(|s| s.content.contains("terminal workspace")));
    }

    #[test]
    fn header_status_line_contains_section_and_status() {
        let palette = LocusPalette::locus_dark();
        let line = header_status_line("main workspace", "Ready", false, false, &palette, 80);
        assert!(line.spans.iter().any(|s| s.content.contains("main workspace")));
        assert!(line.spans.iter().any(|s| s.content.contains("Ready")));
    }
}
