//! tool_explain meta-tool TUI rendering.
//!
//! One line + optional description: `tool_name  N params`.
//! Preview line: short description.

use ratatui::text::{Line, Span};

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

/// Build status line spans for tool_explain: `tool_name  N params`.
pub fn explain_status_summary(
    args: &serde_json::Value,
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let tool_name = args.get("tool").and_then(|v| v.as_str()).unwrap_or("");

    let param_count = result
        .get("parameters")
        .and_then(|v| v.as_array())
        .map(|p| p.len())
        .or_else(|| {
            result
                .get("param_count")
                .and_then(|v| v.as_u64())
                .map(|c| c as usize)
        });

    let mut spans = vec![Span::styled(
        tool_name.to_string(),
        text_muted_style(palette.text_muted),
    )];

    if let Some(count) = param_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{} params", count),
            text_muted_style(palette.text_muted),
        ));
    }

    spans
}

/// Build preview line showing short description.
pub fn explain_preview_line(
    result: &serde_json::Value,
    palette: &LocusPalette,
) -> Option<Line<'static>> {
    let description = result.get("description").and_then(|v| v.as_str())?;
    if description.is_empty() {
        return None;
    }

    // Truncate description
    let truncated = if description.chars().count() > 80 {
        let t: String = description.chars().take(79).collect();
        format!("{}…", t)
    } else {
        description.to_string()
    };

    Some(Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::raw("    "),
        Span::styled(truncated, text_muted_style(palette.text_muted)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_tool_and_params() {
        let args = serde_json::json!({"tool": "edit_file"});
        let result = serde_json::json!({"parameters": [1, 2, 3, 4, 5]});
        let palette = LocusPalette::locus_dark();
        let spans = explain_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("edit_file")));
        assert!(spans.iter().any(|s| s.content.contains("5 params")));
    }

    #[test]
    fn preview_shows_description() {
        let result = serde_json::json!({"description": "Make edits to a text file. Replaces old_str with new_str."});
        let palette = LocusPalette::locus_dark();
        let line = explain_preview_line(&result, &palette);
        assert!(line.is_some());
    }
}
