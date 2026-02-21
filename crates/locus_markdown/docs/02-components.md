# Glow Rust - Components Guide

This document details each UI component and how to implement them using ratatui.

## Core Components

### 1. Viewport

A scrollable content area for displaying rendered markdown.

```rust
// src/ui/components/viewport.rs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Text},
    widgets::{Scrollbar, ScrollbarState, ScrollbarOrientation, Widget},
};

/// Scrollable viewport widget
pub struct Viewport<'a> {
    content: Text<'a>,
    offset: usize,
    style: Style,
}

impl<'a> Viewport<'a> {
    pub fn new(content: Text<'a>) -> Self {
        Self {
            content,
            offset: 0,
            style: Style::default(),
        }
    }
    
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    /// Get total content height
    pub fn content_height(&self) -> usize {
        self.content.lines.len()
    }
    
    /// Get maximum scroll offset
    pub fn max_offset(&self, viewport_height: usize) -> usize {
        self.content_height().saturating_sub(viewport_height)
    }
    
    /// Calculate scroll percentage (0.0 - 1.0)
    pub fn scroll_percent(&self, viewport_height: usize) -> f64 {
        let max = self.max_offset(viewport_height);
        if max == 0 {
            return 1.0;
        }
        self.offset as f64 / max as f64
    }
}

impl<'a> Widget for Viewport<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let height = area.height as usize;
        
        // Get visible lines
        let visible_lines: Vec<Line> = self.content
            .lines
            .into_iter()
            .skip(self.offset)
            .take(height)
            .collect();
        
        // Render visible content
        for (i, line) in visible_lines.into_iter().enumerate() {
            let y = area.y + i as u16;
            if y < area.bottom() {
                line.render(
                    Rect::new(area.x, y, area.width, 1),
                    buf,
                );
            }
        }
        
        // Render scrollbar if content is taller than viewport
        let total_lines = self.content.lines.len();
        if total_lines > height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");
            
            let scrollbar_area = Rect {
                x: area.right().saturating_sub(1),
                y: area.y,
                width: 1,
                height: area.height,
            };
            
            let mut state = ScrollbarState::new(total_lines)
                .position(self.offset)
                .viewport_content_length(height);
            
            scrollbar.render(scrollbar_area, buf, &mut state);
        }
    }
}
```

### 2. Paginator

Handles pagination of file listings.

```rust
// src/ui/components/paginator.rs

use ratatui::{
    text::{Line, Span},
    style::{Color, Style},
};

/// Paginator for navigating pages of items
#[derive(Debug, Clone)]
pub struct Paginator {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
}

impl Paginator {
    pub fn new(per_page: usize) -> Self {
        Self {
            page: 0,
            per_page,
            total_items: 0,
        }
    }
    
    /// Set total items and recalculate pages
    pub fn set_total_items(&mut self, total: usize) {
        self.total_items = total;
        // Ensure current page is valid
        if self.page >= self.total_pages() {
            self.page = self.total_pages().saturating_sub(1);
        }
    }
    
    /// Calculate total pages
    pub fn total_pages(&self) -> usize {
        if self.total_items == 0 {
            1
        } else {
            (self.total_items + self.per_page - 1) / self.per_page
        }
    }
    
    /// Get slice bounds for current page
    pub fn slice_bounds(&self) -> (usize, usize) {
        let start = self.page * self.per_page;
        let end = (start + self.per_page).min(self.total_items);
        (start, end)
    }
    
    /// Number of items on current page
    pub fn items_on_page(&self) -> usize {
        let (start, end) = self.slice_bounds();
        end.saturating_sub(start)
    }
    
    /// Navigate to next page
    pub fn next_page(&mut self) {
        if !self.on_last_page() {
            self.page += 1;
        }
    }
    
    /// Navigate to previous page
    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }
    
    /// Check if on last page
    pub fn on_last_page(&self) -> bool {
        self.page >= self.total_pages().saturating_sub(1)
    }
    
    /// Check if on first page
    pub fn on_first_page(&self) -> bool {
        self.page == 0
    }
    
    /// Render pagination indicator
    pub fn render(&self, width: u16) -> Line {
        if self.total_pages() <= 1 {
            return Line::default();
        }
        
        // Use arabic numerals if too many pages for dots
        if self.total_pages() > width as usize / 2 {
            return Line::from(format!(
                " {} / {} ",
                self.page + 1,
                self.total_pages()
            ));
        }
        
        // Dot pagination
        let dots: Vec<Span> = (0..self.total_pages())
            .map(|i| {
                if i == self.page {
                    Span::styled("•", Style::default().fg(Color::White))
                } else {
                    Span::styled("•", Style::default().fg(Color::DarkGray))
                }
            })
            .collect();
        
        Line::from(dots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pagination() {
        let mut p = Paginator::new(10);
        p.set_total_items(25);
        
        assert_eq!(p.total_pages(), 3);
        assert_eq!(p.slice_bounds(), (0, 10));
        
        p.next_page();
        assert_eq!(p.slice_bounds(), (10, 20));
        
        p.next_page();
        assert_eq!(p.slice_bounds(), (20, 25));
        
        p.next_page(); // Should stay on last page
        assert_eq!(p.page, 2);
    }
}
```

### 3. TextInput

Text input field for filtering/searching.

```rust
// src/ui/components/text_input.rs

use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};

/// Text input component
pub struct TextInput {
    value: String,
    cursor_position: usize,
    prompt: String,
    prompt_style: Style,
    style: Style,
    cursor_style: Style,
    focused: bool,
}

impl TextInput {
    pub fn new(prompt: &str) -> Self {
        Self {
            value: String::new(),
            cursor_position: 0,
            prompt: prompt.to_string(),
            prompt_style: Style::default().fg(Color::Yellow),
            style: Style::default(),
            cursor_style: Style::default().fg(Color::Magenta),
            focused: false,
        }
    }
    
    pub fn value(&self) -> &str {
        &self.value
    }
    
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor_position = self.value.len();
    }
    
    pub fn focus(&mut self) {
        self.focused = true;
    }
    
    pub fn blur(&mut self) {
        self.focused = false;
    }
    
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    
    pub fn cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }
    
    pub fn cursor_start(&mut self) {
        self.cursor_position = 0;
    }
    
    pub fn reset(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }
    
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }
    
    /// Handle a key event, returns true if input was handled
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char(c) => {
                self.insert_char(c);
                true
            }
            KeyCode::Backspace => {
                self.backspace();
                true
            }
            KeyCode::Delete => {
                self.delete();
                true
            }
            KeyCode::Left => {
                self.move_cursor_left();
                true
            }
            KeyCode::Right => {
                self.move_cursor_right();
                true
            }
            KeyCode::Home => {
                self.cursor_start();
                true
            }
            KeyCode::End => {
                self.cursor_end();
                true
            }
            _ => false,
        }
    }
    
    fn insert_char(&mut self, c: char) {
        let pos = self.byte_position(self.cursor_position);
        self.value.insert(pos, c);
        self.cursor_position += 1;
    }
    
    fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let pos = self.byte_position(self.cursor_position);
            self.value.remove(pos);
        }
    }
    
    fn delete(&mut self) {
        if self.cursor_position < self.char_count() {
            let pos = self.byte_position(self.cursor_position);
            self.value.remove(pos);
        }
    }
    
    fn move_cursor_left(&mut self) {
        self.cursor_position = self.cursor_position.saturating_sub(1);
    }
    
    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.char_count() {
            self.cursor_position += 1;
        }
    }
    
    fn char_count(&self) -> usize {
        self.value.chars().count()
    }
    
    fn byte_position(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len())
    }
    
    /// Render the text input
    pub fn render(&self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut spans = vec![
            Span::styled(&self.prompt, self.prompt_style),
        ];
        
        // Get character at cursor for highlighting
        let chars: Vec<char> = self.value.chars().collect();
        
        // Value before cursor
        if self.cursor_position > 0 {
            let before: String = chars[..self.cursor_position].iter().collect();
            spans.push(Span::styled(before, self.style));
        }
        
        // Cursor character (highlighted)
        if self.focused {
            if let Some(&c) = chars.get(self.cursor_position) {
                spans.push(Span::styled(c.to_string(), self.cursor_style));
            } else {
                // Show cursor at end
                spans.push(Span::styled(" ", self.cursor_style));
            }
        } else if let Some(&c) = chars.get(self.cursor_position) {
            spans.push(Span::styled(c.to_string(), self.style));
        }
        
        // Value after cursor
        if self.cursor_position < chars.len() {
            let after: String = chars[self.cursor_position + 1..].iter().collect();
            spans.push(Span::styled(after, self.style));
        }
        
        let paragraph = Paragraph::new(ratatui::text::Line::from(spans));
        paragraph.render(area, buf);
    }
}
```

### 4. Spinner

Loading indicator animation.

```rust
// src/ui/components/spinner.rs

use ratatui::{
    style::Style,
    text::Span,
};

/// Spinner frames for animation
const SPINNER_FRAMES: &[&str] = &[
    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
];

/// Alternative dots spinner
const DOTS_FRAMES: &[&str] = &[
    "⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷",
];

/// Simple ASCII spinner
const ASCII_FRAMES: &[&str] = &[
    "|", "/", "-", "\\",
];

/// Loading spinner component
pub struct Spinner {
    frames: &'static [&'static str],
    current: usize,
    style: Style,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            frames: SPINNER_FRAMES,
            current: 0,
            style: Style::default(),
        }
    }
    
    pub fn dots() -> Self {
        Self {
            frames: DOTS_FRAMES,
            current: 0,
            style: Style::default(),
        }
    }
    
    pub fn ascii() -> Self {
        Self {
            frames: ASCII_FRAMES,
            current: 0,
            style: Style::default(),
        }
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    /// Advance to next frame
    pub fn tick(&mut self) {
        self.current = (self.current + 1) % self.frames.len();
    }
    
    /// Get current frame character
    pub fn frame(&self) -> &'static str {
        self.frames[self.current]
    }
    
    /// Render as styled span
    pub fn render(&self) -> Span {
        Span::styled(self.frame(), self.style)
    }
    
    /// Render with loading text
    pub fn render_with_text(&self, text: &str) -> ratatui::text::Line {
        ratatui::text::Line::from(vec![
            self.render(),
            Span::raw(" "),
            Span::raw(text),
        ])
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}
```

### 5. StatusBar

Status bar at the bottom of the UI.

```rust
// src/ui/components/status_bar.rs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// Status bar configuration
pub struct StatusBar<'a> {
    /// Left side content (logo/title)
    left: Option<Line<'a>>,
    /// Center content (file name/message)
    center: Option<Line<'a>>,
    /// Right side content (scroll position)
    right: Option<Line<'a>>,
    /// Help indicator
    help: bool,
    /// Style
    style: Style,
    /// Message style (for status messages)
    message_style: Style,
}

impl<'a> StatusBar<'a> {
    pub fn new() -> Self {
        Self {
            left: None,
            center: None,
            right: None,
            help: true,
            style: Style::default()
                .fg(Color::Rgb(101, 101, 101))
                .bg(Color::Rgb(36, 36, 36)),
            message_style: Style::default()
                .fg(Color::Rgb(137, 240, 203))
                .bg(Color::Rgb(28, 135, 96)),
        }
    }
    
    pub fn left(mut self, left: Line<'a>) -> Self {
        self.left = Some(left);
        self
    }
    
    pub fn center(mut self, center: Line<'a>) -> Self {
        self.center = Some(center);
        self
    }
    
    pub fn right(mut self, right: Line<'a>) -> Self {
        self.right = Some(right);
        self
    }
    
    pub fn show_help(mut self, show: bool) -> Self {
        self.help = show;
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    /// Create from document info
    pub fn for_document(
        filename: &str,
        scroll_percent: f64,
        show_message: bool,
    ) -> Self {
        let mut bar = Self::new();
        
        // Logo
        bar.left = Some(Line::from(
            Span::styled(" Glow ", Style::default()
                .fg(Color::Rgb(236, 253, 101))
                .bg(Color::Rgb(238, 111, 248))
                .bold())
        ));
        
        // Filename or message
        if show_message {
            bar.center = Some(Line::from(
                Span::styled(filename, bar.message_style)
            ));
        } else {
            bar.center = Some(Line::from(
                Span::styled(filename, bar.style)
            ));
        }
        
        // Scroll position
        let percent = (scroll_percent * 100.0).round() as i32;
        bar.right = Some(Line::from(
            Span::styled(format!(" {:3}% ", percent), bar.style)
        ));
        
        bar
    }
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Fill background
        for x in 0..area.width {
            for y in 0..area.height {
                buf[(area.x + x, area.y + y)]
                    .set_style(self.style);
            }
        }
        
        let mut x = area.x;
        
        // Left content
        if let Some(ref left) = self.left {
            let width = left.width() as u16;
            if x + width <= area.right() {
                left.render(
                    Rect::new(x, area.y, width, 1),
                    buf,
                );
                x += width;
            }
        }
        
        // Right content and help (calculate from right edge)
        let right_width: u16 = self.right.as_ref().map(|r| r.width() as u16).unwrap_or(0);
        let help_width: u16 = if self.help { 8 } else { 0 }; // " ? Help "
        
        // Center content (fills remaining space)
        if let Some(ref center) = self.center {
            let available = area.width.saturating_sub(x).saturating_sub(right_width).saturating_sub(help_width);
            if available > 0 {
                // Truncate center if needed
                let truncated = truncate_line(center, available as usize);
                truncated.render(
                    Rect::new(x, area.y, available, 1),
                    buf,
                );
            }
        }
        
        // Right content
        if let Some(ref right) = self.right {
            let rx = area.right().saturating_sub(right_width).saturating_sub(help_width);
            right.render(
                Rect::new(rx, area.y, right_width, 1),
                buf,
            );
        }
        
        // Help indicator
        if self.help {
            let help_span = Span::styled(" ? Help ", self.style);
            Line::from(help_span).render(
                Rect::new(area.right().saturating_sub(8), area.y, 8, 1),
                buf,
            );
        }
    }
}

fn truncate_line(line: &Line, max_width: usize) -> Line {
    let width = line.width();
    if width <= max_width {
        return line.clone();
    }
    
    let mut spans = Vec::new();
    let mut current_width = 0;
    
    for span in &line.spans {
        let span_width = span.width();
        if current_width + span_width <= max_width {
            spans.push(span.clone());
            current_width += span_width;
        } else {
            let remaining = max_width - current_width;
            if remaining > 0 {
                let content = span.content.chars().take(remaining).collect::<String>();
                spans.push(Span::styled(content, span.style));
            }
            break;
        }
    }
    
    Line::from(spans)
}
```

## Component Usage Examples

### Using the Viewport in Pager

```rust
// In pager view
fn render_pager(f: &mut Frame, pager: &PagerModel, area: Rect) {
    let viewport = Viewport::new(pager.rendered_content.clone())
        .offset(pager.viewport.offset_y);
    
    f.render_widget(viewport, area);
}
```

### Using the StatusBar

```rust
// In document view
fn render_status_bar(f: &mut Frame, pager: &PagerModel, area: Rect) {
    let filename = pager.current_document
        .as_ref()
        .map(|d| d.note.as_str())
        .unwrap_or("");
    
    let status = StatusBar::for_document(
        filename,
        pager.scroll_percent(),
        pager.state == PagerState::StatusMessage,
    );
    
    f.render_widget(status, area);
}
```

### Using TextInput for Filtering

```rust
// In stash model
impl StashModel {
    pub fn start_filtering(&mut self) {
        // Build filter values for all documents
        for md in &mut self.markdowns {
            md.build_filter_value();
        }
        
        self.filter_input.focus();
        self.filter_input.cursor_end();
        self.filter_state = FilterState::Filtering;
    }
    
    pub fn handle_filter_key(&mut self, key: KeyEvent) {
        if self.filter_input.handle_key(key.code) {
            // Input was handled, filter results
            self.filter_markdowns();
        }
    }
}
```
