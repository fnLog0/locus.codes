//! Input component - multiline text input with editing support.
//!
//! Layout:
//! ```text
//! > Add tests for the error cases_
//! ```

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

/// Text input area with editing and history support.
#[derive(Debug, Clone)]
pub struct Input {
    /// Current input text.
    text: String,
    /// Cursor position (byte index in text).
    cursor: usize,
    /// Command history (most recent last).
    history: Vec<String>,
    /// Current position in history (None = not navigating history).
    history_index: Option<usize>,
    /// Saved text before history navigation.
    saved_text: String,
    /// Placeholder text when empty.
    placeholder: String,
}

impl Input {
    /// Create a new input field.
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_index: None,
            saved_text: String::new(),
            placeholder: "Type a message...".into(),
        }
    }

    /// Get the current text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Check if input is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Set placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
    }

    /// Insert a character at cursor position.
    pub fn insert(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    /// Insert a string at cursor position.
    pub fn insert_str(&mut self, s: &str) {
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the start of the previous grapheme
            let graphemes: Vec<(usize, &str)> = self.text.grapheme_indices(true).collect();
            // Find the grapheme that ends at cursor position, or the last one if cursor is at end
            let prev_idx = graphemes
                .iter()
                .position(|(i, g)| *i + g.len() == self.cursor)
                .or_else(|| graphemes.iter().rposition(|(i, _)| *i < self.cursor));
            if let Some(p) = prev_idx {
                let pos = graphemes[p].0;
                let end = self.cursor;
                self.cursor = pos;
                self.text.drain(pos..end);
            }
        }
    }

    /// Delete character at cursor (delete key).
    pub fn delete(&mut self) {
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }

    /// Move cursor left by one grapheme.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            let graphemes: Vec<(usize, &str)> = self.text.grapheme_indices(true).collect();
            if let Some(pos) = graphemes
                .iter()
                .position(|(i, _)| *i == self.cursor)
                .and_then(|p| if p > 0 { Some(p - 1) } else { None })
            {
                self.cursor = graphemes[pos].0;
            }
        }
    }

    /// Move cursor right by one grapheme.
    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            let graphemes: Vec<(usize, &str)> = self.text.grapheme_indices(true).collect();
            if let Some(pos) = graphemes
                .iter()
                .position(|(i, _)| *i == self.cursor)
                .map(|p| p + 1)
            {
                if pos < graphemes.len() {
                    self.cursor = graphemes[pos].0;
                } else {
                    self.cursor = self.text.len();
                }
            }
        }
    }

    /// Move cursor to start of line.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end of line.
    pub fn move_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Submit the current text and add to history.
    /// Returns Some(text) if there was text to submit.
    pub fn submit(&mut self) -> Option<String> {
        if self.text.is_empty() {
            return None;
        }

        let text = std::mem::take(&mut self.text);
        self.cursor = 0;

        // Add to history (avoid consecutive duplicates)
        if self.history.last() != Some(&text) {
            self.history.push(text.clone());
        }

        // Limit history size
        const MAX_HISTORY: usize = 100;
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }

        // Reset history navigation
        self.history_index = None;
        self.saved_text.clear();

        Some(text)
    }

    /// Navigate up in history.
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        // Save current text before first navigation
        if self.history_index.is_none() {
            self.saved_text = self.text.clone();
        }

        let new_index = match self.history_index {
            None => self.history.len() - 1,
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
        };

        self.history_index = Some(new_index);
        self.text = self.history[new_index].clone();
        self.cursor = self.text.len();
    }

    /// Navigate down in history.
    pub fn history_down(&mut self) {
        match self.history_index {
            None => {}
            Some(i) if i + 1 >= self.history.len() => {
                // At the end, restore saved text
                self.history_index = None;
                self.text = std::mem::take(&mut self.saved_text);
                self.cursor = self.text.len();
            }
            Some(i) => {
                self.history_index = Some(i + 1);
                self.text = self.history[i + 1].clone();
                self.cursor = self.text.len();
            }
        }
    }

    /// Clear the input.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.history_index = None;
        self.saved_text.clear();
    }

    /// Render the input into the frame.
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Use Block with padding - background fills entire area, content in padded inner
        let block = Block::default()
            .style(Style::default().bg(theme.input))
            .padding(Padding::new(2, 2, 1, 1)); // left, right, top, bottom

        let inner = block.inner(area);
        f.render_widget(block, area);

        let (display_text, cursor_pos) = if self.text.is_empty() {
            (self.placeholder.as_str(), 0)
        } else {
            (self.text.as_str(), self.text[..self.cursor].width())
        };

        let prompt = Span::styled("> ", Style::default().fg(theme.primary));
        let content = if self.text.is_empty() {
            Span::styled(display_text, Style::default().fg(theme.muted_fg))
        } else {
            Span::styled(display_text, Style::default().fg(theme.fg))
        };

        // Add cursor indicator
        let cursor_indicator = if self.text.is_empty() {
            Span::styled("_", Style::default().fg(theme.primary))
        } else {
            Span::raw("")
        };

        let line = Line::from(vec![prompt, content, cursor_indicator]);
        let paragraph = Paragraph::new(line);

        f.render_widget(paragraph, inner);

        // Position cursor for display (actual terminal cursor)
        // Note: cursor_pos is the width of text before cursor
        let cursor_x = inner.x + 2 + cursor_pos as u16; // 2 for "> "
        let cursor_y = inner.y;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_basic() {
        let mut input = Input::new();
        assert!(input.is_empty());

        input.insert('a');
        input.insert('b');
        input.insert('c');
        assert_eq!(input.text(), "abc");
    }

    #[test]
    fn input_backspace() {
        let mut input = Input::new();
        input.insert_str("hello");
        input.backspace();
        assert_eq!(input.text(), "hell");
    }

    #[test]
    fn input_submit() {
        let mut input = Input::new();
        input.insert_str("test message");
        let result = input.submit();
        assert_eq!(result, Some("test message".to_string()));
        assert!(input.is_empty());
    }

    #[test]
    fn input_history() {
        let mut input = Input::new();
        input.insert_str("first");
        input.submit();
        input.insert_str("second");
        input.submit();

        input.history_up();
        assert_eq!(input.text(), "second");

        input.history_up();
        assert_eq!(input.text(), "first");

        input.history_down();
        assert_eq!(input.text(), "second");
    }
}
