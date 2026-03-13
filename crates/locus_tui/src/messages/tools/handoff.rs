//! handoff tool TUI rendering — sub-agent handoff.
//!
//! Running: truncated goal + elapsed.
//! Done: goal + duration + optional result summary line.

use ratatui::text::{Line, Span};

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

const PREVIEW_INDENT: &str = "    ";
const MAX_GOAL_LEN: usize = 40;

/// Build status line spans for handoff: truncated goal.
pub fn handoff_status_summary(
    args: &serde_json::Value,
    _result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let goal = args.get("goal").and_then(|v| v.as_str()).unwrap_or("");
    let truncated = truncate_goal(goal);
    vec![Span::styled(
        truncated,
        text_muted_style(palette.text_muted),
    )]
}

/// Build preview line showing result summary (for completed handoff).
pub fn handoff_preview_line(
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Option<Line<'static>> {
    let summary = result.get("summary").and_then(|v| v.as_str())?;
    if summary.is_empty() {
        return None;
    }

    Some(Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::raw(PREVIEW_INDENT),
        Span::styled(summary.to_string(), text_muted_style(palette.text_muted)),
    ]))
}

fn truncate_goal(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= MAX_GOAL_LEN {
        trimmed.to_string()
    } else {
        let t: String = trimmed
            .chars()
            .take(MAX_GOAL_LEN.saturating_sub(1))
            .collect();
        format!("{}…", t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_truncated_goal() {
        let args = serde_json::json!({"goal": "Fix auth middleware and add tests"});
        let result = serde_json::json!({});
        let palette = LocusPalette::locus_dark();
        let spans = handoff_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("Fix auth")));
    }

    #[test]
    fn preview_shows_summary() {
        let result = serde_json::json!({"summary": "Modified 3 files, added 2 tests"});
        let palette = LocusPalette::locus_dark();
        let line = handoff_preview_line(&result, &palette);
        assert!(line.is_some());
        let line = line.unwrap();
        assert!(
            line.spans
                .iter()
                .any(|s| s.content.contains("Modified 3 files"))
        );
    }
}
