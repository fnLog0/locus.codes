//! edit_file tool TUI rendering — status line with diff block.
//!
//! Status line: path + duration (compact, stats in diff header).
//! Diff block: bordered with line numbers, stats (+N -M), range.

use ratatui::text::{Line, Span};

use crate::diff::line_diff_with_numbers;
use crate::layouts::{accent_style, danger_style, success_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

const DIFF_HEADER_PREFIX: &str = "╭─ ";
const DIFF_LEFT_BORDER: &str = "│ ";
const DIFF_FOOTER_PREFIX: &str = "╰─ ";

/// Build status line spans for edit_file: just `path` (stats go in diff header).
pub fn edit_file_status_summary(
    args: &serde_json::Value,
    _result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
    vec![Span::styled(path.to_string(), text_muted_style(palette.text_muted))]
}

/// Build diff block lines with stats in header: `╭─ diff  path  +N -M  Lstart-Lend`.
pub fn edit_file_diff_lines(
    path: &str,
    old_content: &str,
    new_content: &str,
    palette: &LocusPalette,
    width: usize,
    max_lines: usize,
) -> Vec<Line<'static>> {
    let rows = line_diff_with_numbers(old_content, new_content);
    
    if rows.is_empty() {
        return vec![
            diff_header_line(path, 0, 0, None, None, palette, width),
            diff_footer_line(palette),
        ];
    }

    // Count additions and removals
    let additions = rows.iter().filter(|r| matches!(r.change, crate::diff::ChangeType::Added)).count();
    let removals = rows.iter().filter(|r| matches!(r.change, crate::diff::ChangeType::Removed)).count();
    
    // Find line range
    let first_old = rows.iter().find_map(|r| r.old_line_no);
    let last_old = rows.iter().rev().find_map(|r| r.old_line_no.or(r.new_line_no));
    let first_new = rows.iter().find_map(|r| r.new_line_no);
    let last_new = rows.iter().rev().find_map(|r| r.new_line_no.or(r.old_line_no));
    
    let range = match (first_old.or(first_new), last_old.or(last_new)) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    };

    let mut lines = Vec::new();
    lines.push(diff_header_line(path, additions, removals, range, None, palette, width));

    let content_width = width
        .saturating_sub(LEFT_PADDING.len() + DIFF_LEFT_BORDER.len() + 12)
        .max(20);

    let show_rows = rows.len().min(max_lines);
    for row in rows.iter().take(show_rows) {
        lines.push(diff_content_line(row, palette, content_width));
    }

    let remaining = rows.len().saturating_sub(max_lines);
    if remaining > 0 {
        lines.push(diff_more_line(remaining, palette));
    }

    lines.push(diff_footer_line(palette));
    lines
}

/// Build diff header line: `╭─ diff  path  +N -M  Lstart-Lend`
fn diff_header_line(
    path: &str,
    additions: usize,
    removals: usize,
    range: Option<(u32, u32)>,
    _total: Option<usize>,
    palette: &LocusPalette,
    width: usize,
) -> Line<'static> {
    let max_path = width.saturating_sub(40).max(10);
    let truncated_path = if path.chars().count() > max_path {
        let mut s: String = path.chars().take(max_path.saturating_sub(1)).collect();
        s.push('…');
        s
    } else {
        path.to_string()
    };

    let mut spans = vec![
        Span::raw(LEFT_PADDING),
        Span::styled(DIFF_HEADER_PREFIX.to_string(), text_muted_style(palette.text_muted)),
        Span::styled("diff".to_string(), accent_style(palette.accent)),
        Span::raw("  "),
        Span::styled(truncated_path, text_style(palette.text)),
    ];

    // Stats
    if additions > 0 || removals > 0 {
        spans.push(Span::raw("  "));
        if additions > 0 {
            spans.push(Span::styled(
                format!("+{}", additions),
                success_style(palette.success),
            ));
        }
        if removals > 0 {
            if additions > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!("-{}", removals),
                danger_style(palette.danger),
            ));
        }
    }

    // Range
    if let Some((start, end)) = range {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("L{}-L{}", start, end),
            text_muted_style(palette.text_muted),
        ));
    }

    Line::from(spans)
}

/// Build diff content line: `│  line_num  marker  content`
fn diff_content_line(
    row: &crate::diff::LineDiffRow,
    palette: &LocusPalette,
    content_width: usize,
) -> Line<'static> {
    let (marker, style) = match row.change {
        crate::diff::ChangeType::Added => ("+ ", success_style(palette.success)),
        crate::diff::ChangeType::Removed => ("- ", danger_style(palette.danger)),
        crate::diff::ChangeType::Unchanged => ("  ", text_style(palette.text)),
    };

    let line_num = row.new_line_no.or(row.old_line_no).unwrap_or(0);
    let content = if row.text.chars().count() > content_width {
        let mut s: String = row.text.chars().take(content_width.saturating_sub(1)).collect();
        s.push('…');
        s
    } else {
        row.text.clone()
    };

    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(DIFF_LEFT_BORDER.to_string(), text_muted_style(palette.text_muted)),
        Span::styled(format!("{:>4}  ", line_num), text_muted_style(palette.text_muted)),
        Span::styled(marker.to_string(), style),
        Span::styled(content, style),
    ])
}

/// Build "… N more lines" line
fn diff_more_line(count: usize, palette: &LocusPalette) -> Line<'static> {
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(DIFF_LEFT_BORDER.to_string(), text_muted_style(palette.text_muted)),
        Span::styled(
            format!("… {} more lines", count),
            text_muted_style(palette.text_muted),
        ),
    ])
}

/// Build diff footer line: `╰─`
fn diff_footer_line(palette: &LocusPalette) -> Line<'static> {
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(
            DIFF_FOOTER_PREFIX.to_string(),
            text_muted_style(palette.text_muted),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_path() {
        let args = serde_json::json!({"file_path": "src/main.rs"});
        let result = serde_json::json!({});
        let palette = LocusPalette::locus_dark();
        let spans = edit_file_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("src/main.rs")));
    }

    #[test]
    fn diff_lines_have_header_and_footer() {
        let palette = LocusPalette::locus_dark();
        let lines = edit_file_diff_lines(
            "src/main.rs",
            "old line\n",
            "new line\n",
            &palette,
            80,
            12,
        );
        assert!(lines.first().unwrap().spans.iter().any(|s| s.content.contains("diff")));
        assert!(lines.last().unwrap().spans.iter().any(|s| s.content.contains("╰─")));
    }
}
