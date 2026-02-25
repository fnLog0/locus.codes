//! Tool message UI: display tool list and tool call status.
//!
//! Types here are TUI-only (no dependency on locus_toolbus). The runtime
//! maps ToolBus tools to [ToolInfo] and tool results to [ToolCallMessage]
//! for display. Colors from [crate::theme] only.

use std::time::Duration;

use ratatui::text::{Line, Span};

use crate::layouts::{danger_style, success_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::{format_duration, LEFT_PADDING};

/// One supported tool (for list view). Map from locus_toolbus `Tool::name` / `description`.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// Status of a single tool call for display.
#[derive(Debug, Clone)]
pub enum ToolCallStatus {
    /// Tool is running.
    Running,
    /// Tool finished (duration_ms, success).
    Done { duration_ms: u64, success: bool },
    /// Tool failed with message.
    Error { message: String },
}

/// Old/new content for edit_file so the TUI can show a word-level diff.
#[derive(Debug, Clone)]
pub struct EditDiff {
    pub path: String,
    pub old_content: String,
    pub new_content: String,
}

/// Dedicated chat block for a file diff (bordered, with line numbers). Linked to tool by tool_id.
#[derive(Debug, Clone)]
pub struct EditDiffMessage {
    pub path: String,
    pub old_content: String,
    pub new_content: String,
    pub tool_id: Option<String>,
}

/// One tool invocation to show in the chat/log. Map from ToolBus execution.
#[derive(Debug, Clone)]
pub struct ToolCallMessage {
    /// Tool use ID from the LLM (for matching ToolDone events).
    pub id: Option<String>,
    pub tool_name: String,
    pub status: ToolCallStatus,
    /// Optional short summary (e.g. "edit src/main.rs").
    pub summary: Option<String>,
    /// When status is Running, millis since epoch when started (for elapsed time).
    pub started_at_ms: Option<u64>,
    /// When tool is edit_file and result included old/new content, show inline diff.
    pub edit_diff: Option<EditDiff>,
}

impl ToolCallMessage {
    pub fn running(id: impl Into<String>, tool_name: impl Into<String>, summary: Option<String>) -> Self {
        let started_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as u64);
        Self {
            id: Some(id.into()),
            tool_name: tool_name.into(),
            status: ToolCallStatus::Running,
            summary,
            started_at_ms,
            edit_diff: None,
        }
    }

    pub fn done(
        id: Option<String>,
        tool_name: impl Into<String>,
        duration_ms: u64,
        success: bool,
        summary: Option<String>,
        edit_diff: Option<EditDiff>,
    ) -> Self {
        Self {
            id,
            tool_name: tool_name.into(),
            status: ToolCallStatus::Done { duration_ms, success },
            summary,
            started_at_ms: None,
            edit_diff,
        }
    }

    pub fn error(id: Option<String>, tool_name: impl Into<String>, message: impl Into<String>, summary: Option<String>) -> Self {
        Self {
            id,
            tool_name: tool_name.into(),
            status: ToolCallStatus::Error { message: message.into() },
            summary,
            started_at_ms: None,
            edit_diff: None,
        }
    }
}

/// Extra indent when rendering inside a "Tools ▸" group.
const TOOL_GROUP_INDENT: &str = "    ";

/// Build one or more lines for a tool call. Running: one line with optional shimmer name + elapsed.
/// Done: one compact line. Error: first line tool name, second line indented error in danger.
pub fn tool_call_lines(
    msg: &ToolCallMessage,
    palette: &LocusPalette,
    running_elapsed_ms: Option<u64>,
    running_name_spans: Option<Vec<Span<'static>>>,
    in_group: bool,
) -> Vec<Line<'static>> {
    let prefix = if in_group { TOOL_GROUP_INDENT } else { LEFT_PADDING };

    let mut line1 = vec![Span::raw(prefix)];

    match &msg.status {
        ToolCallStatus::Running => {
            line1.push(Span::styled("▶ ", text_style(palette.accent)));
            if let Some(spans) = running_name_spans {
                line1.extend(spans);
            } else {
                line1.push(Span::styled(msg.tool_name.clone(), text_style(palette.text)));
            }
            if let Some(s) = &msg.summary {
                line1.push(Span::raw(" "));
                line1.push(Span::styled(s.clone(), text_muted_style(palette.text_muted)));
            } else {
                line1.push(Span::raw(" …"));
            }
            if let Some(elapsed) = running_elapsed_ms {
                line1.push(Span::raw("  "));
                line1.push(Span::styled(
                    format_duration(Duration::from_millis(elapsed)),
                    text_muted_style(palette.text_muted),
                ));
            }
            vec![Line::from(line1)]
        }
        ToolCallStatus::Done { duration_ms, success } => {
            let icon = if *success { "✓ " } else { "✗ " };
            let icon_style = if *success {
                success_style(palette.success)
            } else {
                danger_style(palette.danger)
            };
            let duration = format_duration(Duration::from_millis(*duration_ms));
            line1.push(Span::styled(icon.to_string(), icon_style));
            line1.push(Span::styled(msg.tool_name.clone(), text_muted_style(palette.text_muted)));
            if let Some(s) = &msg.summary {
                line1.push(Span::raw("  "));
                line1.push(Span::styled(s.clone(), text_style(palette.text)));
            }
            line1.push(Span::raw("  "));
            line1.push(Span::styled(duration, text_muted_style(palette.text_muted)));
            vec![Line::from(line1)]
        }
        ToolCallStatus::Error { message } => {
            line1.push(Span::styled("✗ ", danger_style(palette.danger)));
            line1.push(Span::styled(msg.tool_name.clone(), text_muted_style(palette.text_muted)));
            let first = Line::from(line1);
            let second = Line::from(vec![
                Span::raw(prefix),
                Span::raw(LEFT_PADDING),
                Span::styled(message.clone(), danger_style(palette.danger)),
            ]);
            vec![first, second]
        }
    }
}

/// Build a single [Line] for a tool call (backward compat; for Running/Done returns first line only).
pub fn tool_call_line(msg: &ToolCallMessage, palette: &LocusPalette) -> Line<'static> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64);
    let elapsed = msg
        .started_at_ms
        .and_then(|start| now_ms.map(|now| now.saturating_sub(start)));
    let lines = tool_call_lines(msg, palette, elapsed, None, false);
    lines.into_iter().next().unwrap_or_else(|| Line::from(LEFT_PADDING))
}

/// Build a single [Line] for a tool list entry (e.g. "  bash  Run shell commands").
pub fn tool_info_line(info: &ToolInfo, palette: &LocusPalette) -> Line<'static> {
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(info.name.clone(), text_style(palette.text)),
        Span::raw("  "),
        Span::styled(info.description.clone(), text_muted_style(palette.text_muted)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_running_has_icon() {
        let msg = ToolCallMessage::running("t1", "bash", Some("ls".into()));
        let palette = LocusPalette::locus_dark();
        let line = tool_call_line(&msg, &palette);
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn tool_info_line_builds() {
        let info = ToolInfo {
            name: "edit_file".into(),
            description: "Edit a file.".into(),
        };
        let palette = LocusPalette::locus_dark();
        let line = tool_info_line(&info, &palette);
        assert_eq!(line.spans.len(), 4);
    }

    #[test]
    fn tool_call_done_success() {
        let msg = ToolCallMessage::done(Some("t1".into()), "edit_file", 150, true, Some("src/main.rs".into()), None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, false);
        assert!(!lines.is_empty());
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✓")));
    }

    #[test]
    fn tool_call_done_failure() {
        let msg = ToolCallMessage::done(None, "bash", 300, false, None, None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, false);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✗")));
    }

    #[test]
    fn tool_call_error_two_lines() {
        let msg = ToolCallMessage::error(None, "grep", "file not found", None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, false);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn tool_call_running_with_elapsed() {
        let msg = ToolCallMessage::running("t2", "bash", Some("ls".into()));
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, Some(1234), None, false);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("1s")));
    }

    #[test]
    fn tool_call_grouped_indent() {
        let msg = ToolCallMessage::running("t3", "bash", None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, true);
        // Grouped tools use 4-space indent instead of 2-space
        assert!(lines[0].spans[0].content.starts_with("    "));
    }
}
