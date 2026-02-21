# Glow Rust Migration Documentation

## Overview

This document provides a complete implementation guide for migrating the Go-based [Glow](https://github.com/charmbracelet/glow) markdown renderer to Rust using [ratatui](https://github.com/ratatui-org/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm).

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Project Structure](#project-structure)
3. [Dependencies](#dependencies)
4. [Core Types and Models](#core-types-and-models)
5. [Application State Machine](#application-state-machine)
6. [TUI Components](#tui-components)
7. [Markdown Rendering](#markdown-rendering)
8. [File System Operations](#file-system-operations)
9. [Key Bindings](#key-bindings)
10. [Configuration](#configuration)
11. [Implementation Phases](#implementation-phases)

---

## Architecture Overview

### Original Go Architecture (Bubble Tea)

The original Glow uses the Bubble Tea framework which implements The Elm Architecture:

```
┌─────────────────────────────────────────────────────┐
│                    Application                       │
├─────────────────────────────────────────────────────┤
│  ┌─────────┐    ┌─────────┐    ┌─────────────────┐  │
│  │ Model   │◄───│ Update  │◄───│ Messages/Events │  │
│  │ (State) │───►│ (Logic) │───►│ (User Input)    │  │
│  └────┬────┘    └─────────┘    └─────────────────┘  │
│       │                                             │
│       ▼                                             │
│  ┌─────────┐                                        │
│  │  View   │                                        │
│  │ (Render)│                                        │
│  └─────────┘                                        │
└─────────────────────────────────────────────────────┘
```

### Rust Architecture (Ratatui)

The Rust version uses a similar architecture with ratatui's immediate mode rendering:

```rust
// Main event loop pattern
loop {
    // 1. Handle terminal events
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) => app.handle_key(key),
            Event::Resize(w, h) => app.resize(w, h),
            // ...
        }
    }
    
    // 2. Update application state
    app.update();
    
    // 3. Render
    terminal.draw(|f| ui(f, &app))?;
}
```

---

## Project Structure

```
glow-rs/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point, CLI setup
│   ├── app.rs                  # Main application struct and logic
│   ├── config.rs               # Configuration handling
│   ├── error.rs                # Error types
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── render.rs           # Main UI rendering
│   │   ├── pager.rs            # Document viewer component
│   │   ├── stash.rs            # File listing component
│   │   ├── styles.rs           # Color schemes and styling
│   │   ├── help.rs             # Help views
│   │   └── components/
│   │       ├── mod.rs
│   │       ├── viewport.rs     # Scrollable viewport
│   │       ├── paginator.rs    # Pagination component
│   │       ├── text_input.rs   # Text input for filtering
│   │       └── spinner.rs      # Loading spinner
│   ├── markdown/
│   │   ├── mod.rs
│   │   ├── document.rs         # Markdown document type
│   │   ├── renderer.rs         # Markdown to terminal rendering
│   │   └── frontmatter.rs      # YAML frontmatter handling
│   ├── file/
│   │   ├── mod.rs
│   │   ├── finder.rs           # File discovery (like gitcha)
│   │   ├── watcher.rs          # File system watching
│   │   └── ignore.rs           # gitignore pattern handling
│   └── util/
│       ├── mod.rs
│       ├── text.rs             # Text utilities (truncate, indent)
│       └── time.rs             # Relative time formatting
```

---

## Dependencies

### Cargo.toml

```toml
[package]
name = "glow-rs"
version = "0.1.0"
edition = "2021"
description = "Render markdown on the CLI, with pizzazz!"
authors = ["Your Name <you@example.com>"]
license = "MIT"

[dependencies]
# TUI Framework
ratatui = "0.26"
crossterm = { version = "0.27", features = ["events"] }

# CLI
clap = { version = "4.5", features = ["derive", "env"] }

# Markdown Rendering
pulldown-cmark = "0.10"
syntect = "5.2"
textwrap = "0.16"

# Terminal styling
ansi_term = "0.12"

# File system
walkdir = "2.5"
ignore = "0.4"              # For gitignore parsing (like gitcha)
notify = "6.1"              # File watching (like fsnotify)

# Configuration
directories = "5.0"         # XDG paths
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }

# Utilities
fuzzy-matcher = "0.3"       # Fuzzy search (like sahil/fuzzy)
unicode-normalization = "0.1"
time = { version = "0.3", features = ["formatting", "local-offset"] }

# Clipboard
arboard = "3.3"             # Cross-platform clipboard

# Editor
edit = "0.1"                # Open text editor

# Async runtime (optional, for HTTP)
tokio = { version = "1.0", features = ["rt", "rt-multi-thread"] }
reqwest = { version = "0.11", optional = true }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

[features]
default = []
http = ["reqwest"]

[[bin]]
name = "glow"
path = "src/main.rs"
```

---

## Core Types and Models

### Application State

```rust
// src/app.rs

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};

/// Main application state
pub struct App {
    /// Current application state
    pub state: AppState,
    
    /// Terminal dimensions
    pub size: Rect,
    
    /// Configuration
    pub config: Config,
    
    /// Stash (file listing) model
    pub stash: StashModel,
    
    /// Pager (document viewer) model  
    pub pager: PagerModel,
    
    /// Any fatal error that occurred
    pub fatal_error: Option<anyhow::Error>,
    
    /// File finder channel receiver
    pub file_receiver: Option<mpsc::Receiver<FileSearchResult>>,
}

/// Top-level application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Showing the file listing
    ShowStash,
    /// Showing a document in the pager
    ShowDocument,
}

/// Common fields shared across components
pub struct CommonModel {
    pub config: Config,
    pub cwd: PathBuf,
    pub width: u16,
    pub height: u16,
}
```

### Document Model

```rust
// src/markdown/document.rs

use std::path::PathBuf;
use time::OffsetDateTime;

/// Represents a markdown document
pub struct MarkdownDocument {
    /// Full path to the local markdown file
    pub local_path: PathBuf,
    
    /// Value used for filtering (normalized filename)
    pub filter_value: String,
    
    /// The raw markdown content
    pub body: String,
    
    /// Display name (relative path from cwd)
    pub note: String,
    
    /// Last modification time
    pub modified: OffsetDateTime,
}

impl MarkdownDocument {
    /// Generate the filter value from the note
    pub fn build_filter_value(&mut self) {
        self.filter_value = normalize(&self.note);
    }
    
    /// Get relative time string (e.g., "2 hours ago")
    pub fn relative_time(&self) -> String {
        relative_time(self.modified)
    }
}

/// Normalize text for filtering (remove diacritics)
pub fn normalize(input: &str) -> String {
    use unicode_normalization::{nfkd, char::is_combining_mark};
    nfkd(input)
        .filter(|c| !is_combining_mark(*c))
        .collect()
}
```

### Stash Model (File Listing)

```rust
// src/ui/stash.rs

use fuzzy_matcher::FuzzyMatcher;
use std::time::{Duration, Instant};

/// Stash view states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashViewState {
    Ready,
    LoadingDocument,
    ShowingError,
}

/// Filter states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterState {
    Unfiltered,
    Filtering,
    FilterApplied,
}

/// Section types for different document views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKey {
    Documents,
    Filter,
}

/// A section in the stash UI
pub struct Section {
    pub key: SectionKey,
    pub paginator: Paginator,
    pub cursor: usize,
}

/// The file listing model
pub struct StashModel {
    pub common: CommonModel,
    pub error: Option<anyhow::Error>,
    pub spinner: Spinner,
    pub filter_input: TextInput,
    pub view_state: StashViewState,
    pub filter_state: FilterState,
    pub show_full_help: bool,
    pub show_status_message: bool,
    pub status_message: StatusMessage,
    pub status_message_timer: Option<Instant>,
    
    /// Available sections for navigation
    pub sections: Vec<Section>,
    pub section_index: usize,
    
    /// Whether files have been loaded
    pub loaded: bool,
    
    /// All discovered markdown documents
    pub markdowns: Vec<MarkdownDocument>,
    
    /// Filtered documents for display
    pub filtered_markdowns: Vec<MarkdownDocument>,
}

impl StashModel {
    /// Get the currently visible markdowns
    pub fn get_visible_markdowns(&self) -> &[MarkdownDocument] {
        match self.filter_state {
            FilterState::Filtering | _ if self.current_section().key == SectionKey::Filter => {
                &self.filtered_markdowns
            }
            _ => &self.markdowns,
        }
    }
    
    /// Get the currently selected markdown
    pub fn selected_markdown(&self) -> Option<&MarkdownDocument> {
        let index = self.markdown_index();
        let mds = self.get_visible_markdowns();
        mds.get(index)
    }
    
    /// Calculate markdown index from paginator
    pub fn markdown_index(&self) -> usize {
        let paginator = self.paginator();
        paginator.page * paginator.per_page + self.cursor()
    }
    
    /// Open the selected markdown document
    pub fn open_markdown(&mut self, md: &MarkdownDocument) -> Command {
        self.view_state = StashViewState::LoadingDocument;
        Command::LoadDocument(md.local_path.clone())
    }
    
    /// Handle filtering input changes
    pub fn handle_filtering(&mut self, key: KeyEvent) -> Vec<Command> {
        let mut cmds = Vec::new();
        
        match key.code {
            KeyCode::Esc => {
                self.reset_filtering();
            }
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(md) = self.selected_markdown().cloned() {
                    cmds.push(self.open_markdown(&md));
                }
            }
            _ => {
                self.filter_input.handle_key(key);
                cmds.push(Command::FilterMarkdowns);
            }
        }
        
        cmds
    }
    
    /// Filter markdowns using fuzzy matching
    pub fn filter_markdowns(&mut self) {
        if self.filter_input.value().is_empty() {
            self.filtered_markdowns = self.markdowns.clone();
            return;
        }
        
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        let mut ranked: Vec<(i64, &MarkdownDocument)> = self.markdowns
            .iter()
            .filter_map(|md| {
                matcher.fuzzy_match(&md.filter_value, self.filter_input.value())
                    .map(|score| (score, md))
            })
            .collect();
        
        ranked.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        self.filtered_markdowns = ranked
            .into_iter()
            .map(|(_, md)| md.clone())
            .collect();
    }
}
```

### Pager Model (Document Viewer)

```rust
// src/ui/pager.rs

use ratatui::widgets::Paragraph;
use std::time::{Duration, Instant};

/// Pager states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagerState {
    Browse,
    StatusMessage,
}

/// The document viewer model
pub struct PagerModel {
    pub common: CommonModel,
    pub viewport: Viewport,
    pub state: PagerState,
    pub show_help: bool,
    
    /// Current status message
    pub status_message: String,
    pub status_message_timer: Option<Instant>,
    
    /// Currently displayed document
    pub current_document: Option<MarkdownDocument>,
    
    /// Rendered content (with ANSI codes)
    pub rendered_content: Text<'static>,
    
    /// File watcher
    pub watcher: Option<FileWatcher>,
}

impl PagerModel {
    /// Set viewport size
    pub fn set_size(&mut self, width: u16, height: u16) {
        self.viewport.width = width;
        self.viewport.height = height.saturating_sub(STATUS_BAR_HEIGHT);
        
        if self.show_help {
            self.viewport.height = self.viewport.height.saturating_sub(HELP_HEIGHT);
        }
    }
    
    /// Scroll to top of document
    pub fn goto_top(&mut self) {
        self.viewport.offset_y = 0;
    }
    
    /// Scroll to bottom of document
    pub fn goto_bottom(&mut self) {
        let max_offset = self.viewport.content_height.saturating_sub(self.viewport.height);
        self.viewport.offset_y = max_offset;
    }
    
    /// Scroll half page down
    pub fn half_page_down(&mut self) {
        let half = self.viewport.height / 2;
        self.viewport.offset_y = self.viewport.offset_y
            .saturating_add(half as usize)
            .min(self.max_scroll());
    }
    
    /// Scroll half page up
    pub fn half_page_up(&mut self) {
        let half = self.viewport.height / 2;
        self.viewport.offset_y = self.viewport.offset_y.saturating_sub(half as usize);
    }
    
    /// Get scroll percentage (0.0 - 1.0)
    pub fn scroll_percent(&self) -> f64 {
        if self.viewport.content_height == 0 {
            return 0.0;
        }
        let max = self.max_scroll();
        if max == 0 {
            return 1.0;
        }
        self.viewport.offset_y as f64 / max as f64
    }
    
    fn max_scroll(&self) -> usize {
        self.viewport.content_height.saturating_sub(self.viewport.height as usize)
    }
    
    /// Unload current document and return to stash
    pub fn unload(&mut self) {
        self.current_document = None;
        self.rendered_content = Text::default();
        self.viewport.offset_y = 0;
        self.show_help = false;
        self.stop_watcher();
    }
    
    /// Copy content to clipboard
    pub fn copy_to_clipboard(&self) -> anyhow::Result<()> {
        if let Some(doc) = &self.current_document {
            let mut clipboard = arboard::Clipboard::new()?;
            clipboard.set_text(&doc.body)?;
        }
        Ok(())
    }
}

/// Simple viewport for scrolling
pub struct Viewport {
    pub width: u16,
    pub height: u16,
    pub offset_y: usize,
    pub content_height: usize,
}
```

---

## Application State Machine

```rust
// src/app.rs

impl App {
    /// Handle a key event
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Global keys
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(()); // Will trigger quit
            }
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Suspend (SIGTSTP)
                #[cfg(unix)]
                {
                    use std::process::Command;
                    Command::new("kill")
                        .args(["-STOP", &std::process::id().to_string()])
                        .spawn()?;
                }
                return Ok(());
            }
            _ => {}
        }
        
        // State-specific handling
        match self.state {
            AppState::ShowStash => {
                self.handle_stash_key(key)?;
            }
            AppState::ShowDocument => {
                self.handle_pager_key(key)?;
            }
        }
        
        Ok(())
    }
    
    /// Handle keys in the stash view
    fn handle_stash_key(&mut self, key: KeyEvent) -> Result<()> {
        // If filtering, pass to filter handler
        if self.stash.filter_state == FilterState::Filtering {
            let cmds = self.stash.handle_filtering(key);
            self.execute_commands(cmds)?;
            return Ok(());
        }
        
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                // Quit
                self.should_quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.stash.move_cursor_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.stash.move_cursor_up();
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.stash.paginator_mut().page = 0;
                self.stash.set_cursor(0);
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.stash.paginator_mut().page = 
                    self.stash.paginator().total_pages.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(md) = self.stash.selected_markdown().cloned() {
                    self.load_document(&md.local_path)?;
                    self.state = AppState::ShowDocument;
                }
            }
            KeyCode::Char('/') => {
                self.stash.start_filtering();
            }
            KeyCode::Char('e') => {
                if let Some(md) = self.stash.selected_markdown() {
                    self.open_editor(&md.local_path, 0)?;
                }
            }
            KeyCode::Char('r') => {
                self.refresh_files()?;
            }
            KeyCode::Char('?') => {
                self.stash.show_full_help = !self.stash.show_full_help;
                self.stash.update_pagination();
            }
            KeyCode::Tab | KeyCode::Char('L') => {
                self.stash.next_section();
            }
            KeyCode::BackTab | KeyCode::Char('H') => {
                self.stash.prev_section();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle keys in the pager view
    fn handle_pager_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                self.unload_document()?;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.pager.viewport.scroll_down(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.pager.viewport.scroll_up(1);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.pager.goto_top();
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.pager.goto_bottom();
            }
            KeyCode::Char('d') => {
                self.pager.half_page_down();
            }
            KeyCode::Char('u') => {
                self.pager.half_page_up();
            }
            KeyCode::Char('e') => {
                if let Some(doc) = &self.pager.current_document {
                    let line = self.pager.approximate_line_number();
                    self.open_editor(&doc.local_path, line)?;
                }
            }
            KeyCode::Char('c') => {
                self.pager.copy_to_clipboard()?;
                self.pager.show_status_message("Copied contents");
            }
            KeyCode::Char('r') => {
                if let Some(doc) = &self.pager.current_document.clone() {
                    self.load_document(&doc.local_path)?;
                }
            }
            KeyCode::Char('?') => {
                self.pager.toggle_help();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Load a document into the pager
    fn load_document(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let content = remove_frontmatter(&content);
        
        let metadata = std::fs::metadata(path)?;
        let modified: OffsetDateTime = metadata.modified()?.into();
        
        let cwd = std::env::current_dir()?;
        let note = path.strip_prefix(&cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        
        self.pager.current_document = Some(MarkdownDocument {
            local_path: path.to_path_buf(),
            body: content.clone(),
            note,
            modified,
            filter_value: String::new(),
        });
        
        // Render with glamour-like styling
        self.pager.rendered_content = render_markdown(
            &content,
            &self.config,
            self.pager.viewport.width,
        )?;
        
        self.state = AppState::ShowDocument;
        
        // Start watching file for changes
        self.pager.watch_file(path)?;
        
        Ok(())
    }
    
    /// Unload the current document and return to stash
    fn unload_document(&mut self) -> Result<()> {
        self.pager.unload();
        self.state = AppState::ShowStash;
        self.stash.view_state = StashViewState::Ready;
        Ok(())
    }
}
```

---

## TUI Components

### Viewport Widget

```rust
// src/ui/components/viewport.rs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{Widget, Scrollbar, ScrollbarState, ScrollbarOrientation},
};

/// A scrollable viewport widget
pub struct Viewport<'a> {
    content: Text<'a>,
    scroll: usize,
}

impl<'a> Viewport<'a> {
    pub fn new(content: Text<'a>) -> Self {
        Self { content, scroll: 0 }
    }
    
    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll = offset;
        self
    }
}

impl<'a> Widget for Viewport<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let height = area.height as usize;
        let lines: Vec<Line> = self.content
            .lines
            .into_iter()
            .skip(self.scroll)
            .take(height)
            .collect();
        
        let visible = Text::from(lines);
        Paragraph::new(visible).render(area, buf);
        
        // Render scrollbar if needed
        let total_lines = self.content.lines.len();
        if total_lines > height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut state = ScrollbarState::new(total_lines)
                .position(self.scroll);
            scrollbar.render(area, buf, &mut state);
        }
    }
}
```

### Paginator Component

```rust
// src/ui/components/paginator.rs

/// Pagination state
#[derive(Debug, Clone)]
pub struct Paginator {
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub total_items: usize,
}

impl Paginator {
    pub fn new(per_page: usize) -> Self {
        Self {
            page: 0,
            per_page,
            total_pages: 1,
            total_items: 0,
        }
    }
    
    /// Set total items and recalculate pages
    pub fn set_total_items(&mut self, total: usize) {
        self.total_items = total;
        self.total_pages = if total == 0 {
            1
        } else {
            (total + self.per_page - 1) / self.per_page
        };
        
        // Ensure current page is valid
        if self.page >= self.total_pages {
            self.page = self.total_pages.saturating_sub(1);
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
        end - start
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
    
    pub fn on_last_page(&self) -> bool {
        self.page >= self.total_pages.saturating_sub(1)
    }
    
    /// Render pagination dots
    pub fn render(&self, width: u16) -> Line {
        if self.total_pages <= 1 {
            return Line::default();
        }
        
        // If too many pages, show arabic numerals
        if self.total_pages > width as usize / 2 {
            return Line::from(format!("{} / {}", self.page + 1, self.total_pages));
        }
        
        // Show dot pagination
        let dots: Vec<Span> = (0..self.total_pages)
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
```

### TextInput Component

```rust
// src/ui/components/text_input.rs

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Text input field for filtering
pub struct TextInput {
    value: String,
    cursor_position: usize,
    prompt: String,
    prompt_style: Style,
    cursor_style: Style,
    focused: bool,
    width: usize,
}

impl TextInput {
    pub fn new(prompt: &str) -> Self {
        Self {
            value: String::new(),
            cursor_position: 0,
            prompt: prompt.to_string(),
            prompt_style: Style::default().fg(Color::Yellow),
            cursor_style: Style::default().fg(Color::Magenta),
            focused: false,
            width: 0,
        }
    }
    
    pub fn value(&self) -> &str {
        &self.value
    }
    
    pub fn focus(&mut self) {
        self.focused = true;
    }
    
    pub fn blur(&mut self) {
        self.focused = false;
    }
    
    pub fn cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }
    
    pub fn reset(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }
    
    /// Handle a key event
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                self.value.insert(self.cursor_position, c);
                self.cursor_position += c.len_utf8();
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.value.remove(self.cursor_position);
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.value.len() {
                    self.value.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
            }
            KeyCode::Right => {
                if self.cursor_position < self.value.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.value.len();
            }
            _ => {}
        }
    }
    
    /// Render the text input
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(&self.prompt, self.prompt_style),
        ];
        
        // Value before cursor
        if self.cursor_position > 0 {
            spans.push(Span::raw(&self.value[..self.cursor_position]));
        }
        
        // Cursor
        if self.focused {
            let cursor_char = self.value
                .chars()
                .nth(self.cursor_position)
                .unwrap_or(' ');
            spans.push(Span::styled(
                cursor_char.to_string(),
                self.cursor_style,
            ));
        }
        
        // Value after cursor
        if self.cursor_position < self.value.len() {
            let after_cursor = &self.value[self.cursor_position
                + self.value[self.cursor_position..].chars().next().map(|c| c.len_utf8()).unwrap_or(0)..];
            spans.push(Span::raw(after_cursor));
        }
        
        let paragraph = Paragraph::new(Line::from(spans));
        f.render_widget(paragraph, area);
    }
}
```

### Spinner Component

```rust
// src/ui/components/spinner.rs

/// Loading spinner animation
pub struct Spinner {
    frames: Vec<&'static str>,
    current: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current: 0,
        }
    }
    
    /// Advance to next frame
    pub fn tick(&mut self) {
        self.current = (self.current + 1) % self.frames.len();
    }
    
    /// Get current frame
    pub fn frame(&self) -> &str {
        self.frames[self.current]
    }
    
    /// Render as styled span
    pub fn render(&self, style: Style) -> Span {
        Span::styled(self.frame(), style)
    }
}
```

---

## Markdown Rendering

```rust
// src/markdown/renderer.rs

use pulldown_cmark::{Parser, Event, Tag, HeadingLevel, CodeBlockKind};
use syntect::{parsing::SyntaxSet, highlighting::ThemeSet, html::highlighted_html_for_string};
use textwrap::wrap;

/// Render markdown to terminal-formatted Text
pub fn render_markdown(
    markdown: &str,
    config: &Config,
    width: u16,
) -> Result<Text<'static>> {
    let mut renderer = MarkdownRenderer::new(config, width);
    renderer.render(markdown)
}

/// Markdown to terminal renderer
pub struct MarkdownRenderer {
    config: Config,
    width: u16,
    line_number_width: usize,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl MarkdownRenderer {
    pub fn new(config: &Config, width: u16) -> Self {
        Self {
            config: config.clone(),
            width,
            line_number_width: 4,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }
    
    pub fn render(&mut self, markdown: &str) -> Result<Text<'static>> {
        let parser = Parser::new(markdown);
        let mut lines = Vec::new();
        let mut current_line_spans = Vec::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();
        
        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading(level, _, _) => {
                            self.flush_line(&mut current_line_spans, &mut lines);
                            current_line_spans.push(self.style_heading(level));
                        }
                        Tag::Paragraph => {
                            // Just continue collecting
                        }
                        Tag::CodeBlock(kind) => {
                            in_code_block = true;
                            if let CodeBlockKind::Fenced(lang) = kind {
                                code_block_lang = lang.to_string();
                            }
                        }
                        Tag::Strong => {
                            current_line_spans.push(Span::styled(
                                "",
                                Style::default().add_modifier(Modifier::BOLD),
                            ));
                        }
                        Tag::Emphasis => {
                            current_line_spans.push(Span::styled(
                                "",
                                Style::default().add_modifier(Modifier::ITALIC),
                            ));
                        }
                        Tag::Link(_, url, _) => {
                            current_line_spans.push(Span::styled(
                                "",
                                Style::default().fg(Color::Cyan).underline(),
                            ));
                        }
                        _ => {}
                    }
                }
                
                Event::End(tag) => {
                    match tag {
                        Tag::Heading(_, _, _) | Tag::Paragraph => {
                            self.flush_line(&mut current_line_spans, &mut lines);
                        }
                        Tag::CodeBlock(_) => {
                            in_code_block = false;
                            let highlighted = self.highlight_code(
                                &code_block_content,
                                &code_block_lang,
                            );
                            for line in highlighted.lines() {
                                lines.push(Line::from(Span::raw(line)));
                            }
                            code_block_content.clear();
                            code_block_lang.clear();
                        }
                        _ => {}
                    }
                }
                
                Event::Text(text) => {
                    if in_code_block {
                        code_block_content.push_str(&text);
                    } else {
                        // Apply word wrapping
                        let wrapped = wrap(&text, self.width as usize);
                        for (i, line) in wrapped.into_iter().enumerate() {
                            if i > 0 {
                                self.flush_line(&mut current_line_spans, &mut lines);
                            }
                            current_line_spans.push(Span::raw(line.to_string()));
                        }
                    }
                }
                
                Event::Code(code) => {
                    // Inline code
                    current_line_spans.push(Span::styled(
                        code.to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(Color::DarkGray),
                    ));
                }
                
                Event::SoftBreak | Event::HardBreak => {
                    self.flush_line(&mut current_line_spans, &mut lines);
                }
                
                Event::List(_) => {
                    current_line_spans.push(Span::raw("  • "));
                }
                
                _ => {}
            }
        }
        
        self.flush_line(&mut current_line_spans, &mut lines);
        
        // Add line numbers if configured
        if self.config.show_line_numbers {
            lines = self.add_line_numbers(lines);
        }
        
        Ok(Text::from(lines))
    }
    
    fn flush_line(&self, spans: &mut Vec<Span>, lines: &mut Vec<Line>) {
        if !spans.is_empty() {
            lines.push(Line::from(spans.clone()));
            spans.clear();
        }
    }
    
    fn style_heading(&self, level: HeadingLevel) -> Span {
        let (prefix, style) = match level {
            HeadingLevel::H1 => ("# ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            HeadingLevel::H2 => ("## ", Style::default().fg(Color::Green)),
            HeadingLevel::H3 => ("### ", Style::default().fg(Color::Cyan)),
            HeadingLevel::H4 => ("#### ", Style::default().fg(Color::Cyan)),
            HeadingLevel::H5 => ("##### ", Style::default().fg(Color::Blue)),
            HeadingLevel::H6 => ("###### ", Style::default().fg(Color::Blue)),
        };
        Span::styled(prefix, style)
    }
    
    fn highlight_code(&self, code: &str, lang: &str) -> String {
        // Use syntect for syntax highlighting
        // Convert to ANSI terminal output
        // This is a simplified version - a full implementation would
        // use syntect's terminal output functionality
        code.to_string()
    }
    
    fn add_line_numbers(&self, lines: Vec<Line>) -> Vec<Line> {
        lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = Span::styled(
                    format!("{:4}", i + 1),
                    Style::default().fg(Color::DarkGray),
                );
                let mut spans = vec![line_num];
                spans.extend(line.spans);
                Line::from(spans)
            })
            .collect()
    }
}
```

---

## File System Operations

### File Finder

```rust
// src/file/finder.rs

use walkdir::WalkDir;
use ignore::gitignore::GitignoreBuilder;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Markdown file extensions to search for
const MARKDOWN_EXTENSIONS: &[&str] = &[
    "md", "mdown", "mkdn", "mkd", "markdown",
];

/// Result from file search
pub struct FileSearchResult {
    pub path: PathBuf,
    pub metadata: std::fs::Metadata,
}

/// Find markdown files in a directory
pub fn find_markdown_files(
    dir: &Path,
    show_all: bool,
    ignore_patterns: &[String],
) -> mpsc::Receiver<FileSearchResult> {
    let (tx, rx) = mpsc::channel();
    let dir = dir.to_path_buf();
    
    std::thread::spawn(move || {
        // Build gitignore matcher if not showing all files
        let gitignore = if !show_all {
            build_gitignore(&dir, ignore_patterns)
        } else {
            None
        };
        
        for entry in WalkDir::new(&dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip if ignored
            if let Some(ref gi) = gitignore {
                if gi.matched(path, false).is_ignore() {
                    continue;
                }
            }
            
            // Check if it's a markdown file
            if is_markdown_file(path) {
                if let Ok(metadata) = entry.metadata() {
                    let _ = tx.send(FileSearchResult {
                        path: path.to_path_buf(),
                        metadata,
                    });
                }
            }
        }
    });
    
    rx
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| MARKDOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn build_gitignore(dir: &Path, extra_patterns: &[String]) -> Option<Gitignore> {
    let mut builder = GitignoreBuilder::new(dir);
    
    // Add standard ignore patterns
    for pattern in &[".git", "node_modules", "target", "vendor"] {
        builder.add_line(None, pattern).ok()?;
    }
    
    // Add extra patterns
    for pattern in extra_patterns {
        builder.add_line(None, pattern).ok()?;
    }
    
    builder.build().ok()
}
```

### File Watcher

```rust
// src/file/watcher.rs

use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, EventKind};
use std::path::PathBuf;
use std::sync::mpsc;

/// File system watcher for auto-reload
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    watched_path: PathBuf,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(tx: mpsc::Sender<WatchEvent>) -> Result<Self> {
        let tx_clone = tx.clone();
        let watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    let _ = tx_clone.send(WatchEvent::Modified(event.paths));
                }
            }
        })?;
        
        Ok(Self {
            watcher,
            watched_path: PathBuf::new(),
        })
    }
    
    /// Start watching a file's directory
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        let dir = path.parent().ok_or_else(|| anyhow::anyhow!("No parent directory"))?;
        self.watcher.watch(dir, RecursiveMode::NonRecursive)?;
        self.watched_path = path.to_path_buf();
        Ok(())
    }
    
    /// Stop watching
    pub fn unwatch(&mut self) -> Result<()> {
        if let Some(dir) = self.watched_path.parent() {
            self.watcher.unwatch(dir)?;
        }
        self.watched_path = PathBuf::new();
        Ok(())
    }
}

#[derive(Debug)]
pub enum WatchEvent {
    Modified(Vec<PathBuf>),
}
```

---

## Key Bindings

```rust
// src/ui/keys.rs

/// Global key bindings
pub const QUIT: KeyBinding = KeyBinding::Char('q');
pub const QUIT_ALT: KeyBinding = KeyBinding::Esc;
pub const SUSPEND: KeyBinding = KeyBinding::Ctrl('z');
pub const CANCEL: KeyBinding = KeyBinding::Ctrl('c');

/// Navigation keys
pub const UP: &[KeyBinding] = &[KeyBinding::Char('k'), KeyBinding::Up];
pub const DOWN: &[KeyBinding] = &[KeyBinding::Char('j'), KeyBinding::Down];
pub const PAGE_UP: &[KeyBinding] = &[KeyBinding::Char('b'), KeyBinding::PageUp];
pub const PAGE_DOWN: &[KeyBinding] = &[KeyBinding::Char('f'), KeyBinding::PageDown];
pub const HALF_UP: KeyBinding = KeyBinding::Char('u');
pub const HALF_DOWN: KeyBinding = KeyBinding::Char('d');
pub const HOME: &[KeyBinding] = &[KeyBinding::Char('g'), KeyBinding::Home];
pub const END: &[KeyBinding] = &[KeyBinding::Char('G'), KeyBinding::End];

/// Action keys
pub const OPEN: KeyBinding = KeyBinding::Enter;
pub const EDIT: KeyBinding = KeyBinding::Char('e');
pub const COPY: KeyBinding = KeyBinding::Char('c');
pub const RELOAD: KeyBinding = KeyBinding::Char('r');
pub const FILTER: KeyBinding = KeyBinding::Char('/');
pub const HELP: KeyBinding = KeyBinding::Char('?');
pub const BACK: &[KeyBinding] = &[KeyBinding::Esc, KeyBinding::Char('h'), KeyBinding::Left];

/// Section navigation
pub const NEXT_SECTION: &[KeyBinding] = &[KeyBinding::Tab, KeyBinding::Char('L')];
pub const PREV_SECTION: &[KeyBinding] = &[KeyBinding::BackTab, KeyBinding::Char('H')];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyBinding {
    Char(char),
    Ctrl(char),
    Alt(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

impl KeyBinding {
    pub fn matches(&self, key: &KeyEvent) -> bool {
        match self {
            KeyBinding::Char(c) => key.code == KeyCode::Char(*c),
            KeyBinding::Ctrl(c) => {
                key.modifiers.contains(KeyModifiers::CONTROL) 
                    && key.code == KeyCode::Char(*c)
            }
            KeyBinding::Alt(c) => {
                key.modifiers.contains(KeyModifiers::ALT)
                    && key.code == KeyCode::Char(*c)
            }
            KeyBinding::Enter => key.code == KeyCode::Enter,
            KeyBinding::Esc => key.code == KeyCode::Esc,
            KeyBinding::Tab => key.code == KeyCode::Tab,
            KeyBinding::BackTab => key.code == KeyCode::BackTab,
            KeyBinding::Up => key.code == KeyCode::Up,
            KeyBinding::Down => key.code == KeyCode::Down,
            KeyBinding::Left => key.code == KeyCode::Left,
            KeyBinding::Right => key.code == KeyCode::Right,
            KeyBinding::Home => key.code == KeyCode::Home,
            KeyBinding::End => key.code == KeyCode::End,
            KeyBinding::PageUp => key.code == KeyCode::PageUp,
            KeyBinding::PageDown => key.code == KeyCode::PageDown,
        }
    }
}

pub fn matches_any(key: &KeyEvent, bindings: &[KeyBinding]) -> bool {
    bindings.iter().any(|b| b.matches(key))
}
```

---

## Configuration

```rust
// src/config.rs

use serde::{Deserialize, Serialize};
use directories::ProjectDirs;
use std::path::PathBuf;

/// TUI-specific configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Show system files and directories
    #[serde(default)]
    pub show_all_files: bool,
    
    /// Show line numbers in pager
    #[serde(default)]
    pub show_line_numbers: bool,
    
    /// Maximum width for glamour rendering
    #[serde(default = "default_glamour_max_width")]
    pub glamour_max_width: u16,
    
    /// Style name or path to style JSON
    #[serde(default = "default_style")]
    pub glamour_style: String,
    
    /// Enable mouse support
    #[serde(default)]
    pub enable_mouse: bool,
    
    /// Preserve newlines in output
    #[serde(default)]
    pub preserve_newlines: bool,
    
    /// Enable glamour rendering
    #[serde(default = "default_true")]
    pub glamour_enabled: bool,
    
    /// High performance pager (use sync scrolling)
    #[serde(default = "default_true")]
    pub high_performance_pager: bool,
    
    /// Working directory or file path
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

fn default_glamour_max_width() -> u16 { 120 }
fn default_style() -> String { "auto".to_string() }
fn default_true() -> bool { true }

impl Default for Config {
    fn default() -> Self {
        Self {
            show_all_files: false,
            show_line_numbers: false,
            glamour_max_width: 120,
            glamour_style: "auto".to_string(),
            enable_mouse: false,
            preserve_newlines: false,
            glamour_enabled: true,
            high_performance_pager: true,
            path: None,
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "charmbracelet", "glow")
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        
        let config_path = dirs.config_dir().join("glow.toml");
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }
    
    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let dirs = ProjectDirs::from("com", "charmbracelet", "glow")
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        
        let config_dir = dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        
        let config_path = config_dir.join("glow.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        
        Ok(())
    }
}
```

---

## Styles

```rust
// src/ui/styles.rs

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

/// Color palette (adaptive for light/dark terminals)
pub struct Colors {
    // Base colors
    pub normal_dim: Color,
    pub gray: Color,
    pub mid_gray: Color,
    pub dark_gray: Color,
    pub bright_gray: Color,
    
    // Accent colors
    pub green: Color,
    pub semi_dim_green: Color,
    pub dim_green: Color,
    pub fuchsia: Color,
    pub dim_fuchsia: Color,
    pub dull_fuchsia: Color,
    pub red: Color,
    
    // UI colors
    pub cream: Color,
    pub yellow_green: Color,
}

impl Colors {
    /// Get colors for dark terminal
    pub fn dark() -> Self {
        Self {
            normal_dim: Color::Rgb(119, 119, 119),
            gray: Color::Rgb(98, 98, 98),
            mid_gray: Color::Rgb(74, 74, 74),
            dark_gray: Color::Rgb(60, 60, 60),
            bright_gray: Color::Rgb(151, 151, 151),
            green: Color::Rgb(4, 181, 117),
            semi_dim_green: Color::Rgb(3, 107, 70),
            dim_green: Color::Rgb(11, 81, 55),
            fuchsia: Color::Rgb(238, 111, 248),
            dim_fuchsia: Color::Rgb(153, 81, 158),
            dull_fuchsia: Color::Rgb(173, 88, 180),
            red: Color::Rgb(237, 86, 122),
            cream: Color::Rgb(255, 253, 245),
            yellow_green: Color::Rgb(236, 253, 101),
        }
    }
    
    /// Get colors for light terminal
    pub fn light() -> Self {
        Self {
            normal_dim: Color::Rgb(164, 159, 165),
            gray: Color::Rgb(144, 144, 144),
            mid_gray: Color::Rgb(178, 178, 178),
            dark_gray: Color::Rgb(221, 218, 218),
            bright_gray: Color::Rgb(132, 122, 133),
            green: Color::Rgb(4, 181, 117),
            semi_dim_green: Color::Rgb(53, 215, 156),
            dim_green: Color::Rgb(114, 210, 176),
            fuchsia: Color::Rgb(238, 111, 248),
            dim_fuchsia: Color::Rgb(241, 168, 255),
            dull_fuchsia: Color::Rgb(247, 147, 255),
            red: Color::Rgb(255, 70, 114),
            cream: Color::Rgb(255, 253, 245),
            yellow_green: Color::Rgb(4, 181, 117),
        }
    }
}

/// Helper style functions
pub fn gray_fg<'a>(s: impl Into<String>) -> Span<'a> {
    Span::styled(s.into(), Style::default().fg(Colors::dark().gray))
}

pub fn green_fg<'a>(s: impl Into<String>) -> Span<'a> {
    Span::styled(s.into(), Style::default().fg(Colors::dark().green))
}

pub fn fuchsia_fg<'a>(s: impl Into<String>) -> Span<'a> {
    Span::styled(s.into(), Style::default().fg(Colors::dark().fuchsia))
}

pub fn red_fg<'a>(s: impl Into<String>) -> Span<'a> {
    Span::styled(s.into(), Style::default().fg(Colors::dark().red))
}

/// Logo style
pub fn logo_style() -> Style {
    Style::default()
        .fg(Color::Rgb(236, 253, 101))
        .bg(Color::Rgb(238, 111, 248))
        .add_modifier(Modifier::BOLD)
}

/// Status bar styles
pub fn status_bar_style() -> Style {
    Style::default()
        .fg(Color::Rgb(101, 101, 101))
        .bg(Color::Rgb(36, 36, 36))
}

/// Error title style
pub fn error_title_style() -> Style {
    Style::default()
        .fg(Color::Rgb(255, 253, 245))
        .bg(Color::Rgb(237, 86, 122))
}
```

---

## Main Entry Point

```rust
// src/main.rs

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

mod app;
mod config;
mod error;
mod file;
mod markdown;
mod ui;
mod util;

use app::App;
use config::Config;

#[derive(Parser, Debug)]
#[command(name = "glow")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File or directory to render
    #[arg(value_name = "SOURCE")]
    source: Option<PathBuf>,
    
    /// Display with pager
    #[arg(short, long)]
    pager: bool,
    
    /// Display with TUI
    #[arg(short, long)]
    tui: bool,
    
    /// Style name or JSON path
    #[arg(short, long, default_value = "auto")]
    style: String,
    
    /// Word-wrap at width (0 to disable)
    #[arg(short, long, default_value = "0")]
    width: u16,
    
    /// Show system files and directories
    #[arg(short = 'a', long)]
    all: bool,
    
    /// Show line numbers
    #[arg(short = 'l', long)]
    line_numbers: bool,
    
    /// Preserve newlines
    #[arg(short = 'n', long)]
    preserve_new_lines: bool,
    
    /// Enable mouse wheel
    #[arg(short, long, hide = true)]
    mouse: bool,
    
    /// Config file path
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Parse arguments
    let args = Args::parse();
    
    // Load configuration
    let mut config = Config::load()?;
    
    // Override config with CLI args
    if args.all { config.show_all_files = true; }
    if args.line_numbers { config.show_line_numbers = true; }
    if args.preserve_new_lines { config.preserve_newlines = true; }
    if args.mouse { config.enable_mouse = true; }
    if args.width > 0 { config.glamour_max_width = args.width; }
    if args.style != "auto" { config.glamour_style = args.style; }
    config.path = args.source;
    
    // Check if we should run TUI or CLI mode
    if args.tui || args.source.is_none() {
        run_tui(config)?;
    } else {
        run_cli(config, args.source.unwrap())?;
    }
    
    Ok(())
}

fn run_tui(config: Config) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    if config.enable_mouse {
        execute!(stdout, EnableMouseCapture)?;
    }
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app
    let mut app = App::new(config)?;
    
    // Main event loop
    let res = run_app(&mut terminal, &mut app);
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    if let Err(err) = res {
        eprintln!("Error: {err:?}");
        return Err(err);
    }
    
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> anyhow::Result<()> {
    loop {
        // Draw UI
        terminal.draw(|f| ui::render(f, app))?;
        
        // Handle events with timeout
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        app.handle_key(key)?;
                        
                        if app.should_quit {
                            return Ok(());
                        }
                    }
                }
                Event::Resize(w, h) => {
                    app.resize(w, h);
                }
                Event::Mouse(mouse) => {
                    if app.config.enable_mouse {
                        app.handle_mouse(mouse)?;
                    }
                }
                _ => {}
            }
        }
        
        // Check for file watcher events
        app.check_file_watcher()?;
        
        // Check for file search results
        app.check_file_search()?;
        
        // Update timers
        app.update_timers();
    }
}

fn run_cli(config: Config, source: PathBuf) -> anyhow::Result<()> {
    let content = if source == PathBuf::from("-") {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else if source.is_dir() {
        // Find README in directory
        find_readme(&source)?
    } else {
        // Read file
        std::fs::read_to_string(&source)?
    };
    
    // Remove frontmatter
    let content = markdown::frontmatter::remove(&content);
    
    // Render markdown
    let rendered = markdown::render::render_to_string(
        &content,
        &config,
        config.glamour_max_width,
    )?;
    
    // Output
    println!("{rendered}");
    
    Ok(())
}

fn find_readme(dir: &Path) -> anyhow::Result<String> {
    let readme_names = ["README.md", "README", "Readme.md", "readme.md"];
    
    for name in &readme_names {
        let path = dir.join(name);
        if path.exists() {
            return Ok(std::fs::read_to_string(&path)?);
        }
    }
    
    anyhow::bail!("No README found in directory")
}
```

---

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1)
- [ ] Setup project structure
- [ ] Implement configuration loading
- [ ] Create basic TUI event loop
- [ ] Implement terminal setup/teardown

### Phase 2: File System (Week 2)
- [ ] Implement file finder with gitignore support
- [ ] Add file watching for auto-reload
- [ ] Create markdown document type
- [ ] Handle frontmatter removal

### Phase 3: Stash UI (Week 3)
- [ ] Implement file listing view
- [ ] Add pagination
- [ ] Implement fuzzy filtering
- [ ] Add keyboard navigation

### Phase 4: Pager UI (Week 4)
- [ ] Implement document viewer
- [ ] Add scrolling support
- [ ] Implement markdown rendering with pulldown-cmark
- [ ] Add syntax highlighting with syntect

### Phase 5: Polish (Week 5)
- [ ] Implement all key bindings
- [ ] Add help views
- [ ] Implement clipboard support
- [ ] Add editor integration
- [ ] Error handling and edge cases

### Phase 6: Testing & Documentation (Week 6)
- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] Performance optimization

---

## Key Differences from Go Version

| Go (Original) | Rust (Migration) |
|---------------|------------------|
| Bubble Tea framework | ratatui + crossterm |
| glamour | pulldown-cmark + syntect |
| gitcha | walkdir + ignore crate |
| fsnotify | notify crate |
| lipgloss | ratatui::style |
| cobra | clap |
| viper | config/toml crate |
| Go channels | mpsc channels |
| goroutines | std::thread |

---

## References

- [Glow Original Repository](https://github.com/charmbracelet/glow)
- [ratatui Documentation](https://docs.rs/ratatui)
- [crossterm Documentation](https://docs.rs/crossterm)
- [pulldown-cmark](https://docs.rs/pulldown-cmark)
- [Bubble Tea Architecture](https://github.com/charmbracelet/bubbletea)
