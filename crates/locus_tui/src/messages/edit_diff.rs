//! Dedicated diff block: bordered, with line numbers. Renders [EditDiffMessage].

use ratatui::text::{Line, Span};

use crate::diff::{is_simple_change, line_diff_with_numbers, render_line_diff_block};
use crate::layouts::{danger_style, success_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

use super::tool::EditDiffMessage;

/// Fixed number of diff content lines shown at a time (user presses `d` to show next 12).
pub const DIFF_PAGE_SIZE: usize = 12;

/// Build lines for a dedicated diff block: border top, title, line-numbered diff (window of 12 lines), footer if more, border bottom.
/// Returns empty vec when the change is not "simple" (avoids huge blocks).
/// `start_line`: offset into content lines (0, 12, 24, …). `max_lines`: page size (typically 12).
pub fn edit_diff_block_lines(
    msg: &EditDiffMessage,
    palette: &LocusPalette,
    width: usize,
    start_line: usize,
    max_lines: usize,
) -> Vec<Line<'static>> {
    if !is_simple_change(&msg.old_content, &msg.new_content) {
        let mut lines = Vec::new();
        let inner_width = width.saturating_sub(LEFT_PADDING.len());
        let border_len = inner_width.saturating_sub(2);
        let top = "┌".to_string() + &"─".repeat(border_len) + "┐";
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(top, text_muted_style(palette.text_muted)),
        ]));
        let title_line = format!("│ diff: {} (file updated — large change)", msg.path);
        let title_trim = title_line.chars().take(inner_width).collect::<String>();
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(title_trim, text_muted_style(palette.text_muted)),
        ]));
        let bottom = "└".to_string() + &"─".repeat(border_len) + "┘";
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(bottom, text_muted_style(palette.text_muted)),
        ]));
        return lines;
    }

    let mut lines = Vec::new();
    let inner_width = width.saturating_sub(LEFT_PADDING.len());
    let content_width = inner_width.saturating_sub(15); // "│     -     - │ - " ~ 15
    let border_len = inner_width.saturating_sub(2);

    let title = format!(" diff: {} ", msg.path);
    let top_border = if title.len() + 2 <= border_len {
        "┌".to_string() + &title + &"─".repeat(border_len.saturating_sub(title.len())) + "┐"
    } else {
        "┌".to_string() + &"─".repeat(border_len) + "┐"
    };
    lines.push(Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(top_border, text_muted_style(palette.text_muted)),
    ]));

    let rows = line_diff_with_numbers(&msg.old_content, &msg.new_content);
    let all_diff_lines = render_line_diff_block(
        &rows,
        text_style(palette.text),
        success_style(palette.success),
        danger_style(palette.danger),
        content_width,
    );
    let total = all_diff_lines.len();
    let start = if total <= max_lines {
        0
    } else {
        start_line.min(total.saturating_sub(max_lines))
    };
    let end = (start + max_lines).min(total);
    let muted = text_muted_style(palette.text_muted);
    for line in &all_diff_lines[start..end] {
        let mut spans = vec![Span::raw(LEFT_PADDING), Span::styled("│ ", muted)];
        spans.extend(line.spans.clone());
        lines.push(Line::from(spans));
    }

    let more_count = total.saturating_sub(end);
    if more_count > 0 {
        let footer = format!("│ ▼ {} more lines (d)", more_count);
        let footer_trim = footer.chars().take(inner_width).collect::<String>();
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(footer_trim, muted),
        ]));
    }

    let bottom_border = "└".to_string() + &"─".repeat(border_len) + "┘";
    lines.push(Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(bottom_border, text_muted_style(palette.text_muted)),
    ]));

    lines
}
