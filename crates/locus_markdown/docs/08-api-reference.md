# Glow Rust - API Reference

This document provides a comprehensive API reference for the glow-rs library.

## Core Modules

### `glow_rs::app`

Main application module.

```rust
use glow_rs::app::{App, AppState};
```

#### `App`

The main application struct.

```rust
pub struct App {
    pub state: AppState,
    pub config: Config,
    pub stash: StashModel,
    pub pager: PagerModel,
    pub should_quit: bool,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Result<Self>;
    
    /// Handle a key event
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()>;
    
    /// Handle a mouse event
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()>;
    
    /// Handle terminal resize
    pub fn resize(&mut self, width: u16, height: u16);
    
    /// Update application state
    pub fn update(&mut self);
    
    /// Render the application
    pub fn render(&self, f: &mut Frame);
    
    /// Check for pending file search results
    pub fn check_file_search(&mut self);
    
    /// Check for file watcher events
    pub fn check_file_watcher(&mut self) -> Result<()>;
}
```

#### `AppState`

```rust
pub enum AppState {
    ShowStash,
    ShowDocument,
}
```

---

### `glow_rs::config`

Configuration management.

```rust
use glow_rs::config::{Config, RenderConfig};
```

#### `Config`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub show_all_files: bool,
    pub show_line_numbers: bool,
    pub glamour_max_width: u16,
    pub glamour_style: String,
    pub enable_mouse: bool,
    pub preserve_newlines: bool,
    pub glamour_enabled: bool,
    pub high_performance_pager: bool,
    pub path: Option<PathBuf>,
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Result<Self>;
    
    /// Save configuration to file
    pub fn save(&self) -> Result<()>;
    
    /// Get default config file path
    pub fn config_path() -> Option<PathBuf>;
}

impl Default for Config {
    fn default() -> Self;
}
```

---

### `glow_rs::ui`

User interface components.

```rust
use glow_rs::ui::{
    // Main render function
    render,
    // Components
    Viewport, Paginator, TextInput, Spinner, StatusBar,
    // Models
    stash::{StashModel, StashViewState, FilterState},
    pager::{PagerModel, PagerState},
    // Theme
    theme::{Theme, Colors},
};
```

#### `StashModel`

File listing model.

```rust
pub struct StashModel {
    pub view_state: StashViewState,
    pub filter_state: FilterState,
    pub filter_input: TextInput,
    pub markdowns: Vec<MarkdownDocument>,
    pub filtered_markdowns: Vec<MarkdownDocument>,
    pub loaded: bool,
    pub show_full_help: bool,
}

impl StashModel {
    pub fn new(config: &Config) -> Self;
    
    /// Get currently visible markdowns
    pub fn get_visible_markdowns(&self) -> &[MarkdownDocument];
    
    /// Get selected markdown
    pub fn selected_markdown(&self) -> Option<&MarkdownDocument>;
    
    /// Add documents to the list
    pub fn add_markdowns(&mut self, docs: Vec<MarkdownDocument>);
    
    /// Start filtering
    pub fn start_filtering(&mut self);
    
    /// Reset filtering
    pub fn reset_filtering(&mut self);
    
    /// Filter markdowns based on current input
    pub fn filter_markdowns(&mut self);
    
    /// Update pagination based on current items
    pub fn update_pagination(&mut self);
}

pub enum StashViewState {
    Ready,
    LoadingDocument,
    ShowingError,
}

pub enum FilterState {
    Unfiltered,
    Filtering,
    FilterApplied,
}
```

#### `PagerModel`

Document viewer model.

```rust
pub struct PagerModel {
    pub viewport: Viewport,
    pub state: PagerState,
    pub current_document: Option<MarkdownDocument>,
    pub rendered_content: Text<'static>,
    pub show_help: bool,
}

impl PagerModel {
    pub fn new(config: &Config) -> Self;
    
    /// Load a document
    pub fn load_document(&mut self, path: &Path) -> Result<()>;
    
    /// Unload current document
    pub fn unload(&mut self);
    
    /// Reload current document
    pub fn reload(&mut self) -> Result<()>;
    
    /// Scroll operations
    pub fn scroll_up(&mut self, lines: usize);
    pub fn scroll_down(&mut self, lines: usize);
    pub fn goto_top(&mut self);
    pub fn goto_bottom(&mut self);
    pub fn half_page_up(&mut self);
    pub fn half_page_down(&mut self);
    
    /// Get scroll percentage (0.0 - 1.0)
    pub fn scroll_percent(&self) -> f64;
    
    /// Copy to clipboard
    pub fn copy_to_clipboard(&self) -> Result<()>;
    
    /// Toggle help display
    pub fn toggle_help(&mut self);
}

pub enum PagerState {
    Browse,
    StatusMessage,
}
```

#### Components

##### `Viewport`

```rust
pub struct Viewport<'a> {
    content: Text<'a>,
    offset: usize,
}

impl<'a> Viewport<'a> {
    pub fn new(content: Text<'a>) -> Self;
    pub fn offset(self, offset: usize) -> Self;
    pub fn content_height(&self) -> usize;
    pub fn max_offset(&self, viewport_height: usize) -> usize;
    pub fn scroll_percent(&self, viewport_height: usize) -> f64;
}

impl<'a> Widget for Viewport<'a>;
```

##### `Paginator`

```rust
pub struct Paginator {
    pub page: usize,
    pub per_page: usize,
    pub total_items: usize,
}

impl Paginator {
    pub fn new(per_page: usize) -> Self;
    pub fn set_total_items(&mut self, total: usize);
    pub fn total_pages(&self) -> usize;
    pub fn slice_bounds(&self) -> (usize, usize);
    pub fn items_on_page(&self) -> usize;
    pub fn next_page(&mut self);
    pub fn prev_page(&mut self);
    pub fn on_first_page(&self) -> bool;
    pub fn on_last_page(&self) -> bool;
    pub fn render(&self, width: u16) -> Line;
}
```

##### `TextInput`

```rust
pub struct TextInput {
    // fields
}

impl TextInput {
    pub fn new(prompt: &str) -> Self;
    pub fn value(&self) -> &str;
    pub fn set_value(&mut self, value: String);
    pub fn focus(&mut self);
    pub fn blur(&mut self);
    pub fn is_focused(&self) -> bool;
    pub fn cursor_end(&mut self);
    pub fn cursor_start(&mut self);
    pub fn reset(&mut self);
    pub fn handle_key(&mut self, key: KeyCode) -> bool;
    pub fn render(&self, area: Rect, buf: &mut Buffer);
}
```

##### `Spinner`

```rust
pub struct Spinner {
    // fields
}

impl Spinner {
    pub fn new() -> Self;
    pub fn dots() -> Self;
    pub fn ascii() -> Self;
    pub fn style(self, style: Style) -> Self;
    pub fn tick(&mut self);
    pub fn frame(&self) -> &'static str;
    pub fn render(&self) -> Span<'static>;
    pub fn render_with_text(&self, text: &str) -> Line<'static>;
}
```

---

### `glow_rs::markdown`

Markdown processing.

```rust
use glow_rs::markdown::{
    MarkdownDocument,
    renderer::{MarkdownRenderer, RenderConfig, Theme},
    frontmatter::{remove_frontmatter, extract_frontmatter},
};
```

#### `MarkdownDocument`

```rust
pub struct MarkdownDocument {
    pub local_path: PathBuf,
    pub filter_value: String,
    pub body: String,
    pub note: String,
    pub modified: OffsetDateTime,
}

impl MarkdownDocument {
    pub fn from_path(path: PathBuf, cwd: &Path) -> Result<Self>;
    pub fn build_filter_value(&mut self);
    pub fn relative_time(&self) -> String;
    pub fn reload(&mut self) -> Result<()>;
}
```

#### `MarkdownRenderer`

```rust
pub struct MarkdownRenderer {
    // fields
}

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub show_line_numbers: bool,
    pub preserve_newlines: bool,
    pub max_width: u16,
    pub line_number_width: usize,
}

impl MarkdownRenderer {
    pub fn new(config: RenderConfig, width: u16, dark_mode: bool) -> Self;
    pub fn render(&mut self, markdown: &str) -> Text<'static>;
}
```

#### Frontmatter

```rust
/// Remove YAML frontmatter from content
pub fn remove_frontmatter(content: &str) -> String;

/// Extract frontmatter as key-value pairs
pub fn extract_frontmatter(content: &str) -> Option<Vec<(String, String)>>;
```

---

### `glow_rs::file`

File system operations.

```rust
use glow_rs::file::{
    finder::{FileFinder, FileSearchResult},
    watcher::{FileWatcher, WatchEvent},
};
```

#### `FileFinder`

```rust
pub struct FileSearchResult {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub is_dir: bool,
}

pub struct FileFinder {
    // fields
}

impl FileFinder {
    pub fn new(directory: &Path) -> Self;
    pub fn show_all(self, show: bool) -> Self;
    pub fn ignore_patterns(self, patterns: Vec<String>) -> Self;
    pub fn sender(self, tx: Sender<FileSearchResult>) -> Self;
    
    /// Start search in background thread
    pub fn spawn(self) -> Receiver<FileSearchResult>;
    
    /// Synchronous search
    pub fn find(self) -> Vec<FileSearchResult>;
}
```

#### `FileWatcher`

```rust
pub enum WatchEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Deleted(PathBuf),
    Error(String),
}

pub struct FileWatcher {
    // fields
}

impl FileWatcher {
    pub fn new() -> Result<Self>;
    pub fn watch(&mut self, path: &Path) -> Result<()>;
    pub fn unwatch(&mut self) -> Result<()>;
    pub fn watched_path(&self) -> Option<&Path>;
    pub fn try_recv(&self) -> Option<WatchEvent>;
    pub fn has_events(&self) -> bool;
    pub fn is_watched_file(&self, path: &Path) -> bool;
}
```

---

### `glow_rs::input`

Key handling.

```rust
use glow_rs::input::{
    keys::{Key, matches_any},
    bindings::{KeyBindings, GlobalKeyBindings, NavigationBindings, StashBindings, PagerBindings},
    handler::{KeyHandler, KeyResult},
};
```

#### `Key`

```rust
pub enum Key {
    Char(char),
    Ctrl(char),
    Alt(char),
    Shift(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Up, Down, Left, Right,
    Home, End,
    PageUp, PageDown,
    F(u8),
}

impl Key {
    pub fn matches(&self, event: &KeyEvent) -> bool;
    pub fn display(&self) -> String;
}

pub fn matches_any(event: &KeyEvent, keys: &[Key]) -> bool;
```

#### `KeyHandler`

```rust
pub enum KeyResult {
    Handled,
    Unhandled,
    Quit,
    Suspend,
    OpenDocument,
    EditDocument { line: usize },
    CopyContent,
    ReloadDocument,
    BackToStash,
    StartFilter,
    ClearFilter,
    ConfirmFilter,
    RefreshFiles,
    ToggleHelp,
    NextSection,
    PrevSection,
    NextPage,
    PrevPage,
    ShowErrors,
}

pub struct KeyHandler {
    bindings: KeyBindings,
}

impl KeyHandler {
    pub fn new() -> Self;
    pub fn with_bindings(bindings: KeyBindings) -> Self;
    pub fn handle_global(&self, event: &KeyEvent) -> Option<KeyResult>;
    pub fn handle_stash(&self, event: &KeyEvent, is_filtering: bool) -> Option<KeyResult>;
    pub fn handle_pager(&self, event: &KeyEvent) -> Option<KeyResult>;
    pub fn handle_filter(&self, event: &KeyEvent) -> Option<KeyResult>;
}
```

---

## Error Types

```rust
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Terminal error: {0}")]
    Terminal(String),
    
    #[error("Config error: {0}")]
    Config(String),
    
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    
    #[error("Parse error: {0}")]
    Parse(String),
}
```

## Examples

### Basic Usage

```rust
use glow_rs::app::App;
use glow_rs::config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let app = App::new(config)?;
    
    // Run the application
    run_app(app)
}
```

### Custom Render

```rust
use glow_rs::markdown::{MarkdownRenderer, RenderConfig};

fn render_markdown(content: &str) -> Text<'static> {
    let config = RenderConfig {
        show_line_numbers: true,
        max_width: 100,
        ..Default::default()
    };
    
    let mut renderer = MarkdownRenderer::new(config, 100, true);
    renderer.render(content)
}
```

### File Discovery

```rust
use glow_rs::file::finder::FileFinder;

fn find_markdown_files(dir: &Path) -> Vec<PathBuf> {
    FileFinder::new(dir)
        .find()
        .into_iter()
        .map(|r| r.path)
        .collect()
}
```
