//! undo_edit tool TUI rendering — one line, minimal.
//!
//! Shows: path + "restored" + duration.

use ratatui::text::Span;

use crate::layouts::text_muted_style;
use crate::theme::LocusPalette;

/// Build status line spans for undo_edit: `path  restored`.
pub fn undo_edit_status_summary(
    args: &serde_json::Value,
    _result: &serde_json::Value,
    palette: &LocusPalette,
) -> Vec<Span<'static>> {
    let path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
    vec![
        Span::styled(path.to_string(), text_muted_style(palette.text_muted)),
        Span::raw("  "),
        Span::styled("restored".to_string(), text_muted_style(palette.text_muted)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shows_path_and_restored() {
        let args = serde_json::json!({"file_path": "src/main.rs"});
        let result = serde_json::json!({});
        let palette = LocusPalette::locus_dark();
        let spans = undo_edit_status_summary(&args, &result, &palette);
        assert!(spans.iter().any(|s| s.content.contains("src/main.rs")));
        assert!(spans.iter().any(|s| s.content.contains("restored")));
    }
}
