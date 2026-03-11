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
const TOOL_LEFT_BORDER: &str = "┊ ";
const TOOL_DETAIL_INDENT: &str = "    ";
const TOOL_NAME_WIDTH: usize = 12;
const TOOL_RUNNING_INDICATOR: &str = "⠋";
const TOOL_SUCCESS_INDICATOR: &str = "✓";
const TOOL_FAILURE_INDICATOR: &str = "✕";

fn push_tool_name(
    spans: &mut Vec<Span<'static>>,
    msg: &ToolCallMessage,
    palette: &LocusPalette,
    running_name_spans: Option<Vec<Span<'static>>>,
) {
    if let Some(name_spans) = running_name_spans {
        spans.extend(name_spans);
        let pad = TOOL_NAME_WIDTH.saturating_sub(msg.tool_name.chars().count());
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
    } else {
        spans.push(Span::styled(
            format!("{:<width$}", msg.tool_name, width = TOOL_NAME_WIDTH),
            text_style(palette.text),
        ));
    }
}

fn tool_detail_line(prefix: &str, palette: &LocusPalette, text: impl Into<String>, danger: bool) -> Line<'static> {
    let detail_style = if danger {
        danger_style(palette.danger)
    } else {
        text_muted_style(palette.text_muted)
    };
    Line::from(vec![
        Span::raw(prefix.to_string()),
        Span::styled(TOOL_LEFT_BORDER.to_string(), text_muted_style(palette.text_muted)),
        Span::raw(TOOL_DETAIL_INDENT),
        Span::styled(text.into(), detail_style),
    ])
}

pub fn tool_group_header_line(tools: &[ToolCallMessage], palette: &LocusPalette) -> Line<'static> {
    let running_count = tools
        .iter()
        .filter(|t| matches!(t.status, ToolCallStatus::Running))
        .count();
    let failed_count = tools
        .iter()
        .filter(|t| {
            matches!(
                t.status,
                ToolCallStatus::Done { success: false, .. } | ToolCallStatus::Error { .. }
            )
        })
        .count();
    let all_done = running_count == 0;
    let total_ms: u64 = tools
        .iter()
        .map(|t| match &t.status {
            ToolCallStatus::Done { duration_ms, .. } => *duration_ms,
            _ => 0,
        })
        .max()
        .unwrap_or(0);

    let (rail_style, status_text, status_style) = if running_count > 0 {
        (
            text_style(palette.accent),
            format!("{} running", running_count),
            text_style(palette.accent),
        )
    } else if failed_count > 0 {
        (
            danger_style(palette.danger),
            format!("{} failed", failed_count),
            danger_style(palette.danger),
        )
    } else {
        (
            text_muted_style(palette.text_muted),
            "complete".to_string(),
            success_style(palette.success),
        )
    };

    let mut spans = vec![
        Span::raw(LEFT_PADDING),
        Span::styled("╭─ ".to_string(), rail_style),
        Span::styled("tools".to_string(), text_style(palette.text)),
        Span::raw("  "),
        Span::styled(format!("{}", tools.len()), text_muted_style(palette.text_muted)),
        Span::raw("  "),
        Span::styled(status_text, status_style),
    ];

    if all_done && total_ms > 0 {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format_duration(Duration::from_millis(total_ms)),
            text_muted_style(palette.text_muted),
        ));
    }

    Line::from(spans)
}

/// Build one or more lines for a tool call. Running: one line with optional shimmer name + elapsed.
/// Done: one compact line. Error: first line tool name, second line indented error in danger.
pub fn tool_call_lines(
    msg: &ToolCallMessage,
    palette: &LocusPalette,
    running_elapsed_ms: Option<u64>,
    running_name_spans: Option<Vec<Span<'static>>>,
    running_indicator: Option<&'static str>,
    in_group: bool,
) -> Vec<Line<'static>> {
    let prefix = if in_group { TOOL_GROUP_INDENT } else { LEFT_PADDING };

    let mut line1 = vec![
        Span::raw(prefix.to_string()),
        Span::styled(TOOL_LEFT_BORDER.to_string(), text_muted_style(palette.text_muted)),
    ];

    match &msg.status {
        ToolCallStatus::Running => {
            line1[1] = Span::styled(TOOL_LEFT_BORDER.to_string(), text_style(palette.accent));
            line1.push(Span::styled(
                format!("{} ", running_indicator.unwrap_or(TOOL_RUNNING_INDICATOR)),
                text_style(palette.accent),
            ));
            push_tool_name(&mut line1, msg, palette, running_name_spans);
            if let Some(s) = &msg.summary {
                line1.push(Span::raw("  "));
                line1.push(Span::styled(s.clone(), text_muted_style(palette.text_muted)));
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
            let status_style = if *success {
                success_style(palette.success)
            } else {
                danger_style(palette.danger)
            };
            line1[1] = Span::styled(TOOL_LEFT_BORDER.to_string(), status_style);
            let duration = format_duration(Duration::from_millis(*duration_ms));
            line1.push(Span::styled(
                format!(
                    "{} ",
                    if *success {
                        TOOL_SUCCESS_INDICATOR
                    } else {
                        TOOL_FAILURE_INDICATOR
                    }
                ),
                status_style,
            ));
            push_tool_name(&mut line1, msg, palette, None);
            if let Some(s) = &msg.summary {
                line1.push(Span::raw("  "));
                line1.push(Span::styled(s.clone(), text_muted_style(palette.text_muted)));
            }
            if !*success {
                line1.push(Span::raw("  "));
                line1.push(Span::styled("failed".to_string(), status_style));
            }
            line1.push(Span::raw("  "));
            line1.push(Span::styled(duration, text_muted_style(palette.text_muted)));
            vec![Line::from(line1)]
        }
        ToolCallStatus::Error { message } => {
            line1[1] = Span::styled(TOOL_LEFT_BORDER.to_string(), danger_style(palette.danger));
            push_tool_name(&mut line1, msg, palette, None);
            line1.push(Span::raw("  "));
            line1.push(Span::styled("error".to_string(), danger_style(palette.danger)));
            let mut lines = vec![Line::from(line1)];
            if let Some(summary) = &msg.summary {
                lines.push(tool_detail_line(prefix, palette, summary.clone(), false));
            }
            lines.push(tool_detail_line(prefix, palette, message.clone(), true));
            lines
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
    let lines = tool_call_lines(msg, palette, elapsed, None, Some(TOOL_RUNNING_INDICATOR), false);
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
        let lines = tool_call_lines(&msg, &palette, None, None, None, false);
        assert!(!lines.is_empty());
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✓")));
        assert!(!lines[0].spans.iter().any(|s| s.content.contains("done")));
    }

    #[test]
    fn tool_call_done_failure() {
        let msg = ToolCallMessage::done(None, "bash", 300, false, None, None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, None, false);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("failed")));
    }

    #[test]
    fn tool_call_error_two_lines() {
        let msg = ToolCallMessage::error(None, "grep", "file not found", None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, None, false);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("error")));
    }

    #[test]
    fn tool_call_running_with_elapsed() {
        let msg = ToolCallMessage::running("t2", "bash", Some("ls".into()));
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, Some(1234), None, Some("⠋"), false);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("1s")));
    }

    #[test]
    fn tool_call_grouped_indent() {
        let msg = ToolCallMessage::running("t3", "bash", None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, Some("⠋"), true);
        // Grouped tools use 4-space indent instead of 2-space
        assert!(lines[0].spans[0].content.starts_with("    "));
    }

    #[test]
    fn tool_group_header_reports_running_state() {
        let palette = LocusPalette::locus_dark();
        let tools = vec![
            ToolCallMessage::running("t1", "bash", None),
            ToolCallMessage::done(Some("t2".into()), "grep", 20, true, None, None),
        ];
        let line = tool_group_header_line(&tools, &palette);
        assert!(line.spans.iter().any(|s| s.content.contains("tools")));
        assert!(line.spans.iter().any(|s| s.content.contains("1 running")));
    }
}
