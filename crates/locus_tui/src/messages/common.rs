//! Shared transcript rendering helpers for rail-based message blocks.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

pub fn rail_span(rail: &str, style: Style) -> Span<'static> {
    Span::styled(rail.to_string(), style)
}

pub fn continuation_padding(width: usize) -> Span<'static> {
    Span::raw(" ".repeat(width))
}

pub fn push_timestamp(
    spans: &mut Vec<Span<'static>>,
    timestamp: Option<&str>,
    palette: &LocusPalette,
) {
    if let Some(timestamp) = timestamp {
        spans.push(Span::styled(
            format!("{timestamp}  "),
            text_muted_style(palette.text_muted),
        ));
    }
}

pub fn push_wrapped_plain_continuations(
    lines: &mut Vec<Line<'static>>,
    rail: &Span<'static>,
    continuation_width: usize,
    wrapped: &[String],
    body_style: Style,
) {
    for segment in wrapped.iter().skip(1) {
        lines.push(Line::from(vec![
            rail.clone(),
            continuation_padding(continuation_width),
            Span::styled(segment.clone(), body_style),
        ]));
    }
}
