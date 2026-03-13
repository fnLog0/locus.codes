//! Meta-tool message UI: display tool_search, tool_explain, and task.
//!
//! Types are TUI-only (no dependency on locus_runtime). The runtime maps
//! meta-tool names to [MetaToolKind] and builds [MetaToolMessage] for
//! display. Colors from [crate::theme] only.
//!
//! Per-meta-tool rendering modules handle specific layouts.

mod explain;
mod search;
mod tasks;

use std::time::Duration;

use ratatui::text::{Line, Span};

use crate::layouts::{danger_style, success_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::{LEFT_PADDING, format_duration};

// Re-export per-meta-tool functions
pub use explain::{explain_preview_line, explain_status_summary};
pub use search::search_status_summary;
pub use tasks::{task_completed_span, task_preview_line, task_status_summary};

/// Which meta-tool (for display label and optional icon).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaToolKind {
    /// tool_search — search for tools by intent.
    ToolSearch,
    /// tool_explain — get schema for a tool.
    ToolExplain,
    /// task — run a sub-agent task.
    Task,
}

impl MetaToolKind {
    /// Display label in the UI.
    pub fn label(self) -> &'static str {
        match self {
            MetaToolKind::ToolSearch => "Search tools",
            MetaToolKind::ToolExplain => "Explain tool",
            MetaToolKind::Task => "Task",
        }
    }

    /// Parse from runtime tool name (e.g. "tool_search" -> ToolSearch).
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "tool_search" => Some(MetaToolKind::ToolSearch),
            "tool_explain" => Some(MetaToolKind::ToolExplain),
            "task" => Some(MetaToolKind::Task),
            _ => None,
        }
    }
}

/// Status of a meta-tool call for display.
#[derive(Debug, Clone)]
pub enum MetaToolStatus {
    Running,
    Done { duration_ms: u64, success: bool },
    Error { message: String },
}

/// One meta-tool invocation to show in the chat/log.
#[derive(Debug, Clone)]
pub struct MetaToolMessage {
    pub kind: MetaToolKind,
    pub status: MetaToolStatus,
    /// Optional detail: query (search), tool_id (explain), description (task).
    pub detail: Option<String>,
    /// Optional args JSON for per-meta-tool rendering.
    pub args: Option<serde_json::Value>,
    /// Optional result JSON for per-meta-tool rendering.
    pub result: Option<serde_json::Value>,
}

impl MetaToolMessage {
    pub fn running(kind: MetaToolKind, detail: Option<String>) -> Self {
        Self {
            kind,
            status: MetaToolStatus::Running,
            detail,
            args: None,
            result: None,
        }
    }

    pub fn done(
        kind: MetaToolKind,
        duration_ms: u64,
        success: bool,
        detail: Option<String>,
    ) -> Self {
        Self {
            kind,
            status: MetaToolStatus::Done {
                duration_ms,
                success,
            },
            detail,
            args: None,
            result: None,
        }
    }

    pub fn error(kind: MetaToolKind, message: impl Into<String>, detail: Option<String>) -> Self {
        Self {
            kind,
            status: MetaToolStatus::Error {
                message: message.into(),
            },
            detail,
            args: None,
            result: None,
        }
    }

    /// Create with args and result for per-meta-tool rendering.
    pub fn with_data(
        kind: MetaToolKind,
        status: MetaToolStatus,
        detail: Option<String>,
        args: serde_json::Value,
        result: serde_json::Value,
    ) -> Self {
        Self {
            kind,
            status,
            detail,
            args: Some(args),
            result: Some(result),
        }
    }
}

const META_DETAIL_INDENT: &str = "   ";
const META_LABEL_WIDTH: usize = 13;
const META_RUNNING_INDICATOR: &str = "⠋";
const META_SUCCESS_INDICATOR: &str = "✓";
const META_FAILURE_INDICATOR: &str = "✗";

fn push_meta_label(spans: &mut Vec<Span<'static>>, msg: &MetaToolMessage, palette: &LocusPalette) {
    spans.push(Span::styled(
        format!("{:<width$}", msg.kind.label(), width = META_LABEL_WIDTH),
        text_style(palette.text),
    ));
}

fn meta_tool_detail_line(
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
        Span::raw(LEFT_PADDING),
        Span::raw(META_DETAIL_INDENT),
        Span::styled(text.into(), detail_style),
    ])
}

/// Get per-meta-tool status summary spans based on kind and args/result.
fn get_meta_tool_status_spans(msg: &MetaToolMessage, palette: &LocusPalette) -> Vec<Span<'static>> {
    let args = match &msg.args {
        Some(a) => a,
        None => return vec![],
    };
    let result = msg.result.as_ref().unwrap_or(&serde_json::Value::Null);

    match msg.kind {
        MetaToolKind::ToolSearch => search_status_summary(args, result, palette),
        MetaToolKind::ToolExplain => explain_status_summary(args, result, palette),
        MetaToolKind::Task => task_status_summary(args, result, palette),
    }
}

/// Get per-meta-tool preview lines based on kind and result.
fn get_meta_tool_preview_lines(
    msg: &MetaToolMessage,
    palette: &LocusPalette,
) -> Vec<Line<'static>> {
    let result = match &msg.result {
        Some(r) => r,
        None => return vec![],
    };

    match msg.kind {
        MetaToolKind::ToolSearch => vec![], // search has no preview
        MetaToolKind::ToolExplain => explain_preview_line(result, palette).into_iter().collect(),
        MetaToolKind::Task => task_preview_line(result, palette).into_iter().collect(),
    }
}

/// Build lines for a meta-tool call so it matches the transcript hierarchy.
pub fn meta_tool_lines(
    msg: &MetaToolMessage,
    palette: &LocusPalette,
    running_indicator: Option<&'static str>,
) -> Vec<Line<'static>> {
    let mut spans = vec![Span::raw(LEFT_PADDING)];

    match &msg.status {
        MetaToolStatus::Running => {
            spans.push(Span::styled(
                format!("{} ", running_indicator.unwrap_or(META_RUNNING_INDICATOR)),
                text_style(palette.accent),
            ));
            push_meta_label(&mut spans, msg, palette);

            // Use per-meta-tool status summary if available
            let summary_spans = get_meta_tool_status_spans(msg, palette);
            if !summary_spans.is_empty() {
                spans.push(Span::raw("  "));
                spans.extend(summary_spans);
            } else if let Some(d) = &msg.detail {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    d.clone(),
                    text_muted_style(palette.text_muted),
                ));
            }

            vec![Line::from(spans)]
        }
        MetaToolStatus::Done {
            duration_ms,
            success,
        } => {
            let status_style = if *success {
                success_style(palette.success)
            } else {
                danger_style(palette.danger)
            };
            let duration = format_duration(Duration::from_millis(*duration_ms));
            spans.push(Span::styled(
                format!(
                    "{} ",
                    if *success {
                        META_SUCCESS_INDICATOR
                    } else {
                        META_FAILURE_INDICATOR
                    }
                ),
                status_style,
            ));
            push_meta_label(&mut spans, msg, palette);

            // Use per-meta-tool status summary if available
            let summary_spans = get_meta_tool_status_spans(msg, palette);
            if !summary_spans.is_empty() {
                spans.push(Span::raw("  "));
                spans.extend(summary_spans);
            } else if let Some(d) = &msg.detail {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    d.clone(),
                    text_muted_style(palette.text_muted),
                ));
            }

            if !*success {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("failed".to_string(), status_style));
            }
            spans.push(Span::raw("  "));
            spans.push(Span::styled(duration, text_muted_style(palette.text_muted)));

            let mut lines = vec![Line::from(spans)];

            // Add preview lines for meta-tools that have them
            if *success {
                let preview = get_meta_tool_preview_lines(msg, palette);
                lines.extend(preview);
            }

            lines
        }
        MetaToolStatus::Error { message } => {
            spans.push(Span::styled("✗ ".to_string(), danger_style(palette.danger)));
            push_meta_label(&mut spans, msg, palette);
            let mut lines = vec![Line::from(spans)];
            if let Some(detail) = &msg.detail {
                lines.push(meta_tool_detail_line(palette, detail.clone(), false));
            }
            lines.push(meta_tool_detail_line(palette, message.clone(), true));
            lines
        }
    }
}

/// Build a single [Line] for a meta-tool call (backward compat for list consumers).
pub fn meta_tool_line(msg: &MetaToolMessage, palette: &LocusPalette) -> Line<'static> {
    meta_tool_lines(msg, palette, Some(META_RUNNING_INDICATOR))
        .into_iter()
        .next()
        .unwrap_or_else(|| Line::from(LEFT_PADDING))
}

/// One meta-tool for list view (e.g. in a "Meta tools" section).
#[derive(Debug, Clone)]
pub struct MetaToolInfo {
    pub kind: MetaToolKind,
    pub description: String,
}

/// Build a single [Line] for a meta-tool list entry.
pub fn meta_tool_info_line(info: &MetaToolInfo, palette: &LocusPalette) -> Line<'static> {
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(info.kind.label().to_string(), text_style(palette.text)),
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
    fn meta_tool_kind_from_name() {
        assert_eq!(
            MetaToolKind::from_name("tool_search"),
            Some(MetaToolKind::ToolSearch)
        );
        assert_eq!(
            MetaToolKind::from_name("tool_explain"),
            Some(MetaToolKind::ToolExplain)
        );
        assert_eq!(MetaToolKind::from_name("task"), Some(MetaToolKind::Task));
        assert!(MetaToolKind::from_name("bash").is_none());
    }

    #[test]
    fn meta_tool_line_running() {
        let msg = MetaToolMessage::running(MetaToolKind::ToolSearch, Some("create PR".into()));
        let palette = LocusPalette::locus_dark();
        let lines = meta_tool_lines(&msg, &palette, Some("⠋"));
        assert!(lines[0].spans.iter().any(|s| s.content.contains("⠋")));
    }

    #[test]
    fn meta_tool_info_line_builds() {
        let info = MetaToolInfo {
            kind: MetaToolKind::Task,
            description: "Run a sub-task in a separate agent.".into(),
        };
        let palette = LocusPalette::locus_dark();
        let line = meta_tool_info_line(&info, &palette);
        assert_eq!(line.spans.len(), 4);
    }

    #[test]
    fn meta_tool_done_success() {
        let msg = MetaToolMessage::done(
            MetaToolKind::ToolSearch,
            200,
            true,
            Some("find files".into()),
        );
        let palette = LocusPalette::locus_dark();
        let lines = meta_tool_lines(&msg, &palette, None);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✓")));
        assert!(!lines[0].spans.iter().any(|s| s.content.contains("done")));
    }

    #[test]
    fn meta_tool_error_shows_message() {
        let msg = MetaToolMessage::error(MetaToolKind::Task, "timed out", None);
        let palette = LocusPalette::locus_dark();
        let lines = meta_tool_lines(&msg, &palette, None);
        assert!(
            lines
                .iter()
                .any(|line| line.spans.iter().any(|s| s.content.contains("✗")))
        );
    }

    #[test]
    fn meta_tool_all_kinds_parse() {
        assert!(MetaToolKind::from_name("tool_search").is_some());
        assert!(MetaToolKind::from_name("tool_explain").is_some());
        assert!(MetaToolKind::from_name("task").is_some());
        assert!(MetaToolKind::from_name("unknown").is_none());
    }

    #[test]
    fn meta_tool_with_data_search() {
        let msg = MetaToolMessage::with_data(
            MetaToolKind::ToolSearch,
            MetaToolStatus::Done {
                duration_ms: 100,
                success: true,
            },
            None,
            serde_json::json!({"query": "edit files"}),
            serde_json::json!({"tools": ["edit_file", "create_file"]}),
        );
        let palette = LocusPalette::locus_dark();
        let lines = meta_tool_lines(&msg, &palette, None);
        // Should contain the query and tool count
        let content: String = lines[0].spans.iter().map(|s| s.content.clone()).collect();
        assert!(content.contains("edit files"));
        assert!(content.contains("2 tools"));
    }

    #[test]
    fn meta_tool_with_data_explain_preview() {
        let msg = MetaToolMessage::with_data(
            MetaToolKind::ToolExplain,
            MetaToolStatus::Done {
                duration_ms: 50,
                success: true,
            },
            None,
            serde_json::json!({"tool": "bash"}),
            serde_json::json!({"description": "Execute shell commands in the repository"}),
        );
        let palette = LocusPalette::locus_dark();
        let lines = meta_tool_lines(&msg, &palette, None);
        // Should have 2 lines: status + preview
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn meta_tool_with_data_task_preview() {
        let msg = MetaToolMessage::with_data(
            MetaToolKind::Task,
            MetaToolStatus::Done {
                duration_ms: 1500,
                success: true,
            },
            None,
            serde_json::json!({"description": "Add tests"}),
            serde_json::json!({"summary": "Created 3 test files"}),
        );
        let palette = LocusPalette::locus_dark();
        let lines = meta_tool_lines(&msg, &palette, None);
        // Should have 2 lines: status + preview
        assert_eq!(lines.len(), 2);
    }
}
