# Glow Rust - Quick Reference

## Common Patterns

### Creating the App

```rust
use glow_rs::app::App;
use glow_rs::config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut app = App::new(config)?;
    
    // Setup terminal
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    
    // Main loop
    loop {
        terminal.draw(|f| app.render(f))?;
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key)?;
                    if app.should_quit { break; }
                }
            }
        }
    }
    
    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

### Rendering a Widget

```rust
use ratatui::{Frame, layout::Rect, widgets::Paragraph};

fn render(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("Hello, World!")
        .style(Style::default().fg(Color::Green));
    f.render_widget(paragraph, area);
}
```

### Layout with Constraints

```rust
use ratatui::layout::{Layout, Constraint, Direction};

let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),    // Fixed height
        Constraint::Min(10),      // At least 10
        Constraint::Percentage(50), // 50% of remaining
        Constraint::Max(5),       // At most 5
    ])
    .split(area);
```

### Styled Text

```rust
use ratatui::text::{Text, Line, Span};
use ratatui::style::{Style, Color, Modifier};

let text = Text::from(vec![
    Line::from(vec![
        Span::styled("Hello", Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled("World", Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD)),
    ]),
]);
```

### Key Matching

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn is_quit(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Char('c') 
        if key.modifiers.contains(KeyModifiers::CONTROL))
}
```

## File Operations

### Read Markdown File

```rust
use std::path::Path;

fn read_markdown(path: &Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)?;
    let content = glow_rs::markdown::frontmatter::remove(&content);
    Ok(content)
}
```

### Find Markdown Files

```rust
use glow_rs::file::finder::FileFinder;

let results = FileFinder::new(dir.as_path())
    .show_all(config.show_all_files)
    .find();

for result in results {
    println!("Found: {:?}", result.path);
}
```

### Watch File for Changes

```rust
use glow_rs::file::watcher::FileWatcher;

let mut watcher = FileWatcher::new()?;
watcher.watch(&path)?;

// In main loop
if let Some(event) = watcher.try_recv() {
    match event {
        WatchEvent::Modified(p) => reload_document(&p)?,
        _ => {}
    }
}
```

## Markdown Rendering

### Basic Rendering

```rust
use glow_rs::markdown::renderer::{MarkdownRenderer, RenderConfig};

let config = RenderConfig::default();
let mut renderer = MarkdownRenderer::new(config, 80, true);
let text = renderer.render("# Hello\n\nWorld");
```

### With Line Numbers

```rust
let config = RenderConfig {
    show_line_numbers: true,
    line_number_width: 4,
    ..Default::default()
};
```

## UI Components

### Viewport

```rust
use glow_rs::ui::components::Viewport;

let viewport = Viewport::new(rendered_content)
    .offset(scroll_position);
f.render_widget(viewport, area);
```

### Paginator

```rust
use glow_rs::ui::components::Paginator;

let mut paginator = Paginator::new(10);
paginator.set_total_items(documents.len());

// Navigate
paginator.next_page();
paginator.prev_page();

// Render
let pagination_line = paginator.render(area.width);
```

### TextInput

```rust
use glow_rs::ui::components::TextInput;
use crossterm::event::KeyCode;

let mut input = TextInput::new("Search: ");
input.focus();

// Handle input
input.handle_key(KeyCode::Char('a'));

// Get value
let value = input.value();
```

### Spinner

```rust
use glow_rs::ui::components::Spinner;

let mut spinner = Spinner::new();
spinner.tick(); // Advance frame

let display = spinner.render_with_text("Loading...");
```

## State Management

### App State

```rust
pub enum AppState {
    ShowStash,
    ShowDocument,
}

impl App {
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.state {
            AppState::ShowStash => self.stash.handle_key(key),
            AppState::ShowDocument => self.pager.handle_key(key),
        }
    }
}
```

### Stash State

```rust
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

## Error Handling

```rust
use anyhow::{Context, Result};

fn load_document(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read: {}", path.display()))
}
```

## Configuration

### Load Config

```rust
use glow_rs::config::Config;

let config = Config::load().unwrap_or_default();
```

### Config File (TOML)

```toml
style = "auto"
width = 120
showLineNumbers = true
mouse = true
```

## Testing

### Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_paginator() {
        let mut p = Paginator::new(10);
        p.set_total_items(25);
        assert_eq!(p.total_pages(), 3);
    }
}
```

### Mock Terminal

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();

terminal.draw(|f| {
    // render
}).unwrap();

let buffer = terminal.backend().buffer();
```

## Common Pitfalls

### Forgot to Enable Raw Mode

```rust
// Always enable raw mode for TUI
enable_raw_mode()?;
// ... your TUI code ...
disable_raw_mode()?; // Don't forget to disable!
```

### Not Handling KeyEventKind

```rust
// Key events fire twice (Press and Release)
if let Event::Key(key) = event::read()? {
    if key.kind == KeyEventKind::Press {  // Check this!
        // handle key
    }
}
```

### Not Cloning Rc/Arc in Closures

```rust
// For closures that need owned values
let config = Rc::new(config);
let config_clone = config.clone();

thread::spawn(move || {
    // use config_clone
});
```

## Performance Tips

1. **Cache rendered content** - Only re-render when content changes
2. **Limit viewport** - Only render visible lines
3. **Use channels** - For async file operations
4. **Debounce events** - For file watching

```rust
// Debounce file events
let last_event = self.last_file_event;
if now - last_event < DEBOUNCE_DURATION {
    return; // Skip
}
```

## Useful Commands

```bash
# Build
cargo build --release

# Run with logging
RUST_LOG=debug cargo run

# Test
cargo test

# Check
cargo clippy -- -D warnings

# Format
cargo fmt

# Docs
cargo doc --open
```
