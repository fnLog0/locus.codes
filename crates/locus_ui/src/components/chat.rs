//! Chat view component - main conversation panel with scrolling.
//!
//! Combines a list of messages with scroll state and auto-scroll behavior.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme::Theme;
use crate::components::{
    collapse_empty_lines, horizontal_padding, Message, ScrollIndicator, ScrollPanel,
    LEFT_PADDING, MESSAGE_SPACING_LINES,
};

/// Maximum number of messages to keep in memory.
const MAX_MESSAGES: usize = 1000;

/// Chat view holding messages and scroll state.
#[derive(Debug, Clone)]
pub struct Chat {
    messages: Vec<Message>,
    scroll: ScrollPanel,
}

impl Chat {
    /// Create a new empty chat view.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll: ScrollPanel::new(),
        }
    }

    /// Get the list of messages.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get the number of messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if chat is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Push a new message to the chat.
    pub fn push(&mut self, msg: Message) {
        // Enforce message limit
        if self.messages.len() >= MAX_MESSAGES {
            self.messages.remove(0);
        }
        self.messages.push(msg);
        self.recalculate_scroll();
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll.set_content_height(0);
    }

    /// Scroll up by lines.
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll.scroll_up(lines);
    }

    /// Scroll down by lines.
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll.scroll_down(lines);
    }

    /// Scroll up by a page.
    pub fn page_up(&mut self) {
        self.scroll.page_up();
    }

    /// Scroll down by a page.
    pub fn page_down(&mut self) {
        self.scroll.page_down();
    }

    /// Scroll to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll.scroll_to_bottom();
    }

    /// Check if auto-scroll is enabled.
    pub fn auto_scroll(&self) -> bool {
        self.scroll.auto_scroll
    }

    /// Enable or disable auto-scroll.
    pub fn set_auto_scroll(&mut self, enabled: bool) {
        self.scroll.set_auto_scroll(enabled);
    }

    /// Recalculate scroll state based on current messages.
    fn recalculate_scroll(&mut self) {
        // Estimate total content height
        // This is approximate - actual height depends on render width
        let estimated_height: usize = self
            .messages
            .iter()
            .map(|m| m.height(80) as usize) // Assume 80 width for estimate
            .sum();
        self.scroll.set_content_height(estimated_height);
    }

    /// Render the chat into the frame.
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Fill background for full area first
        let bg_block = ratatui::widgets::Block::default().style(Style::default().bg(theme.bg));
        f.render_widget(bg_block, area);

        // Apply horizontal padding using layout utility
        let padded_area = horizontal_padding(area);

        // Build all lines from messages
        let mut all_lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            // Role label with distinct hierarchy
            let role_style = match msg.role {
                crate::components::Role::User => Style::default()
                    .fg(theme.primary)
                    .add_modifier(ratatui::style::Modifier::BOLD),
                crate::components::Role::Assistant => Style::default().fg(theme.muted_fg),
                crate::components::Role::System => Style::default()
                    .fg(theme.warning)
                    .add_modifier(ratatui::style::Modifier::ITALIC),
            };
            let role_label = match msg.role {
                crate::components::Role::User => "YOU",
                crate::components::Role::Assistant => "ASSISTANT",
                crate::components::Role::System => "SYSTEM",
            };
            all_lines.push(Line::from(Span::styled(role_label, role_style)));

            // Content blocks
            for block in &msg.content {
                match block {
                    crate::components::ContentBlock::Text(text) => {
                        let wrapped = textwrap::wrap(text, padded_area.width as usize);
                        for line in wrapped {
                            all_lines.push(Line::from(Span::styled(
                                line.to_string(),
                                Style::default().fg(theme.fg),
                            )));
                        }
                    }
                    crate::components::ContentBlock::Thinking { text, expanded } => {
                        if *expanded {
                            all_lines.push(Line::from(Span::styled(
                                format!("{}▼ thinking", LEFT_PADDING),
                                Style::default().fg(theme.faint),
                            )));
                            for line in text.lines() {
                                all_lines.push(Line::from(Span::styled(
                                    format!("{}  {}", LEFT_PADDING, line),
                                    Style::default().fg(theme.faint),
                                )));
                            }
                        } else {
                            all_lines.push(Line::from(Span::styled(
                                format!("{}▶ thinking...", LEFT_PADDING),
                                Style::default().fg(theme.faint),
                            )));
                        }
                    }
                    crate::components::ContentBlock::ToolUse(tool) => {
                        let (indicator_color, status_bg) = match tool.status {
                            crate::components::ToolStatus::Running => (theme.fg, theme.tool_bg),
                            crate::components::ToolStatus::Done => (theme.primary_fg, theme.success),
                            crate::components::ToolStatus::Error => (theme.primary_fg, theme.danger),
                        };

                        // Tool header with background emphasis
                        all_lines.push(Line::from(vec![
                            Span::styled(
                                format!(" {} ", tool.status.indicator()),
                                Style::default().fg(indicator_color).bg(status_bg),
                            ),
                            Span::raw(" "),
                            Span::styled(
                                tool.name.clone(),
                                Style::default()
                                    .fg(theme.fg)
                                    .add_modifier(ratatui::style::Modifier::BOLD),
                            ),
                        ]));

                        // Args with left border grouping
                        if let Some(path) = tool.args.get("file_path").and_then(|v| v.as_str()) {
                            all_lines.push(Line::from(vec![
                                Span::styled("│", Style::default().fg(theme.border)),
                                Span::styled(format!(" {}", path), Style::default().fg(theme.faint)),
                            ]));
                        } else if let Some(cmd) = tool.args.get("command").and_then(|v| v.as_str())
                        {
                            all_lines.push(Line::from(vec![
                                Span::styled("│", Style::default().fg(theme.border)),
                                Span::styled(format!(" {}", cmd), Style::default().fg(theme.faint)),
                            ]));
                        }

                        // Output with grouped container
                        if let Some(ref output) = tool.output {
                            all_lines.push(Line::from(vec![
                                Span::styled("│", Style::default().fg(theme.border)),
                                Span::styled(
                                    " ".repeat(padded_area.width.saturating_sub(1) as usize),
                                    Style::default().bg(theme.tool_bg),
                                ),
                            ]));
                            for line in output.lines() {
                                all_lines.push(Line::from(vec![
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
                            all_lines.push(Line::from(Span::styled(
                                format!("{}{}ms", LEFT_PADDING, d.as_millis()),
                                Style::default().fg(theme.faint),
                            )));
                        }
                    }
                    crate::components::ContentBlock::ToolResult {
                        output, is_error, ..
                    } => {
                        let color = if *is_error { theme.danger } else { theme.fg };
                        for line in output.lines() {
                            all_lines.push(Line::from(Span::styled(
                                format!("{}{}", LEFT_PADDING, line),
                                Style::default().fg(color),
                            )));
                        }
                    }
                }
            }
            // Separator line between messages for breathing room
            all_lines.push(Line::from(Span::styled(
                "─".repeat(padded_area.width.saturating_sub(0) as usize),
                Style::default().fg(theme.border),
            )));
            // Add spacing lines between messages
            for _ in 0..MESSAGE_SPACING_LINES {
                all_lines.push(Line::raw(""));
            }
        }

        // Collapse excessive empty lines
        let all_lines = collapse_empty_lines(all_lines);

        // Apply scroll offset
        let total_lines = all_lines.len();
        let viewport_lines = area.height as usize;

        // Update scroll state
        let mut scroll = self.scroll.clone();
        scroll.set_viewport_height(viewport_lines);
        scroll.set_content_height(total_lines);

        let start = scroll.offset.min(all_lines.len().saturating_sub(viewport_lines));
        let visible_lines: Vec<Line> = all_lines.into_iter().skip(start).take(viewport_lines).collect();

        let paragraph = Paragraph::new(visible_lines).style(Style::default().bg(theme.bg));
        f.render_widget(paragraph, padded_area);

        // Render scroll indicator if needed
        if let Some(indicator) = scroll.indicator() {
            let indicator_text = match indicator {
                ScrollIndicator::CanScrollDown => "[scroll down for more...]",
                ScrollIndicator::CanScrollUp => "[scroll up for more...]",
                ScrollIndicator::CanScrollBoth => "[\u{2191} scroll up \u{2022} scroll down \u{2193}]",
            };
            let _indicator_span = Span::styled(indicator_text, Style::default().fg(theme.muted_fg));
            // Note: In a full implementation, we'd overlay this at the bottom
        }
    }
}

impl Default for Chat {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_new() {
        let chat = Chat::new();
        assert!(chat.is_empty());
        assert_eq!(chat.len(), 0);
    }

    #[test]
    fn chat_push() {
        let mut chat = Chat::new();
        chat.push(Message::user("Hello"));
        assert_eq!(chat.len(), 1);
        chat.push(Message::assistant_text("Hi"));
        assert_eq!(chat.len(), 2);
    }

    #[test]
    fn chat_clear() {
        let mut chat = Chat::new();
        chat.push(Message::user("Hello"));
        chat.clear();
        assert!(chat.is_empty());
    }
}
