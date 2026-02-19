//! Message component - single message block with role and content.
//!
//! User message:
//! ```text
//! USER
//! Add error handling to the login function
//! ```
//!
//! Assistant message with code:
//! ```text
//! ASSISTANT
//! Here's the updated function:
//!
//!     pub fn login() { ... }
//! ```

use chrono::{DateTime, Utc};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Duration;

use crate::theme::Theme;
use super::constants::LEFT_PADDING;

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            Role::User => "YOU",
            Role::Assistant => "ASSISTANT",
            Role::System => "SYSTEM",
        }
    }
}

/// Content block within a message.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// Plain text content.
    Text(String),
    /// Thinking/reasoning block (collapsible).
    Thinking { text: String, expanded: bool },
    /// Tool use display.
    ToolUse(ToolDisplay),
    /// Tool result.
    ToolResult {
        tool_id: String,
        output: String,
        is_error: bool,
    },
}

/// Tool execution display.
#[derive(Debug, Clone)]
pub struct ToolDisplay {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    pub status: ToolStatus,
    pub output: Option<String>,
    pub duration: Option<Duration>,
}

/// Tool execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Done,
    Error,
}

impl ToolStatus {
    /// Get the status indicator symbol.
    pub fn indicator(&self) -> &'static str {
        match self {
            ToolStatus::Running => "*",
            ToolStatus::Done => "+",
            ToolStatus::Error => "!",
        }
    }
}

/// A single message in the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
}

impl Message {
    /// Create a user message with text.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text(text.into())],
            timestamp: Utc::now(),
        }
    }

    /// Create an assistant message with content blocks.
    pub fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: blocks,
            timestamp: Utc::now(),
        }
    }

    /// Create an assistant message with plain text.
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text(text.into())],
            timestamp: Utc::now(),
        }
    }

    /// Estimate the height needed to render this message.
    pub fn height(&self, width: u16) -> u16 {
        if width == 0 {
            return 1;
        }

        let mut lines = 1; // Role label

        for block in &self.content {
            match block {
                ContentBlock::Text(text) => {
                    let wrapped = textwrap::wrap(text, width as usize);
                    lines += wrapped.len().max(1);
                }
                ContentBlock::Thinking { text, expanded } => {
                    if *expanded {
                        let wrapped = textwrap::wrap(text, (width - 4) as usize);
                        lines += 1 + wrapped.len().max(1); // header + content
                    } else {
                        lines += 1; // Just "Thinking..."
                    }
                }
                ContentBlock::ToolUse(tool) => {
                    lines += 2; // header + path
                    if let Some(ref output) = tool.output {
                        let wrapped = textwrap::wrap(output, (width - 4) as usize);
                        lines += 1 + wrapped.len(); // separator + output
                    }
                }
                ContentBlock::ToolResult { output, .. } => {
                    let wrapped = textwrap::wrap(output, (width - 4) as usize);
                    lines += wrapped.len().max(1);
                }
            }
            lines += 1; // Blank line after each block
        }

        lines as u16
    }

    /// Render the message into the frame.
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines: Vec<Line> = Vec::new();

        // Role label with distinct hierarchy
        let role_style = match self.role {
            Role::User => Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
            Role::Assistant => Style::default().fg(theme.muted_fg),
            Role::System => Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::ITALIC),
        };
        lines.push(Line::from(Span::styled(self.role.label(), role_style)));

        // Content blocks
        for block in &self.content {
            match block {
                ContentBlock::Text(text) => {
                    let wrapped = textwrap::wrap(text, area.width as usize);
                    for line in wrapped {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(theme.fg),
                        )));
                    }
                }
                ContentBlock::Thinking { text, expanded } => {
                    if *expanded {
                        lines.push(Line::from(Span::styled(
                            format!("{}▼ thinking", LEFT_PADDING),
                            Style::default().fg(theme.faint),
                        )));
                        for line in text.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("{}  {}", LEFT_PADDING, line),
                                Style::default().fg(theme.faint),
                            )));
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("{}▶ thinking...", LEFT_PADDING),
                            Style::default().fg(theme.faint),
                        )));
                    }
                }
                ContentBlock::ToolUse(tool) => {
                    let (indicator_color, status_bg) = match tool.status {
                        ToolStatus::Running => (theme.fg, theme.tool_bg),
                        ToolStatus::Done => (theme.primary_fg, theme.success),
                        ToolStatus::Error => (theme.primary_fg, theme.danger),
                    };

                    // Tool header with background emphasis
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(" {} ", tool.status.indicator()),
                            Style::default().fg(indicator_color).bg(status_bg),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            tool.name.clone(),
                            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    // Tool args with left border grouping
                    if let Some(path) = tool.args.get("file_path").and_then(|v| v.as_str()) {
                        lines.push(Line::from(vec![
                            Span::styled("│", Style::default().fg(theme.border)),
                            Span::styled(format!(" {}", path), Style::default().fg(theme.faint)),
                        ]));
                    } else if let Some(cmd) = tool.args.get("command").and_then(|v| v.as_str()) {
                        lines.push(Line::from(vec![
                            Span::styled("│", Style::default().fg(theme.border)),
                            Span::styled(format!(" {}", cmd), Style::default().fg(theme.faint)),
                        ]));
                    }

                    // Output with grouped container
                    if let Some(ref output) = tool.output {
                        lines.push(Line::from(vec![
                            Span::styled("│", Style::default().fg(theme.border)),
                            Span::styled(
                                " ".repeat(area.width.saturating_sub(1) as usize),
                                Style::default().bg(theme.tool_bg),
                            ),
                        ]));
                        for line in output.lines() {
                            lines.push(Line::from(vec![
                                Span::styled("│", Style::default().fg(theme.border)),
                                Span::styled(
                                    format!(" {}", line),
                                    Style::default().fg(theme.fg).bg(theme.tool_bg),
                                ),
                            ]));
                        }
                    }

                    // Duration at bottom, very subtle
                    if let Some(d) = tool.duration {
                        lines.push(Line::from(Span::styled(
                            format!("{}{}ms", LEFT_PADDING, d.as_millis()),
                            Style::default().fg(theme.faint),
                        )));
                    }
                }
                ContentBlock::ToolResult {
                    output, is_error, ..
                } => {
                    let color = if *is_error { theme.danger } else { theme.fg };
                    for line in output.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("{}{}", LEFT_PADDING, line),
                            Style::default().fg(color),
                        )));
                    }
                }
            }
            // Blank line after each block for breathing room
            lines.push(Line::raw(""));
        }

        let paragraph = Paragraph::new(lines);
        f.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content.len(), 1);
    }

    #[test]
    fn message_assistant() {
        let msg = Message::assistant_text("Hi there");
        assert_eq!(msg.role, Role::Assistant);
    }

    #[test]
    fn tool_status_indicators() {
        assert_eq!(ToolStatus::Running.indicator(), "*");
        assert_eq!(ToolStatus::Done.indicator(), "+");
        assert_eq!(ToolStatus::Error.indicator(), "!");
    }
}
