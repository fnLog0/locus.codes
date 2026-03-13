//! Tool message UI: display tool list and tool call status.
//!
//! Types here are TUI-only (no dependency on locus_toolbus). The runtime
//! maps ToolBus tools to [ToolInfo] and tool results to [ToolCallMessage]
//! for display. Colors from [crate::theme] only.
//!
//! Per-tool rendering modules handle summary extraction and preview lines.

mod bash;
mod create_file;
mod edit_file;
mod finder;
mod glob;
mod grep;
mod handoff;
mod read;
mod task_list;
mod undo_edit;
mod web_automation;

use std::time::Duration;

use ratatui::{
    style::Modifier,
    text::{Line, Span},
};

use crate::layouts::{danger_style, success_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::{LEFT_PADDING, format_duration};

// Re-export types
pub use bash::{bash_preview_lines, bash_status_summary, bash_summary};
pub use create_file::create_file_status_summary;
pub use edit_file::{edit_file_diff_lines, edit_file_status_summary};
pub use finder::finder_status_summary;
pub use glob::{glob_preview_lines, glob_status_summary};
pub use grep::{grep_preview_lines, grep_status_summary};
pub use handoff::{handoff_preview_line, handoff_status_summary};
pub use read::{read_dir_status_summary, read_file_status_summary};
pub use task_list::task_list_status_summary;
pub use undo_edit::undo_edit_status_summary;
pub use web_automation::{web_fetch_status_summary, web_search_status_summary};

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
    /// Optional args JSON for per-tool rendering.
    pub args: Option<serde_json::Value>,
    /// Optional result JSON for per-tool rendering.
    pub result: Option<serde_json::Value>,
}

impl ToolCallMessage {
    pub fn running(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        summary: Option<String>,
    ) -> Self {
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
            args: None,
            result: None,
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
            status: ToolCallStatus::Done {
                duration_ms,
                success,
            },
            summary,
            started_at_ms: None,
            edit_diff,
            args: None,
            result: None,
        }
    }

    pub fn error(
        id: Option<String>,
        tool_name: impl Into<String>,
        message: impl Into<String>,
        summary: Option<String>,
    ) -> Self {
        Self {
            id,
            tool_name: tool_name.into(),
            status: ToolCallStatus::Error {
                message: message.into(),
            },
            summary,
            started_at_ms: None,
            edit_diff: None,
            args: None,
            result: None,
        }
    }
}

/// Extra indent when rendering inside a "Tools" group.
const TOOL_DETAIL_INDENT: &str = "    ";
const TOOL_NAME_WIDTH: usize = 12;
const TOOL_RUNNING_INDICATOR: &str = "⠋";
const TOOL_SUCCESS_INDICATOR: &str = "✓";
const TOOL_FAILURE_INDICATOR: &str = "✗";

/// Format tool name for display: edit_file → Edit, create_file → Create, etc.
fn format_tool_name(name: &str) -> String {
    match name {
        "edit_file" => "Edit",
        "create_file" => "Create",
        "undo_edit" => "Undo",
        "bash" => "Bash",
        "read" | "view" => "Read",
        "glob" => "Glob",
        "grep" => "Grep",
        "finder" => "Finder",
        "handoff" => "Handoff",
        "task_list" => "Tasks",
        "web_fetch" | "fetch" => "Fetch",
        "web_search" => "Search",
        _ => name,
    }
    .to_string()
}

fn push_tool_name(
    spans: &mut Vec<Span<'static>>,
    msg: &ToolCallMessage,
    palette: &LocusPalette,
    running_name_spans: Option<Vec<Span<'static>>>,
) {
    let display_name = format_tool_name(&msg.tool_name);
    if let Some(name_spans) = running_name_spans {
        spans.extend(name_spans);
        let pad = TOOL_NAME_WIDTH.saturating_sub(display_name.chars().count());
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
    } else {
        let name_style = text_style(palette.text).add_modifier(Modifier::BOLD);
        spans.push(Span::styled(
            format!("{:<width$}", display_name, width = TOOL_NAME_WIDTH),
            name_style,
        ));
    }
}

fn tool_detail_line(
    prefix: &str,
    palette: &LocusPalette,
    text: impl Into<String>,
    danger: bool,
) -> Line<'static> {
    let detail_style = if danger {
        danger_style(palette.danger)
    } else {
        text_muted_style(palette.text_muted)
    };
    Line::from(vec![
        Span::raw(prefix.to_string()),
        Span::raw(TOOL_DETAIL_INDENT),
        Span::styled(text.into(), detail_style),
    ])
}

pub fn tool_group_header_line(
    tools: &[ToolCallMessage],
    palette: &LocusPalette,
    running_indicator: Option<&'static str>,
) -> Line<'static> {
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

    let (status_icon, status_style) = if running_count > 0 {
        (
            running_indicator.unwrap_or(TOOL_RUNNING_INDICATOR),
            text_style(palette.accent),
        )
    } else if failed_count > 0 {
        (TOOL_FAILURE_INDICATOR, danger_style(palette.danger))
    } else {
        (TOOL_SUCCESS_INDICATOR, success_style(palette.success))
    };

    let mut spans = vec![
        Span::raw(LEFT_PADDING),
        Span::styled(
            "Tools".to_string(),
            text_style(palette.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(format!("{} ", status_icon), status_style),
        Span::styled(
            format!("{}", tools.len()),
            text_muted_style(palette.text_muted),
        ),
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
    is_last: bool,
) -> Vec<Line<'static>> {
    let tree_connector = if in_group {
        if is_last { "└── " } else { "├── " }
    } else {
        ""
    };

    let mut line1 = vec![Span::raw(LEFT_PADDING.to_string())];
    if in_group {
        line1.push(Span::styled(
            tree_connector.to_string(),
            text_muted_style(palette.text_muted),
        ));
    }

    match &msg.status {
        ToolCallStatus::Running => {
            line1.push(Span::styled(
                format!("{} ", running_indicator.unwrap_or(TOOL_RUNNING_INDICATOR)),
                text_style(palette.accent),
            ));
            push_tool_name(&mut line1, msg, palette, running_name_spans);

            // Use per-tool status summary if available
            let summary_spans = get_tool_status_spans(msg, palette);
            if !summary_spans.is_empty() {
                line1.push(Span::raw("  "));
                line1.extend(summary_spans);
            } else if let Some(s) = &msg.summary {
                line1.push(Span::raw("  "));
                line1.push(Span::styled(
                    s.clone(),
                    text_muted_style(palette.text_muted),
                ));
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
        ToolCallStatus::Done {
            duration_ms,
            success,
        } => {
            let status_style = if *success {
                success_style(palette.success)
            } else {
                danger_style(palette.danger)
            };
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

            // Use per-tool status summary if available
            let summary_spans = get_tool_status_spans(msg, palette);
            if !summary_spans.is_empty() {
                line1.push(Span::raw("  "));
                line1.extend(summary_spans);
            } else if let Some(s) = &msg.summary {
                line1.push(Span::raw("  "));
                line1.push(Span::styled(
                    s.clone(),
                    text_muted_style(palette.text_muted),
                ));
            }

            if !*success {
                line1.push(Span::raw("  "));
                line1.push(Span::styled("failed".to_string(), status_style));
            }
            line1.push(Span::raw("  "));
            line1.push(Span::styled(duration, text_muted_style(palette.text_muted)));

            let mut lines = vec![Line::from(line1)];

            // Add preview lines for tools that have them
            let preview = get_tool_preview_lines(msg, palette, *success);
            lines.extend(preview);

            lines
        }
        ToolCallStatus::Error { message } => {
            line1.push(Span::styled("✗ ".to_string(), danger_style(palette.danger)));
            push_tool_name(&mut line1, msg, palette, None);
            let mut lines = vec![Line::from(line1)];
            let error_prefix = if in_group {
                format!("{}    ", LEFT_PADDING)
            } else {
                LEFT_PADDING.to_string()
            };
            if let Some(summary) = &msg.summary {
                lines.push(tool_detail_line(
                    &error_prefix,
                    palette,
                    summary.clone(),
                    false,
                ));
            }
            lines.push(tool_detail_line(
                &error_prefix,
                palette,
                message.clone(),
                true,
            ));
            lines
        }
    }
}

/// Get per-tool status summary spans based on tool name and args/result.
fn get_tool_status_spans(msg: &ToolCallMessage, palette: &LocusPalette) -> Vec<Span<'static>> {
    let args = match &msg.args {
        Some(a) => a,
        None => return vec![],
    };
    let result = msg.result.as_ref().unwrap_or(&serde_json::Value::Null);

    match msg.tool_name.as_str() {
        "bash" => bash_status_summary(args, palette),
        "create_file" => create_file_status_summary(args, result, palette),
        "edit_file" => edit_file_status_summary(args, result, palette),
        "undo_edit" => undo_edit_status_summary(args, result, palette),
        "read" | "view" => {
            // Try file first, then dir
            if args.get("file_path").is_some() {
                read_file_status_summary(args, result, palette)
            } else {
                read_dir_status_summary(args, result, palette)
            }
        }
        "glob" => glob_status_summary(args, result, palette),
        "grep" => grep_status_summary(args, result, palette),
        "finder" => finder_status_summary(args, result, palette),
        "handoff" => handoff_status_summary(args, result, palette),
        "task_list" => task_list_status_summary(args, result, palette),
        "web_fetch" | "fetch" => web_fetch_status_summary(args, result, palette),
        "web_search" => web_search_status_summary(args, result, palette),
        _ => vec![],
    }
}

/// Get per-tool preview lines based on tool name and result.
fn get_tool_preview_lines(
    msg: &ToolCallMessage,
    palette: &LocusPalette,
    success: bool,
) -> Vec<Line<'static>> {
    let result = match &msg.result {
        Some(r) => r,
        None => return vec![],
    };

    match msg.tool_name.as_str() {
        "bash" => bash_preview_lines(result, palette, success),
        "glob" => glob_preview_lines(result, palette),
        "grep" => grep_preview_lines(result, palette),
        "handoff" => handoff_preview_line(result, palette).into_iter().collect(),
        _ => vec![],
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
    let lines = tool_call_lines(
        msg,
        palette,
        elapsed,
        None,
        Some(TOOL_RUNNING_INDICATOR),
        false,
        false,
    );
    lines
        .into_iter()
        .next()
        .unwrap_or_else(|| Line::from(LEFT_PADDING))
}

/// Build a single [Line] for a tool list entry (e.g. "  Bash  Run shell commands").
pub fn tool_info_line(info: &ToolInfo, palette: &LocusPalette) -> Line<'static> {
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(format_tool_name(&info.name), text_style(palette.text)),
        Span::raw("  "),
        Span::styled(
            info.description.clone(),
            text_muted_style(palette.text_muted),
        ),
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
        let msg = ToolCallMessage::done(
            Some("t1".into()),
            "edit_file",
            150,
            true,
            Some("src/main.rs".into()),
            None,
        );
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, None, false, false);
        assert!(!lines.is_empty());
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✓")));
        assert!(!lines[0].spans.iter().any(|s| s.content.contains("done")));
    }

    #[test]
    fn tool_call_done_failure() {
        let msg = ToolCallMessage::done(None, "bash", 300, false, None, None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, None, false, false);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("failed")));
    }

    #[test]
    fn tool_call_error_two_lines() {
        let msg = ToolCallMessage::error(None, "grep", "file not found", None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, None, false, false);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✗")));
    }

    #[test]
    fn tool_call_running_with_elapsed() {
        let msg = ToolCallMessage::running("t2", "bash", Some("ls".into()));
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, Some(1234), None, Some("⠋"), false, false);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("1s")));
    }

    #[test]
    fn tool_call_grouped_indent() {
        let msg = ToolCallMessage::running("t3", "bash", None);
        let palette = LocusPalette::locus_dark();
        let lines = tool_call_lines(&msg, &palette, None, None, Some("⠋"), true, false);
        // Grouped tools have tree connector (├── or └──)
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.content.contains("├──") || s.content.contains("└──"))
        );
    }

    #[test]
    fn tool_group_header_reports_running_state() {
        let palette = LocusPalette::locus_dark();
        let tools = vec![
            ToolCallMessage::running("t1", "bash", None),
            ToolCallMessage::done(Some("t2".into()), "grep", 20, true, None, None),
        ];
        let line = tool_group_header_line(&tools, &palette, Some("⠋"));
        assert!(line.spans.iter().any(|s| s.content.contains("Tools")));
        assert!(line.spans.iter().any(|s| s.content.contains("⠋")));
        assert!(line.spans.iter().any(|s| s.content.contains("2")));
    }
}
