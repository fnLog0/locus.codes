//! Bash tool TUI rendering — summary extraction and result preview lines.
//!
//! Special: `$` prompt marker in accent color, full command in text color.
//! Preview shows stdout tail (success) or stderr head (failure), max 3 lines.

use ratatui::text::{Line, Span};

use crate::layouts::{accent_style, danger_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::LEFT_PADDING;

const PREVIEW_INDENT: &str = "    ";
const MAX_PREVIEW_LINES: usize = 3;

/// Extract a summary from bash tool args: the command (first line, not truncated in status).
/// For bash, we return the FULL command (first line if multiline) - truncation is left to terminal wrapping.
pub fn bash_summary(args: &serde_json::Value) -> Option<String> {
    let cmd = args.get("command").and_then(|v| v.as_str())?;
    let first_line = cmd.lines().next().unwrap_or(cmd);
    Some(first_line.to_string())
}

/// Build the status line spans for bash: `$ command` where `$` is accent and command is text.
/// Returns spans for: `$ ` + `command`
pub fn bash_status_summary(args: &serde_json::Value, palette: &LocusPalette) -> Vec<Span<'static>> {
    let cmd = args
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let first_line = cmd.lines().next().unwrap_or(cmd);
    vec![
        Span::styled("$ ".to_string(), accent_style(palette.accent)),
        Span::styled(first_line.to_string(), text_style(palette.text)),
    ]
}

/// Build preview lines showing stdout tail (success) or stderr head (failure).
pub fn bash_preview_lines(
    result: &serde_json::Value,
    palette: &LocusPalette,
    success: bool,
) -> Vec<Line<'static>> {
    if success {
        stdout_preview(result, palette)
    } else {
        stderr_preview(result, palette)
    }
}

fn stdout_preview(result: &serde_json::Value, palette: &LocusPalette) -> Vec<Line<'static>> {
    let stdout = result.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    if stdout.trim().is_empty() {
        return vec![];
    }

    let all_lines: Vec<&str> = stdout.lines().collect();
    let total = all_lines.len();
    let skip = total.saturating_sub(MAX_PREVIEW_LINES);
    let mut lines = Vec::new();

    if skip > 0 {
        lines.push(preview_line(
            palette,
            format!("… {} lines above", skip),
            false,
        ));
    }

    for line in all_lines.iter().skip(skip) {
        lines.push(preview_line(palette, truncate(line, 120), false));
    }

    lines
}

fn stderr_preview(result: &serde_json::Value, palette: &LocusPalette) -> Vec<Line<'static>> {
    let stderr = result
        .get("stderr")
        .and_then(|v| v.as_str())
        .or_else(|| result.get("error").and_then(|v| v.as_str()))
        .unwrap_or("");
    if stderr.trim().is_empty() {
        return vec![];
    }

    let all_lines: Vec<&str> = stderr.lines().collect();
    let show = all_lines.len().min(MAX_PREVIEW_LINES);
    let mut lines = Vec::new();

    for line in all_lines.iter().take(show) {
        lines.push(preview_line(palette, truncate(line, 120), true));
    }

    let remaining = all_lines.len().saturating_sub(show);
    if remaining > 0 {
        lines.push(preview_line(
            palette,
            format!("… {} more lines", remaining),
            true,
        ));
    }

    lines
}

fn preview_line(palette: &LocusPalette, text: String, danger: bool) -> Line<'static> {
    let style = if danger {
        danger_style(palette.danger)
    } else {
        text_muted_style(palette.text_muted)
    };
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::raw(PREVIEW_INDENT),
        Span::styled(text, style),
    ])
}

fn truncate(s: &str, max: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max {
        trimmed.to_string()
    } else {
        let t: String = trimmed.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_extracts_command() {
        let args = serde_json::json!({"command": "cargo build --release"});
        assert_eq!(bash_summary(&args), Some("cargo build --release".into()));
    }

    #[test]
    fn summary_takes_first_line_of_multiline() {
        let args = serde_json::json!({"command": "echo hello\necho world"});
        assert_eq!(bash_summary(&args), Some("echo hello".into()));
    }

    #[test]
    fn summary_none_when_no_command() {
        let args = serde_json::json!({});
        assert!(bash_summary(&args).is_none());
    }

    #[test]
    fn status_summary_has_dollar_marker() {
        let args = serde_json::json!({"command": "cargo test"});
        let palette = LocusPalette::locus_dark();
        let spans = bash_status_summary(&args, &palette);
        assert_eq!(spans.len(), 2);
        assert!(spans[0].content.contains("$"));
    }

    #[test]
    fn preview_stdout_last_lines() {
        let result = serde_json::json!({"stdout": "line1\nline2\nline3\nline4\nline5"});
        let palette = LocusPalette::locus_dark();
        let lines = bash_preview_lines(&result, &palette, true);
        // Should show "… 2 lines above" + last 3 lines = 4 lines
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn preview_stdout_short_output() {
        let result = serde_json::json!({"stdout": "ok\n"});
        let palette = LocusPalette::locus_dark();
        let lines = bash_preview_lines(&result, &palette, true);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn preview_stdout_empty() {
        let result = serde_json::json!({"stdout": ""});
        let palette = LocusPalette::locus_dark();
        let lines = bash_preview_lines(&result, &palette, true);
        assert!(lines.is_empty());
    }

    #[test]
    fn preview_stderr_on_failure() {
        let result = serde_json::json!({"stderr": "error: not found\ndetail line\n"});
        let palette = LocusPalette::locus_dark();
        let lines = bash_preview_lines(&result, &palette, false);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn preview_stderr_long() {
        let stderr = (0..10)
            .map(|i| format!("err {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = serde_json::json!({"stderr": stderr});
        let palette = LocusPalette::locus_dark();
        let lines = bash_preview_lines(&result, &palette, false);
        // 3 error lines + "… 7 more lines" = 4 lines
        assert_eq!(lines.len(), 4);
    }
}
