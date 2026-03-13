//! tool_search meta-tool TUI rendering.
//!
//! One line: `"query"  N tools`. Uses `╎` rail.

use ratatui::text::Span;

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

/// Build status line spans for tool_search: `"query"  N tools`.
pub fn search_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

    let tool_count = result
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|t| t.len())
        .or_else(|| {
            result
                .get("count")
                .and_then(|v| v.as_u64())
                .map(|c| c as usize)
        });

    let mut spans = vec![Span::styled(
        format!("\"{}\"", query),
        text_muted_style(palette.text_muted),
    )];

    if let Some(count) = tool_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} tools", count),
            text_muted_style(palette.text_muted),
        ));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_query_and_count() {
        let args = serde_json::json!({"query": "file operations"});
        let result = serde_json::json!({"tools": [1, 2, 3, 4]});
        let palette = LocusPalette::locus_dark();
        let spans = search_status_summary(&args, &result, &palette);
        assert!(
            spans
                .iter()
                .any(|s| s.content.contains("\"file operations\""))
        );
        assert!(spans.iter().any(|s| s.content.contains("4 tools")));
    }
}
