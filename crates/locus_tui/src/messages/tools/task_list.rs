//! task_list tool TUI rendering — task management.
//!
//! One line: `action  N tasks (M active)`. Content goes to LLM.

use ratatui::text::Span;

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

/// Build status line spans for task_list: `action  N tasks (M active)`.
pub fn task_list_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
    
    let total_count = result
        .get("tasks")
        .and_then(|v| v.as_array())
        .map(|t| t.len())
        .or_else(|| result.get("total").and_then(|v| v.as_u64()).map(|c| c as usize));

    let active_count = result
        .get("active")
        .and_then(|v| v.as_u64())
        .map(|c| c as usize);

    let mut spans = vec![Span::styled(
        action.to_string(),
        text_muted_style(palette.text_muted),
    )];

    if let Some(total) = total_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} tasks", total),
            text_muted_style(palette.text_muted),
        ));

        if let Some(active) = active_count {
            if active > 0 {
                spans.push(Span::styled(
                    format!(" ({} active)", active),
                    text_muted_style(palette.text_muted),
                ));
            }
        }
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_action_and_count() {
        let args = serde_json::json!({"action": "list"});
        let result = serde_json::json!({"tasks": [1, 2, 3], "active": 1});
        let palette = LocusPalette::locus_dark();
        let spans = task_list_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("list")));
        assert!(spans.iter().any(|s| s.content.contains("3 tasks")));
        assert!(spans.iter().any(|s| s.content.contains("(1 active)")));
    }
}
