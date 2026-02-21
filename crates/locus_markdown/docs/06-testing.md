# Glow Rust - Testing Guide

This document covers testing strategies and examples.

## Test Structure

```
tests/
├── integration/
│   ├── main_test.rs        # CLI integration tests
│   └── render_test.rs      # Rendering tests
└── fixtures/
    ├── sample.md           # Test markdown files
    └── expected/           # Expected outputs

src/
└── (module)/
    └── (module)_test.rs    # Unit tests alongside code
```

## Unit Tests

### Testing Components

```rust
// src/ui/components/paginator_test.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginator_new() {
        let p = Paginator::new(10);
        assert_eq!(p.per_page, 10);
        assert_eq!(p.page, 0);
        assert_eq!(p.total_items, 0);
    }

    #[test]
    fn test_paginator_set_total_items() {
        let mut p = Paginator::new(10);
        p.set_total_items(25);
        
        assert_eq!(p.total_items, 25);
        assert_eq!(p.total_pages(), 3);
    }

    #[test]
    fn test_paginator_slice_bounds() {
        let mut p = Paginator::new(10);
        p.set_total_items(25);
        
        // First page
        assert_eq!(p.slice_bounds(), (0, 10));
        
        // Second page
        p.next_page();
        assert_eq!(p.slice_bounds(), (10, 20));
        
        // Last page (partial)
        p.next_page();
        assert_eq!(p.slice_bounds(), (20, 25));
    }

    #[test]
    fn test_paginator_navigation() {
        let mut p = Paginator::new(10);
        p.set_total_items(25);
        
        assert_eq!(p.page, 0);
        assert!(p.on_first_page());
        
        p.next_page();
        assert_eq!(p.page, 1);
        assert!(!p.on_first_page());
        assert!(!p.on_last_page());
        
        p.next_page();
        assert_eq!(p.page, 2);
        assert!(p.on_last_page());
        
        // Can't go past last page
        p.next_page();
        assert_eq!(p.page, 2);
        
        p.prev_page();
        assert_eq!(p.page, 1);
    }

    #[test]
    fn test_paginator_empty() {
        let mut p = Paginator::new(10);
        p.set_total_items(0);
        
        assert_eq!(p.total_pages(), 1);
        assert_eq!(p.slice_bounds(), (0, 0));
    }

    #[test]
    fn test_paginator_single_page() {
        let mut p = Paginator::new(10);
        p.set_total_items(5);
        
        assert_eq!(p.total_pages(), 1);
        assert!(p.on_first_page());
        assert!(p.on_last_page());
    }

    #[test]
    fn test_paginator_render_dots() {
        let mut p = Paginator::new(10);
        p.set_total_items(25);
        
        let line = p.render(80);
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_paginator_render_arabic() {
        let mut p = Paginator::new(10);
        p.set_total_items(1000);
        
        // Too many pages for dots, should use arabic
        let line = p.render(20);
        let text: String = line.spans.iter()
            .map(|s| s.content.as_str())
            .collect();
        assert!(text.contains('/'));
    }
}
```

### Testing Text Input

```rust
// src/ui/components/text_input_test.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    #[test]
    fn test_text_input_new() {
        let input = TextInput::new("Find:");
        assert_eq!(input.value(), "");
        assert!(!input.is_focused());
    }

    #[test]
    fn test_text_input_insert() {
        let mut input = TextInput::new("> ");
        input.focus();
        
        input.handle_key(KeyCode::Char('a'));
        input.handle_key(KeyCode::Char('b'));
        input.handle_key(KeyCode::Char('c'));
        
        assert_eq!(input.value(), "abc");
    }

    #[test]
    fn test_text_input_backspace() {
        let mut input = TextInput::new("> ");
        input.focus();
        
        input.handle_key(KeyCode::Char('a'));
        input.handle_key(KeyCode::Char('b'));
        input.handle_key(KeyCode::Char('c'));
        input.handle_key(KeyCode::Backspace);
        
        assert_eq!(input.value(), "ab");
    }

    #[test]
    fn test_text_input_delete() {
        let mut input = TextInput::new("> ");
        input.focus();
        
        input.handle_key(KeyCode::Char('a'));
        input.handle_key(KeyCode::Char('b'));
        input.handle_key(KeyCode::Char('c'));
        input.handle_key(KeyCode::Left);
        input.handle_key(KeyCode::Delete);
        
        assert_eq!(input.value(), "ab");
    }

    #[test]
    fn test_text_input_cursor_movement() {
        let mut input = TextInput::new("> ");
        input.focus();
        
        input.handle_key(KeyCode::Char('a'));
        input.handle_key(KeyCode::Char('b'));
        input.handle_key(KeyCode::Char('c'));
        
        input.handle_key(KeyCode::Left);
        input.handle_key(KeyCode::Char('X'));
        
        assert_eq!(input.value(), "abXc");
        
        input.handle_key(KeyCode::Home);
        input.handle_key(KeyCode::Char('Y'));
        assert_eq!(input.value(), "YabXc");
        
        input.handle_key(KeyCode::End);
        input.handle_key(KeyCode::Char('Z'));
        assert_eq!(input.value(), "YabXcZ");
    }

    #[test]
    fn test_text_input_unicode() {
        let mut input = TextInput::new("> ");
        input.focus();
        
        input.handle_key(KeyCode::Char('h'));
        input.handle_key(KeyCode::Char('e'));
        input.handle_key(KeyCode::Char('l'));
        input.handle_key(KeyCode::Char('l'));
        input.handle_key(KeyCode::Char('o'));
        input.handle_key(KeyCode::Char(' '));
        input.handle_key(KeyCode::Char('世界'));
        
        // Note: This depends on how you handle multi-char input
        // You may need to adjust for proper Unicode handling
    }
}
```

### Testing Markdown Renderer

```rust
// src/markdown/renderer_test.rs

#[cfg(test)]
mod tests {
    use super::*;

    fn default_renderer(width: u16) -> MarkdownRenderer {
        MarkdownRenderer::new(
            RenderConfig::default(),
            width,
            true, // dark mode
        )
    }

    #[test]
    fn test_render_heading() {
        let mut renderer = default_renderer(80);
        let text = renderer.render("# Hello World");
        
        assert!(text.lines.len() > 0);
        // First line should be styled heading
        let line = &text.lines[0];
        assert!(line.spans.iter().any(|s| s.content.contains("Hello")));
    }

    #[test]
    fn test_render_paragraph() {
        let mut renderer = default_renderer(80);
        let text = renderer.render("This is a paragraph.");
        
        assert!(text.lines.len() >= 1);
    }

    #[test]
    fn test_render_code_block() {
        let mut renderer = default_renderer(80);
        let md = r#"```rust
fn main() {
    println!("Hello");
}
```"#;
        
        let text = renderer.render(md);
        assert!(text.lines.len() > 3); // Should have code lines
    }

    #[test]
    fn test_render_inline_code() {
        let mut renderer = default_renderer(80);
        let text = renderer.render("Use the `print` function.");
        
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_list() {
        let mut renderer = default_renderer(80);
        let md = "- Item 1\n- Item 2\n- Item 3";
        let text = renderer.render(md);
        
        // Should have at least 3 lines for items
        assert!(text.lines.len() >= 3);
    }

    #[test]
    fn test_render_links() {
        let mut renderer = default_renderer(80);
        let text = renderer.render("[Click here](https://example.com)");
        
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_emphasis() {
        let mut renderer = default_renderer(80);
        let text = renderer.render("This is **bold** and *italic* text.");
        
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_mixed() {
        let mut renderer = default_renderer(80);
        let md = r#"# Title

A paragraph with **bold** text.

## Subtitle

- Item 1
- Item 2

```python
print("code")
```

[Link](https://example.com)
"#;
        
        let text = renderer.render(md);
        assert!(text.lines.len() > 10);
    }

    #[test]
    fn test_line_numbers() {
        let mut config = RenderConfig::default();
        config.show_line_numbers = true;
        
        let mut renderer = MarkdownRenderer::new(config, 80, true);
        let text = renderer.render("Line 1\n\nLine 2\n\nLine 3");
        
        // First span of each line should be line number
        for line in &text.lines {
            if !line.spans.is_empty() {
                let first = &line.spans[0];
                assert!(first.content.trim().parse::<u32>().is_ok());
            }
        }
    }

    #[test]
    fn test_word_wrapping() {
        let mut renderer = default_renderer(20); // Narrow width
        let long_text = "This is a very long line that should be wrapped to multiple lines";
        let text = renderer.render(long_text);
        
        // Should wrap to multiple lines
        assert!(text.lines.len() > 1);
    }
}
```

### Testing File Finder

```rust
// src/file/finder_test.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_files(dir: &Path) {
        File::create(dir.join("readme.md")).unwrap();
        File::create(dir.join("guide.markdown")).unwrap();
        File::create(dir.join("notes.mdown")).unwrap();
        File::create(dir.join("ignore.txt")).unwrap();
        
        // Create subdirectory with files
        fs::create_dir(dir.join("subdir")).unwrap();
        File::create(dir.join("subdir/nested.md")).unwrap();
    }

    #[test]
    fn test_find_markdown_files() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());
        
        let results = FileFinder::new(dir.path()).find();
        
        assert_eq!(results.len(), 4);
        assert!(results.iter().any(|r| r.path.ends_with("readme.md")));
        assert!(results.iter().any(|r| r.path.ends_with("guide.markdown")));
        assert!(results.iter().any(|r| r.path.ends_with("notes.mdown")));
        assert!(results.iter().any(|r| r.path.ends_with("nested.md")));
    }

    #[test]
    fn test_find_respects_gitignore() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());
        
        // Create .gitignore
        let mut gitignore = File::create(dir.path().join(".gitignore")).unwrap();
        writeln!(gitignore, "ignore.txt").unwrap();
        writeln!(gitignore, "subdir/").unwrap();
        
        let results = FileFinder::new(dir.path()).find();
        
        // Should not include files in ignored directories
        assert!(!results.iter().any(|r| r.path.ends_with("nested.md")));
    }

    #[test]
    fn test_find_show_all() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());
        
        // Create .gitignore that ignores everything
        let mut gitignore = File::create(dir.path().join(".gitignore")).unwrap();
        writeln!(gitignore, "*").unwrap();
        
        let results = FileFinder::new(dir.path())
            .show_all(true)
            .find();
        
        // Should include all files despite gitignore
        assert!(results.len() >= 4);
    }

    #[test]
    fn test_find_async() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());
        
        let rx = FileFinder::new(dir.path()).spawn();
        
        let mut count = 0;
        while let Ok(_) = rx.try_recv() {
            count += 1;
        }
        
        // Give thread time to complete
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        while let Ok(_) = rx.try_recv() {
            count += 1;
        }
        
        assert_eq!(count, 4);
    }
}
```

## Integration Tests

### Testing CLI

```rust
// tests/integration/main_test.rs

use std::process::Command;

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("glow"));
    assert!(stdout.contains("markdown"));
}

#[test]
fn test_version_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("glow"));
}

#[test]
fn test_render_file() {
    // Create temp file
    use std::io::Write;
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(temp_file, "# Test\n\nHello world.").unwrap();
    
    let output = Command::new("cargo")
        .args(["run", "--", temp_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test"));
}
```

### Testing Rendering

```rust
// tests/integration/render_test.rs

use glow_rs::markdown::{renderer::MarkdownRenderer, config::RenderConfig};

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{}", name))
        .expect("Failed to read fixture")
}

#[test]
fn test_render_readme() {
    let content = read_fixture("sample.md");
    let mut renderer = MarkdownRenderer::new(
        RenderConfig::default(),
        80,
        true,
    );
    
    let result = renderer.render(&content);
    
    // Basic validation
    assert!(!result.lines.is_empty());
}

#[test]
fn test_render_with_frontmatter() {
    let content = read_fixture("with_frontmatter.md");
    let content = glow_rs::markdown::frontmatter::remove(&content);
    
    let mut renderer = MarkdownRenderer::new(
        RenderConfig::default(),
        80,
        true,
    );
    
    let result = renderer.render(&content);
    assert!(!result.lines.is_empty());
}
```

## Mock Terminal for Testing

```rust
// tests/support/mock_terminal.rs

use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    Terminal,
};

pub struct MockTerminal {
    terminal: Terminal<TestBackend>,
}

impl MockTerminal {
    pub fn new(width: u16, height: u16) -> Self {
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).unwrap();
        Self { terminal }
    }
    
    pub fn draw<F>(&mut self, f: F) -> &Buffer
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f).unwrap();
        self.terminal.backend().buffer()
    }
    
    pub fn buffer(&self) -> &Buffer {
        self.terminal.backend().buffer()
    }
    
    pub fn assert_line(&self, y: u16, expected: &str) {
        let buffer = self.buffer();
        let line: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        
        assert_eq!(line.trim(), expected);
    }
    
    pub fn assert_contains(&self, text: &str) {
        let buffer = self.buffer();
        let content = buffer.content();
        let content_str: String = content.iter()
            .map(|c| c.symbol())
            .collect();
        
        assert!(content_str.contains(text), 
            "Expected buffer to contain '{}'", text);
    }
}

// Usage in tests
#[test]
fn test_render_stash() {
    let mut term = MockTerminal::new(80, 24);
    
    let mut app = App::new(Config::default()).unwrap();
    app.stash.add_document(MarkdownDocument {
        note: "test.md".to_string(),
        ..Default::default()
    });
    
    term.draw(|f| {
        ui::render(f, &app);
    });
    
    term.assert_contains("test.md");
}
```

## Test Fixtures

```markdown
<!-- tests/fixtures/sample.md -->
# Sample Document

This is a **sample** markdown document for testing.

## Features

- Bullet lists
- **Bold** and *italic* text
- `inline code`

```rust
fn main() {
    println!("Hello, world!");
}
```

## Links

[Example](https://example.com)

## Tables

| A | B |
|---|---|
| 1 | 2 |
```

```markdown
<!-- tests/fixtures/with_frontmatter.md -->
---
title: Document Title
date: 2024-01-01
tags:
  - test
  - example
---

# Content

This content comes after frontmatter.
```

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_paginator_navigation

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run with coverage (requires tarpaulin)
cargo tarpaulin --out Html
```
