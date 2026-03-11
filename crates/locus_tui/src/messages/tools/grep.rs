//! grep tool TUI rendering — text search.
//!
//! One line with stats: `"pattern"  scope  N in M files`.
//! Preview: max 2 matches + "+N more".

use ratatui::text::{Line, Span};

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

const PREVIEW_INDENT: &str = "    ";
const MAX_PREVIEW_LINES: usize = 2;

/// Build status line spans for grep: `"pattern"  scope  N in M files`.
pub fn grep_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let scope = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    
    // Get match and file counts
    let match_count = result
        .get("matches")
        .and_then(|v| v.as_array())
        .map(|m| m.len())
        .or_else(|| result.get("count").and_then(|v| v.as_u64()).map(|c| c as usize));

    let file_count = result.get("files").and_then(|v| v.as_u64());

    let mut spans = vec![
        Span::styled(format!("\"{}\"", pattern), text_muted_style(palette.text_muted)),
    ];

    if !scope.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(scope.to_string(), text_muted_style(palette.text_muted)));
    }

    if let Some(count) = match_count {
        spans.push(Span::raw("  "));
        if let Some(files) = file_count {
            spans.push(Span::styled(
                format!("{} in {} files", count, files),
                text_muted_style(palette.text_muted),
            ));
        } else {
            spans.push(Span::styled(
                format!("{} matches", count),
                text_muted_style(palette.text_muted),
            ));
        }
    }

    spans
}

/// Build preview lines for grep: max 2 matches + "+N more".
pub fn grep_preview_lines(
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Line<'static>> {
    let matches = result.get("matches").and_then(|v| v.as_array());
    let matches = match matches {
        Some(m) => m,
        None => return vec![],
    };

    if matches.is_empty() {
        return vec![];
    }

    let mut lines = Vec::new();
    let muted = text_muted_style(palette.text_muted);

    // Show first 2 matches
    for m in matches.iter().take(MAX_PREVIEW_LINES) {
        let file = m.get("file").and_then(|v| v.as_str()).unwrap_or("");
        let line_no = m.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
        let text = m.get("text").and_then(|v| v.as_str()).unwrap_or("");
        
        // Format: "file:line  text"
        let content = format!("{}:{}    {}", file, line_no, truncate(text, 80));
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::raw(PREVIEW_INDENT),
            Span::styled(content, muted),
        ]));
    }

    let remaining = matches.len().saturating_sub(MAX_PREVIEW_LINES);
    if remaining > 0 {
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::raw(PREVIEW_INDENT),
            Span::styled(format!("+{} more", remaining), muted),
        ]));
    }

    lines
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_pattern_and_stats() {
        let args = serde_json::json!({"pattern": "TODO:", "path": "src/"});
        let result = serde_json::json!({"matches": [{"file": "a.rs", "line": 1, "text": "// TODO"}], "files": 1});
        let palette = LocusPalette::locus_dark();
        let spans = grep_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("\"TODO:\"")));
        assert!(spans.iter().any(|s| s.content.contains("1 in 1 files")));
    }

    #[test]
    fn preview_shows_matches() {
        let result = serde_json::json!({
            "matches": [
                {"file": "a.rs", "line": 12, "text": "// TODO: fix"},
                {"file": "b.rs", "line": 45, "text": "// TODO: add tests"},
                {"file": "c.rs", "line": 8, "text": "// TODO: refactor"},
            ]
        });
        let palette = LocusPalette::locus_dark();
        let lines = grep_preview_lines(&result, &palette);
        // 2 matches + "+1 more" = 3 lines
        assert_eq!(lines.len(), 3);
    }
}
