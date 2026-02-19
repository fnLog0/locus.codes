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
            Role::User => "USER",
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

        // Role label
        let role_style = Style::default()
            .fg(theme.muted_fg)
            .add_modifier(Modifier::BOLD);
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
                            "v Thinking",
                            Style::default().fg(theme.muted_fg),
                        )));
                        for line in text.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("    {}", line),
                                Style::default().fg(theme.muted_fg),
                            )));
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            "> Thinking...",
                            Style::default().fg(theme.muted_fg),
                        )));
                    }
                }
                ContentBlock::ToolUse(tool) => {
                    let indicator_color = match tool.status {
                        ToolStatus::Running => theme.accent,
                        ToolStatus::Done => theme.success,
                        ToolStatus::Error => theme.danger,
                    };
                    let status_text = match tool.status {
                        ToolStatus::Running => String::new(),
                        ToolStatus::Done | ToolStatus::Error => {
                            format!(" • {}ms", tool.duration.map(|d| d.as_millis()).unwrap_or(0))
                        }
                    };

                    lines.push(Line::from(vec![
                        Span::styled(
                            tool.status.indicator(),
                            Style::default().fg(indicator_color),
                        ),
                        Span::raw(" "),
                        Span::styled(tool.name.clone(), Style::default().fg(theme.tool_name)),
                        Span::styled(status_text, Style::default().fg(theme.muted_fg)),
                    ]));

                    // Tool args (file path, command, etc.)
                    if let Some(path) = tool.args.get("file_path").and_then(|v| v.as_str()) {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", path),
                            Style::default().fg(theme.file_path),
                        )));
                    } else if let Some(cmd) = tool.args.get("command").and_then(|v| v.as_str()) {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", cmd),
                            Style::default().fg(theme.fg),
                        )));
                    }

                    // Output if present
                    if let Some(ref output) = tool.output {
                        lines.push(Line::from(Span::styled(
                            "    ─".repeat(area.width as usize / 2),
                            Style::default().fg(theme.muted_fg),
                        )));
                        for line in output.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("    {}", line),
                                Style::default().fg(theme.fg),
                            )));
                        }
                    }
                }
                ContentBlock::ToolResult {
                    output, is_error, ..
                } => {
                    let color = if *is_error { theme.danger } else { theme.fg };
                    for line in output.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", line),
                            Style::default().fg(color),
                        )));
                    }
                }
            }
            // Blank line after each block
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
