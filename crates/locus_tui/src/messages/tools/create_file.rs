//! create_file tool TUI rendering — one line, no content preview.
//!
//! Shows: path, line count, duration. No preview because user knows what they created.

use ratatui::text::Span;

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

/// Build status line spans for create_file: `path  N lines  duration`.
pub fn create_file_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
    
    // Try to get line count from result or content
    let line_count = result
        .get("lines")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            args.get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.lines().count() as u64)
        });

    let mut spans = vec![Span::styled(path.to_string(), text_muted_style(palette.text_muted))];
    
    if let Some(count) = line_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} lines", count),
            text_muted_style(palette.text_muted),
        ));
    }
    
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_path_and_lines() {
        let args = serde_json::json!({"file_path": "src/new.rs", "content": "fn main() {}\n"});
        let result = serde_json::json!({});
        let palette = LocusPalette::locus_dark();
        let spans = create_file_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("src/new.rs")));
        assert!(spans.iter().any(|s| s.content.contains("1 lines")));
    }
}
