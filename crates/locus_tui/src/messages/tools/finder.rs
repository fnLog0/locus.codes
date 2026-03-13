//! finder tool TUI rendering — semantic code search.
//!
//! One line only: `"query"  N results`. No preview — results go to LLM.

use ratatui::text::Span;

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

/// Build status line spans for finder: `"query"  N results`.
pub fn finder_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

    let result_count = result
        .get("results")
        .and_then(|v| v.as_array())
        .map(|r| r.len())
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

    if let Some(count) = result_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} results", count),
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
        let args = serde_json::json!({"query": "JWT validation logic"});
        let result = serde_json::json!({"results": [1, 2, 3, 4, 5]});
        let palette = LocusPalette::locus_dark();
        let spans = finder_status_summary(&args, &result, &palette);
        assert!(
            spans
                .iter()
                .any(|s| s.content.contains("\"JWT validation logic\""))
        );
        assert!(spans.iter().any(|s| s.content.contains("5 results")));
    }
}
