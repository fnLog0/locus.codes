//! web_automation tool TUI rendering — web browsing and search.
//!
//! URL fetch: `domain/path  duration`. Protocol stripped.
//! Web search: `"query"  N results  duration`.
//! One line each — content goes to LLM.

use ratatui::text::Span;

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

/// Build status line spans for web fetch: `domain/path` (protocol stripped).
pub fn web_fetch_status_summary(
    args: &serde_json::Value,
    _result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
    
    // Strip protocol
    let display_url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    vec![Span::styled(
        display_url.to_string(),
        text_muted_style(palette.text_muted),
    )]
}

/// Build status line spans for web search: `"query"  N results`.
pub fn web_search_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    
    let result_count = result
        .get("results")
        .and_then(|v| v.as_array())
        .map(|r| r.len())
        .or_else(|| result.get("count").and_then(|v| v.as_u64()).map(|c| c as usize));

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
    fn fetch_strips_protocol() {
        let args = serde_json::json!({"url": "https://docs.rs/tokio"});
        let result = serde_json::json!({});
        let palette = LocusPalette::locus_dark();
        let spans = web_fetch_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("docs.rs/tokio")));
        assert!(!spans.iter().any(|s| s.content.contains("https://")));
    }

    #[test]
    fn search_shows_query_and_count() {
        let args = serde_json::json!({"query": "rust async channels"});
        let result = serde_json::json!({"results": [1, 2, 3, 4, 5]});
        let palette = LocusPalette::locus_dark();
        let spans = web_search_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("\"rust async channels\"")));
        assert!(spans.iter().any(|s| s.content.contains("5 results")));
    }
}
