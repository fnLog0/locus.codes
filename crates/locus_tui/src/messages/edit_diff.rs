//! Dedicated diff block: bordered, with line numbers. Renders [EditDiffMessage].

use ratatui::text::{Line, Span};

use crate::diff::{is_simple_change, line_diff_with_numbers, render_line_diff_block};
use crate::layouts::{danger_style, success_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

use super::tool::EditDiffMessage;

/// Fixed number of diff content lines shown at a time (user presses `d` to show next 12).
pub const DIFF_PAGE_SIZE: usize = 12;
const DIFF_HEADER_PREFIX: &str = "╭─ ";
const DIFF_LEFT_BORDER: &str = "│ ";
const DIFF_FOOTER_PREFIX: &str = "╰─ ";

fn diff_header_line(msg: &EditDiffMessage, palette: &LocusPalette, width: usize) -> Line<'static> {
    let max_path_width = width
        .saturating_sub(LEFT_PADDING.len() + DIFF_HEADER_PREFIX.len() + 8)
        .max(1);
    let path = if msg.path.chars().count() > max_path_width {
        let mut truncated: String = msg
            .path
            .chars()
            .take(max_path_width.saturating_sub(1))
            .collect();
        truncated.push('…');
        truncated
    } else {
        msg.path.clone()
    };

    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(
            DIFF_HEADER_PREFIX.to_string(),
            text_muted_style(palette.text_muted),
        ),
        Span::styled("diff".to_string(), text_style(palette.accent)),
        Span::raw("  "),
        Span::styled(path, text_style(palette.text)),
    ])
}

fn diff_meta_line(palette: &LocusPalette, text: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(
            DIFF_LEFT_BORDER.to_string(),
            text_muted_style(palette.text_muted),
        ),
        Span::styled(text.into(), text_muted_style(palette.text_muted)),
    ])
}

fn diff_footer_line(palette: &LocusPalette, text: Option<String>) -> Line<'static> {
    let mut spans = vec![
        Span::raw(LEFT_PADDING),
        Span::styled(
            DIFF_FOOTER_PREFIX.to_string(),
            text_muted_style(palette.text_muted),
        ),
    ];
    if let Some(text) = text {
        spans.push(Span::styled(text, text_muted_style(palette.text_muted)));
    }
    Line::from(spans)
}

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
        return vec![
            diff_header_line(msg, palette, width),
            diff_meta_line(palette, "large update; diff preview omitted"),
            diff_footer_line(palette, Some("file updated".to_string())),
        ];
    }

    let mut lines = Vec::new();
    let content_width = width
        .saturating_sub(LEFT_PADDING.len() + DIFF_LEFT_BORDER.len() + 15)
        .max(1); // line numbers + marker gutter

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
    lines.push(diff_header_line(msg, palette, width));
    lines.push(diff_meta_line(
        palette,
        format!(
            "showing {}-{} of {} lines",
            start + 1,
            end.max(start + 1),
            total.max(1)
        ),
    ));
    for line in &all_diff_lines[start..end] {
        let mut spans = vec![
            Span::raw(LEFT_PADDING),
            Span::styled(DIFF_LEFT_BORDER.to_string(), muted),
        ];
        spans.extend(line.spans.clone());
        lines.push(Line::from(spans));
    }

    let more_count = total.saturating_sub(end);
    if more_count > 0 {
        lines.push(diff_meta_line(
            palette,
            format!("{} more lines  press d", more_count),
        ));
    }

    lines.push(diff_footer_line(palette, None));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_diff_has_header_and_footer() {
        let palette = LocusPalette::locus_dark();
        let msg = EditDiffMessage {
            path: "src/main.rs".into(),
            old_content: "fn main() {}\n".into(),
            new_content: "fn main() {\n    println!(\"hi\");\n}\n".into(),
            tool_id: None,
        };
        let lines = edit_diff_block_lines(&msg, &palette, 80, 0, DIFF_PAGE_SIZE);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("diff")));
        assert!(
            lines
                .last()
                .unwrap()
                .spans
                .iter()
                .any(|s| s.content.contains("╰─"))
        );
    }

    #[test]
    fn large_diff_shows_omitted_message() {
        let palette = LocusPalette::locus_dark();
        let old_content = (0..25).map(|i| format!("old {i}\n")).collect::<String>();
        let new_content = (0..25).map(|i| format!("new {i}\n")).collect::<String>();
        let msg = EditDiffMessage {
            path: "src/lib.rs".into(),
            old_content,
            new_content,
            tool_id: None,
        };
        let lines = edit_diff_block_lines(&msg, &palette, 80, 0, DIFF_PAGE_SIZE);
        assert!(
            lines[1]
                .spans
                .iter()
                .any(|s| s.content.contains("diff preview omitted"))
        );
    }
}
