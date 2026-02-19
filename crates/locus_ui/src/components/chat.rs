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
use crate::components::{Message, ScrollIndicator, ScrollPanel};

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
        // Build all lines from messages
        let mut all_lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            // Role label
            all_lines.push(Line::from(Span::styled(
                msg.role.label(),
                Style::default()
                    .fg(theme.muted_fg)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )));

            // Content blocks
            for block in &msg.content {
                match block {
                    crate::components::ContentBlock::Text(text) => {
                        let wrapped = textwrap::wrap(text, area.width as usize);
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
                                "v Thinking",
                                Style::default().fg(theme.muted_fg),
                            )));
                            for line in text.lines() {
                                all_lines.push(Line::from(Span::styled(
                                    format!("    {}", line),
                                    Style::default().fg(theme.muted_fg),
                                )));
                            }
                        } else {
                            all_lines.push(Line::from(Span::styled(
                                "> Thinking...",
                                Style::default().fg(theme.muted_fg),
                            )));
                        }
                    }
                    crate::components::ContentBlock::ToolUse(tool) => {
                        let indicator_color = match tool.status {
                            crate::components::ToolStatus::Running => theme.accent,
                            crate::components::ToolStatus::Done => theme.success,
                            crate::components::ToolStatus::Error => theme.danger,
                        };
                        let duration_text = tool
                            .duration
                            .map(|d| format!(" • {}ms", d.as_millis()))
                            .unwrap_or_default();

                        all_lines.push(Line::from(vec![
                            Span::styled(
                                tool.status.indicator(),
                                Style::default().fg(indicator_color),
                            ),
                            Span::raw(" "),
                            Span::styled(&tool.name, Style::default().fg(theme.tool_name)),
                            Span::styled(duration_text, Style::default().fg(theme.muted_fg)),
                        ]));

                        // Args display
                        if let Some(path) = tool.args.get("file_path").and_then(|v| v.as_str()) {
                            all_lines.push(Line::from(Span::styled(
                                format!("    {}", path),
                                Style::default().fg(theme.file_path),
                            )));
                        } else if let Some(cmd) = tool.args.get("command").and_then(|v| v.as_str())
                        {
                            all_lines.push(Line::from(Span::styled(
                                format!("    {}", cmd),
                                Style::default().fg(theme.fg),
                            )));
                        }

                        // Output
                        if let Some(ref output) = tool.output {
                            all_lines.push(Line::from(Span::styled(
                                "    ".to_string() + &"─".repeat(area.width as usize / 2 - 2),
                                Style::default().fg(theme.muted_fg),
                            )));
                            for line in output.lines() {
                                all_lines.push(Line::from(Span::styled(
                                    format!("    {}", line),
                                    Style::default().fg(theme.fg),
                                )));
                            }
                        }
                    }
                    crate::components::ContentBlock::ToolResult {
                        output, is_error, ..
                    } => {
                        let color = if *is_error { theme.danger } else { theme.fg };
                        for line in output.lines() {
                            all_lines.push(Line::from(Span::styled(
                                format!("    {}", line),
                                Style::default().fg(color),
                            )));
                        }
                    }
                }
            }
            // Blank line after each message
            all_lines.push(Line::raw(""));
        }

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
        f.render_widget(paragraph, area);

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
