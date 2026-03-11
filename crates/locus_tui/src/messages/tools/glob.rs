//! glob tool TUI rendering — pattern matching.
//!
//! ≤5 matches: one line with count.
//! >5 matches: preview first 3, "+N more".

use ratatui::text::{Line, Span};

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

const PREVIEW_INDENT: &str = "    ";
const PREVIEW_THRESHOLD: usize = 5;
const MAX_PREVIEW_LINES: usize = 3;

/// Build status line spans for glob: `pattern  N matches`.
pub fn glob_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    
    let match_count = result
        .get("matches")
        .and_then(|v| v.as_array())
        .map(|m| m.len())
        .or_else(|| result.get("count").and_then(|v| v.as_u64()).map(|c| c as usize));

    let mut spans = vec![Span::styled(
        pattern.to_string(),
        text_muted_style(palette.text_muted),
    )];

    if let Some(count) = match_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} matches", count),
            text_muted_style(palette.text_muted),
        ));
    }

    spans
}

/// Build preview lines for glob when >5 matches. Max 3 preview lines + "+N more".
pub fn glob_preview_lines(
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Line<'static>> {
    let matches = result.get("matches").and_then(|v| v.as_array());
    let matches = match matches {
        Some(m) => m,
        None => return vec![],
    };

    if matches.len() <= PREVIEW_THRESHOLD {
        return vec![];
    }

    let mut lines = Vec::new();
    let muted = text_muted_style(palette.text_muted);

    // Show first 3 matches
    for m in matches.iter().take(MAX_PREVIEW_LINES) {
        let path = m.as_str().unwrap_or("");
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::raw(PREVIEW_INDENT),
            Span::styled(path.to_string(), muted),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_pattern_and_count() {
        let args = serde_json::json!({"pattern": "**/*.rs"});
        let result = serde_json::json!({"matches": ["a.rs", "b.rs", "c.rs"]});
        let palette = LocusPalette::locus_dark();
        let spans = glob_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("**/*.rs")));
        assert!(spans.iter().any(|s| s.content.contains("3 matches")));
    }

    #[test]
    fn preview_shows_when_over_threshold() {
        let matches: Vec<_> = (0..10).map(|i| format!("file{}.rs", i)).collect();
        let result = serde_json::json!({"matches": matches});
        let palette = LocusPalette::locus_dark();
        let lines = glob_preview_lines(&result, &palette);
        // 3 preview + 1 "+7 more" = 4 lines
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn no_preview_when_under_threshold() {
        let result = serde_json::json!({"matches": ["a.rs", "b.rs", "c.rs"]});
        let palette = LocusPalette::locus_dark();
        let lines = glob_preview_lines(&result, &palette);
        assert!(lines.is_empty());
    }
}
