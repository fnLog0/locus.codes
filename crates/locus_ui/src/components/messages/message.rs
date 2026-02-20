//! Single message in the conversation: role, content blocks, and render.
//!
//! User and assistant are differentiated by a colored bar and symbol on the same side:
//! ```text
//! █ Add error handling to the login function   (user: primary bar, bright)
//! · Here's the updated function:                (assistant: muted bar, dull)
//! ```

use chrono::{DateTime, Utc};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::components::constants::LEFT_PADDING;
use crate::theme::Theme;

use super::content::ContentBlock;
use super::role::Role;
use super::tool::ToolStatus;

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

        let mut lines = 1;

        for block in &self.content {
            match block {
                ContentBlock::Text(text) => {
                    let wrapped = textwrap::wrap(text, width as usize);
                    lines += wrapped.len().max(1);
                }
                ContentBlock::Thinking { text, expanded } => {
                    if *expanded {
                        let wrapped = textwrap::wrap(text, (width - 4) as usize);
                        lines += 1 + wrapped.len().max(1);
                    } else {
                        lines += 1;
                    }
                }
                ContentBlock::ToolUse(tool) => {
                    lines += 2;
                    if let Some(ref output) = tool.output {
                        let wrapped = textwrap::wrap(output, (width - 4) as usize);
                        lines += 1 + wrapped.len();
                    }
                }
                ContentBlock::ToolResult { output, .. } => {
                    let wrapped = textwrap::wrap(output, (width - 4) as usize);
                    lines += wrapped.len().max(1);
                }
            }
            lines += 1;
        }

        lines as u16
    }

    /// Render the message into the frame. Uses colored bar (█/·/!) on first line; assistant is duller.
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let (role_style, content_fg) = match self.role {
            Role::User => (
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
                theme.fg,
            ),
            Role::Assistant => (Style::default().fg(theme.muted_fg), theme.fg),
            Role::System => (
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::ITALIC),
                theme.fg,
            ),
        };
        let bar = self.role.bar_char();
        let mut first_line = true;

        let push_line = |lines: &mut Vec<Line<'static>>,
                         first_line: &mut bool,
                         prefix: &str,
                         content: &str,
                         style: Style| {
            let line = if *first_line {
                *first_line = false;
                Line::from(vec![
                    Span::styled(bar, role_style),
                    Span::raw(" "),
                    Span::styled(content.to_string(), style),
                ])
            } else {
                Line::from(Span::styled(format!("{}{}", prefix, content), style))
            };
            lines.push(line);
        };

        let push_spans =
            |lines: &mut Vec<Line<'static>>, first_line: &mut bool, spans: Vec<Span<'static>>| {
                if *first_line {
                    *first_line = false;
                    let mut with_bar = vec![Span::styled(bar, role_style), Span::raw(" ")];
                    with_bar.extend(spans);
                    lines.push(Line::from(with_bar));
                } else {
                    lines.push(Line::from(spans));
                }
            };

        for block in &self.content {
            match block {
                ContentBlock::Text(text) => {
                    let wrapped = textwrap::wrap(text, area.width.saturating_sub(2) as usize);
                    for line in wrapped {
                        push_line(
                            &mut lines,
                            &mut first_line,
                            LEFT_PADDING,
                            &line.to_string(),
                            Style::default().fg(content_fg),
                        );
                    }
                }
                ContentBlock::Thinking { text, expanded } => {
                    if *expanded {
                        push_line(
                            &mut lines,
                            &mut first_line,
                            "",
                            "▼ thinking",
                            Style::default().fg(content_fg),
                        );
                        for line in text.lines() {
                            push_line(
                                &mut lines,
                                &mut first_line,
                                "  ",
                                line,
                                Style::default().fg(content_fg),
                            );
                        }
                    } else {
                        push_line(
                            &mut lines,
                            &mut first_line,
                            LEFT_PADDING,
                            "▶ thinking...",
                            Style::default().fg(content_fg),
                        );
                    }
                }
                ContentBlock::ToolUse(tool) => {
                    let (indicator_color, status_bg) = match tool.status {
                        ToolStatus::Running => (theme.fg, theme.tool_bg),
                        ToolStatus::Done => (theme.primary_fg, theme.success),
                        ToolStatus::Error => (theme.primary_fg, theme.danger),
                    };

                    push_spans(
                        &mut lines,
                        &mut first_line,
                        vec![
                            Span::styled(
                                format!(" {} ", tool.status.indicator()),
                                Style::default().fg(indicator_color).bg(status_bg),
                            ),
                            Span::raw(" "),
                            Span::styled(
                                tool.name.clone(),
                                Style::default().fg(content_fg).add_modifier(Modifier::BOLD),
                            ),
                        ],
                    );

                    if let Some(path) = tool.args.get("file_path").and_then(|v| v.as_str()) {
                        push_spans(
                            &mut lines,
                            &mut first_line,
                            vec![
                                Span::styled("│", Style::default().fg(theme.border)),
                                Span::styled(format!(" {}", path), Style::default().fg(content_fg)),
                            ],
                        );
                    } else if let Some(cmd) = tool.args.get("command").and_then(|v| v.as_str()) {
                        push_spans(
                            &mut lines,
                            &mut first_line,
                            vec![
                                Span::styled("│", Style::default().fg(theme.border)),
                                Span::styled(format!(" {}", cmd), Style::default().fg(content_fg)),
                            ],
                        );
                    }

                    if let Some(ref output) = tool.output {
                        push_spans(
                            &mut lines,
                            &mut first_line,
                            vec![
                                Span::styled("│", Style::default().fg(theme.border)),
                                Span::styled(
                                    " ".repeat(area.width.saturating_sub(1) as usize),
                                    Style::default().bg(theme.tool_bg),
                                ),
                            ],
                        );
                        for line in output.lines() {
                            push_spans(
                                &mut lines,
                                &mut first_line,
                                vec![
                                    Span::styled("│", Style::default().fg(theme.border)),
                                    Span::styled(
                                        format!(" {}", line),
                                        Style::default().fg(content_fg).bg(theme.tool_bg),
                                    ),
                                ],
                            );
                        }
                    }

                    if let Some(d) = tool.duration {
                        push_line(
                            &mut lines,
                            &mut first_line,
                            LEFT_PADDING,
                            &format!("{}ms", d.as_millis()),
                            Style::default().fg(content_fg),
                        );
                    }
                }
                ContentBlock::ToolResult {
                    output, is_error, ..
                } => {
                    let color = if *is_error { theme.danger } else { content_fg };
                    for line in output.lines() {
                        push_line(
                            &mut lines,
                            &mut first_line,
                            LEFT_PADDING,
                            line,
                            Style::default().fg(color),
                        );
                    }
                }
            }
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
