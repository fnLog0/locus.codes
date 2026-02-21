# Glow Rust - Implementation Roadmap

## Overview

This document outlines the implementation phases for migrating Glow from Go to Rust.

## Phase 1: Project Setup (Days 1-2)

### Goals
- Set up project structure
- Configure dependencies
- Create basic CLI

### Tasks

- [ ] Initialize Cargo project
  ```bash
  cargo init --name glow_rs
  ```

- [ ] Set up `Cargo.toml` with all dependencies
  - ratatui
  - crossterm
  - clap
  - pulldown-cmark
  - syntect
  - walkdir
  - ignore
  - notify

- [ ] Create module structure
  ```
  src/
  ├── main.rs
  ├── lib.rs
  ├── app/
  ├── config/
  ├── ui/
  ├── markdown/
  ├── file/
  └── util/
  ```

- [ ] Implement basic CLI with clap
  - Parse arguments
  - Handle --help, --version

- [ ] Set up CI/CD
  - GitHub Actions for build/test
  - Release workflow

### Deliverables
- Compiles and runs
- CLI accepts arguments
- Basic tests pass

---

## Phase 2: Terminal & Configuration (Days 3-4)

### Goals
- Terminal setup/teardown
- Configuration loading
- Basic event loop

### Tasks

- [ ] Terminal management
  - Enable/disable raw mode
  - Alternate screen buffer
  - Mouse capture (optional)
  - Proper cleanup on exit

- [ ] Configuration
  - Define Config struct
  - Load from TOML file
  - CLI argument overrides
  - Environment variables

- [ ] Event loop skeleton
  - Poll for events
  - Handle resize
  - Graceful shutdown

### Code Example

```rust
fn run_tui(config: Config) -> Result<()> {
    // Setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // App
    let mut app = App::new(config)?;
    
    // Event loop
    loop {
        terminal.draw(|f| app.render(f))?;
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key)?;
                if app.should_quit {
                    break;
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

### Deliverables
- Clean terminal startup/shutdown
- Config loaded from file
- Responsive event loop

---

## Phase 3: File System (Days 5-7)

### Goals
- File discovery
- gitignore support
- File watching

### Tasks

- [ ] File finder
  - Walk directory tree
  - Filter by extension
  - Respect .gitignore
  - Send results via channel

- [ ] gitignore handling
  - Parse .gitignore
  - Match paths against patterns
  - Handle negation patterns

- [ ] File watcher
  - Watch directory for changes
  - Detect modifications
  - Trigger reload

### Code Example

```rust
pub fn find_files(dir: &Path, tx: Sender<FileResult>) {
    thread::spawn(move || {
        let gitignore = build_gitignore(dir);
        
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if should_ignore(&entry, &gitignore) {
                continue;
            }
            
            if is_markdown(entry.path()) {
                tx.send(FileResult::from_path(entry.path())).ok();
            }
        }
        
        tx.send(FileResult::Complete).ok();
    });
}
```

### Deliverables
- Files discovered asynchronously
- gitignore patterns respected
- File changes detected

---

## Phase 4: Markdown Rendering (Days 8-10)

### Goals
- Parse markdown
- Syntax highlighting
- Terminal rendering

### Tasks

- [ ] Markdown parser
  - Use pulldown-cmark
  - Handle all common elements
  - Extract frontmatter

- [ ] Syntax highlighting
  - Set up syntect
  - Language detection
  - Apply colors

- [ ] Terminal renderer
  - Convert to styled Text
  - Word wrapping
  - Line numbers (optional)

### Code Example

```rust
pub fn render_markdown(content: &str, width: u16) -> Text<'static> {
    let parser = Parser::new(content);
    let mut builder = TextBuilder::new(width);
    
    for event in parser {
        match event {
            Event::Start(Tag::Heading(level, ..)) => {
                builder.start_heading(level);
            }
            Event::Text(text) => {
                builder.add_text(&text);
            }
            // ... handle other events
        }
    }
    
    builder.build()
}
```

### Deliverables
- Markdown parsed correctly
- Code blocks highlighted
- Output styled properly

---

## Phase 5: Stash UI (Days 11-13)

### Goals
- File listing view
- Navigation
- Filtering

### Tasks

- [ ] Stash model
  - Document list
  - Pagination
  - Selection state
  - Filter state

- [ ] Stash view
  - Render file list
  - Render pagination
  - Render help

- [ ] Stash update
  - Handle navigation keys
  - Handle filter input
  - Handle selection

- [ ] Fuzzy filtering
  - Build filter values
  - Fuzzy match
  - Rank results

### Code Example

```rust
impl StashModel {
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor_down();
                None
            }
            KeyCode::Char('/') => {
                self.start_filtering();
                None
            }
            KeyCode::Enter => {
                self.selected_document()
                    .map(|doc| Action::Open(doc.path.clone()))
            }
            _ => None
        }
    }
}
```

### Deliverables
- File list displays correctly
- Navigation works
- Filtering functional

---

## Phase 6: Pager UI (Days 14-16)

### Goals
- Document viewer
- Scrolling
- File watching

### Tasks

- [ ] Pager model
  - Current document
  - Viewport state
  - Rendered content

- [ ] Pager view
  - Render document
  - Render status bar
  - Render help

- [ ] Pager update
  - Handle scroll keys
  - Handle actions (copy, edit)
  - Handle file changes

- [ ] Viewport scrolling
  - Line-by-line
  - Half-page
  - Page
  - Top/bottom

### Code Example

```rust
impl PagerModel {
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.viewport.scroll_down(1);
                None
            }
            KeyCode::Char('d') => {
                self.viewport.half_page_down();
                None
            }
            KeyCode::Char('c') => {
                self.copy_to_clipboard().ok();
                Some(Action::ShowMessage("Copied!".into()))
            }
            KeyCode::Esc => Some(Action::BackToStash),
            _ => None
        }
    }
}
```

### Deliverables
- Documents display correctly
- Scrolling smooth
- Auto-reload works

---

## Phase 7: Polish & Integration (Days 17-19)

### Goals
- Complete key bindings
- Help views
- Error handling
- Editor integration

### Tasks

- [ ] Complete key bindings
  - All navigation
  - All actions
  - Section switching

- [ ] Help system
  - Mini help
  - Full help
  - Context-aware

- [ ] Error handling
  - Error display
  - Recovery
  - Logging

- [ ] Editor integration
  - Open at line
  - Suspend/resume
  - Return to app

- [ ] Clipboard
  - Copy content
  - OSC 52 support

### Deliverables
- All features work
- Errors handled gracefully
- Help available

---

## Phase 8: Testing & Documentation (Days 20-21)

### Goals
- Unit tests
- Integration tests
- Documentation

### Tasks

- [ ] Unit tests
  - Component tests
  - Parser tests
  - Navigation tests

- [ ] Integration tests
  - CLI tests
  - Render tests
  - File system tests

- [ ] Documentation
  - README.md
  - API docs
  - Examples

### Deliverables
- >80% test coverage
- All docs complete
- Examples working

---

## Phase 9: Release Preparation (Day 22)

### Goals
- Performance testing
- Final polish
- Release

### Tasks

- [ ] Performance
  - Profile hot paths
  - Optimize rendering
  - Memory usage

- [ ] Final testing
  - Manual testing
  - Edge cases
  - Cross-platform

- [ ] Release
  - Version bump
  - Changelog
  - Binary builds

### Deliverables
- Release candidate
- Binaries for all platforms
- Published to crates.io

---

## Dependencies Between Phases

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 5 ──► Phase 7
                │                       │
                └──► Phase 4 ──────────┘
                                          │
                               Phase 6 ───┘
                                          │
                               Phase 8 ◄──┘
                                          │
                               Phase 9 ◄──┘
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Terminal compatibility | Test on multiple terminals |
| Performance | Profile early, optimize later |
| File watching edge cases | Extensive testing |
| Markdown edge cases | Test with real files |

## Success Criteria

- [ ] All Go features implemented
- [ ] Tests pass
- [ ] Performance comparable to Go version
- [ ] Documentation complete
- [ ] Works on Linux, macOS, Windows
