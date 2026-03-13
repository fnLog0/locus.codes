//! read tool TUI rendering — one line, no preview.
//!
//! Shows: path + range/entries + duration. Content goes to LLM, not user.

use ratatui::text::Span;

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

/// Build status line spans for read (file): `path  [start-end]  N lines`.
pub fn read_file_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");

    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let limit = args.get("limit").and_then(|v| v.as_u64());

    // Try to get line count from result
    let line_count = result.get("lines").and_then(|v| v.as_u64()).or_else(|| {
        result
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.lines().count() as u64)
    });

    let mut spans = vec![Span::styled(
        path.to_string(),
        text_muted_style(palette.text_muted),
    )];

    // Range [offset-offset+limit] or [offset-end]
    if offset > 0 || limit.is_some() {
        let end = limit
            .map(|l| offset + l)
            .or(line_count)
            .unwrap_or(offset + 1);
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("[{}-{}]", offset, end),
            text_muted_style(palette.text_muted),
        ));
    }

    if let Some(count) = line_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} lines", count),
            text_muted_style(palette.text_muted),
        ));
    }

    spans
}

/// Build status line spans for read (directory): `path/  N entries`.
pub fn read_dir_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");

    let entry_count = result
        .get("entries")
        .and_then(|v| v.as_array())
        .map(|e| e.len() as u64);

    let mut spans = vec![Span::styled(
        format!("{}/", path.trim_end_matches('/')),
        text_muted_style(palette.text_muted),
    )];

    if let Some(count) = entry_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} entries", count),
            text_muted_style(palette.text_muted),
        ));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_status_shows_path_and_range() {
        let args = serde_json::json!({"file_path": "src/lib.rs", "offset": 0, "limit": 50});
        let result = serde_json::json!({"content": "line1\nline2\n"});
        let palette = LocusPalette::locus_dark();
        let spans = read_file_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("src/lib.rs")));
        assert!(spans.iter().any(|s| s.content.contains("[0-50]")));
    }

    #[test]
    fn dir_status_shows_entries() {
        let args = serde_json::json!({"path": "crates"});
        let result = serde_json::json!({"entries": ["a", "b", "c"]});
        let palette = LocusPalette::locus_dark();
        let spans = read_dir_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("crates/")));
        assert!(spans.iter().any(|s| s.content.contains("3 entries")));
    }
}
