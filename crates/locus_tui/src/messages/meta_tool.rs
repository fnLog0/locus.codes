//! Meta-tool message UI: display tool_search, tool_explain, and task.
//!
//! Types are TUI-only (no dependency on locus_runtime). The runtime maps
//! meta-tool names to [MetaToolKind] and builds [MetaToolMessage] for
//! display. Colors from [crate::theme] only.

use std::time::Duration;

use ratatui::text::{Line, Span};

use crate::layouts::{danger_style, success_style, text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::{format_duration, LEFT_PADDING};

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
}

impl MetaToolMessage {
    pub fn running(kind: MetaToolKind, detail: Option<String>) -> Self {
        Self {
            kind,
            status: MetaToolStatus::Running,
            detail,
        }
    }

    pub fn done(kind: MetaToolKind, duration_ms: u64, success: bool, detail: Option<String>) -> Self {
        Self {
            kind,
            status: MetaToolStatus::Done { duration_ms, success },
            detail,
        }
    }

    pub fn error(kind: MetaToolKind, message: impl Into<String>, detail: Option<String>) -> Self {
        Self {
            kind,
            status: MetaToolStatus::Error { message: message.into() },
            detail,
        }
    }
}

const META_LEFT_BORDER: &str = "╎ ";
const META_DETAIL_INDENT: &str = "   ";
const META_LABEL_WIDTH: usize = 13;
const META_RUNNING_INDICATOR: &str = "⠋";
const META_SUCCESS_INDICATOR: &str = "✓";
const META_FAILURE_INDICATOR: &str = "✕";

fn push_meta_label(spans: &mut Vec<Span<'static>>, msg: &MetaToolMessage, palette: &LocusPalette) {
    spans.push(Span::styled(
        format!("{:<width$}", msg.kind.label(), width = META_LABEL_WIDTH),
        text_style(palette.text),
    ));
}

fn meta_tool_detail_line(palette: &LocusPalette, text: impl Into<String>, danger: bool) -> Line<'static> {
    let detail_style = if danger {
        danger_style(palette.danger)
    } else {
        text_muted_style(palette.text_muted)
    };
    Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(META_LEFT_BORDER.to_string(), text_muted_style(palette.text_muted)),
        Span::raw(META_DETAIL_INDENT),
        Span::styled(text.into(), detail_style),
    ])
}

/// Build lines for a meta-tool call so it matches the transcript hierarchy.
pub fn meta_tool_lines(
    msg: &MetaToolMessage,
    palette: &LocusPalette,
    running_indicator: Option<&'static str>,
) -> Vec<Line<'static>> {
    let mut spans = vec![
        Span::raw(LEFT_PADDING),
        Span::styled(META_LEFT_BORDER.to_string(), text_muted_style(palette.text_muted)),
    ];

    match &msg.status {
        MetaToolStatus::Running => {
            spans[1] = Span::styled(META_LEFT_BORDER.to_string(), text_style(palette.accent));
            spans.push(Span::styled(
                format!("{} ", running_indicator.unwrap_or(META_RUNNING_INDICATOR)),
                text_style(palette.accent),
            ));
            push_meta_label(&mut spans, msg, palette);
            if let Some(d) = &msg.detail {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(d.clone(), text_muted_style(palette.text_muted)));
            }
            vec![Line::from(spans)]
        }
        MetaToolStatus::Done { duration_ms, success } => {
            let status_style = if *success {
                success_style(palette.success)
            } else {
                danger_style(palette.danger)
            };
            spans[1] = Span::styled(META_LEFT_BORDER.to_string(), status_style);
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
            if let Some(d) = &msg.detail {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(d.clone(), text_muted_style(palette.text_muted)));
            }
            if !*success {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("failed".to_string(), status_style));
            }
            spans.push(Span::raw("  "));
            spans.push(Span::styled(duration, text_muted_style(palette.text_muted)));
            vec![Line::from(spans)]
        }
        MetaToolStatus::Error { message } => {
            spans[1] = Span::styled(META_LEFT_BORDER.to_string(), danger_style(palette.danger));
            push_meta_label(&mut spans, msg, palette);
            spans.push(Span::raw("  "));
            spans.push(Span::styled("error".to_string(), danger_style(palette.danger)));
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
        Span::styled(info.description.clone(), text_muted_style(palette.text_muted)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_tool_kind_from_name() {
        assert_eq!(MetaToolKind::from_name("tool_search"), Some(MetaToolKind::ToolSearch));
        assert_eq!(MetaToolKind::from_name("tool_explain"), Some(MetaToolKind::ToolExplain));
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
        let msg = MetaToolMessage::done(MetaToolKind::ToolSearch, 200, true, Some("find files".into()));
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
        assert!(lines
            .iter()
            .any(|line| line.spans.iter().any(|s| s.content.contains("timed out"))));
    }

    #[test]
    fn meta_tool_all_kinds_parse() {
        assert!(MetaToolKind::from_name("tool_search").is_some());
        assert!(MetaToolKind::from_name("tool_explain").is_some());
        assert!(MetaToolKind::from_name("task").is_some());
        assert!(MetaToolKind::from_name("unknown").is_none());
    }
}
