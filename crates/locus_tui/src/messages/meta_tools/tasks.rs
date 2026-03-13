//! task (sub-agent) meta-tool TUI rendering.
//!
//! Running: `"description"  elapsed`. Uses `╎` rail.
//! Done: `"description"  completed  duration` + result summary line.

use ratatui::text::{Line, Span};

use crate::layouts::{success_style, text_muted_style};
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

/// Build status line spans for task: `"description"`.
pub fn task_status_summary(
    args: &serde_json::Value,
    _result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let description = args
        .get("description")
        .or_else(|| args.get("task"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    vec![Span::styled(
        format!("\"{}\"", description),
        text_muted_style(palette.text_muted),
    )]
}

/// Build "completed" span for done status.
pub fn task_completed_span(palette: &LocusPalette) -> Span<'static> {
    Span::styled("completed".to_string(), success_style(palette.success))
}

/// Build preview line showing result summary.
pub fn task_preview_line(
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Option<Line<'static>> {
    let summary = result.get("summary").and_then(|v| v.as_str())?;
    if summary.is_empty() {
        return None;
    }

    Some(Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::raw("    "),
        Span::styled(summary.to_string(), text_muted_style(palette.text_muted)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_description() {
        let args = serde_json::json!({"description": "Add tests for auth module"});
        let result = serde_json::json!({});
        let palette = LocusPalette::locus_dark();
        let spans = task_status_summary(&args, &result, &palette);
        assert!(
            spans
                .iter()
                .any(|s| s.content.contains("\"Add tests for auth module\""))
        );
    }

    #[test]
    fn preview_shows_summary() {
        let result =
            serde_json::json!({"summary": "Created tests/auth_test.rs (3 test functions)"});
        let palette = LocusPalette::locus_dark();
        let line = task_preview_line(&result, &palette);
        assert!(line.is_some());
    }
}
