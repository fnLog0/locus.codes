# Glow Rust - Architecture Overview

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Glow Rust                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐   │
│  │    CLI       │───►│    App       │◄───│   File System        │   │
│  │  (clap)      │    │   (State)    │    │   (walkdir/notify)   │   │
│  └──────────────┘    └──────┬───────┘    └──────────────────────┘   │
│                             │                                        │
│              ┌──────────────┼──────────────┐                        │
│              ▼              ▼              ▼                        │
│      ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐            │
│      │   Stash     │ │   Pager     │ │   Markdown      │            │
│      │   Model     │ │   Model     │ │   Renderer      │            │
│      └──────┬──────┘ └──────┬──────┘ └────────┬────────┘            │
│             │               │                 │                      │
│             └───────────────┴─────────────────┘                      │
│                             │                                        │
│                             ▼                                        │
│                    ┌─────────────────┐                              │
│                    │   Terminal UI   │                              │
│                    │   (ratatui)     │                              │
│                    └─────────────────┘                              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── main.rs              # Entry point, CLI parsing, mode selection
├── lib.rs               # Library exports
│
├── app/                 # Application core
│   ├── mod.rs
│   ├── app.rs           # Main App struct and event loop
│   ├── state.rs         # Application state definitions
│   └── command.rs       # Command pattern for side effects
│
├── config/              # Configuration management
│   ├── mod.rs
│   ├── config.rs        # Config struct and loading
│   └── styles.rs        # Style configuration
│
├── ui/                  # User interface components
│   ├── mod.rs
│   ├── render.rs        # Main render function
│   ├── theme.rs         # Colors and styles
│   ├── components/
│   │   ├── mod.rs
│   │   ├── viewport.rs  # Scrollable content area
│   │   ├── paginator.rs # Page navigation
│   │   ├── text_input.rs
│   │   ├── spinner.rs
│   │   └── status_bar.rs
│   ├── stash/
│   │   ├── mod.rs
│   │   ├── model.rs     # Stash state
│   │   ├── view.rs      # Stash rendering
│   │   └── update.rs    # Stash event handling
│   └── pager/
│       ├── mod.rs
│       ├── model.rs     # Pager state
│       ├── view.rs      # Pager rendering
│       └── update.rs    # Pager event handling
│
├── markdown/            # Markdown processing
│   ├── mod.rs
│   ├── document.rs      # Document type
│   ├── renderer.rs      # Terminal rendering
│   ├── frontmatter.rs   # YAML handling
│   └── syntax.rs        # Code highlighting
│
├── file/                # File system operations
│   ├── mod.rs
│   ├── finder.rs        # File discovery
│   ├── watcher.rs       # File watching
│   └── ignore.rs        # gitignore patterns
│
└── util/                # Utilities
    ├── mod.rs
    ├── text.rs          # Text manipulation
    └── time.rs          # Time formatting
```

## Data Flow

```
User Input ──► Event Loop ──► App::handle_key()
                                    │
                                    ▼
                            Update State
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              Stash::update  Pager::update   Commands
                    │               │               │
                    └───────────────┴───────────────┘
                                    │
                                    ▼
                            App::render()
                                    │
                                    ▼
                        Terminal::draw()
```

## State Management

The application uses a centralized state model with The Elm Architecture pattern:

```rust
// Core pattern
pub enum Msg {
    Key(KeyEvent),
    Resize(u16, u16),
    FileFound(FileSearchResult),
    FileSearchComplete,
    DocumentLoaded(MarkdownDocument),
    RenderComplete,
    Error(anyhow::Error),
}

pub enum Command {
    None,
    FindFiles,
    LoadDocument(PathBuf),
    RenderMarkdown(String),
    WatchFile(PathBuf),
    OpenEditor(PathBuf, usize),
    CopyToClipboard(String),
    Quit,
}

impl App {
    pub fn update(&mut self, msg: Msg) -> Vec<Command> {
        // Update state based on message
        // Return commands for side effects
    }
    
    pub fn view(&self) -> impl Widget {
        // Render current state
    }
}
```

## Key Design Decisions

### 1. Immediate Mode Rendering
- ratatui uses immediate mode - the entire UI is redrawn each frame
- No widget tree to manage
- State changes automatically reflected on next render

### 2. Synchronous Event Loop
- Single-threaded main loop for simplicity
- File I/O in background threads with channel communication
- No async runtime required for core functionality

### 3. Component-Based Architecture
- Each major UI area (stash, pager) is a self-contained component
- Components have their own state, update, and view functions
- Parent App coordinates between components

### 4. Command Pattern for Side Effects
- All side effects (file I/O, clipboard, editor) as commands
- Commands executed after state update
- Enables testing without actual side effects

## Thread Model

```
┌─────────────────────────────────────────────────────────────────┐
│                         Main Thread                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Event Loop                             │   │
│  │   loop {                                                  │   │
│  │     poll_events() ─► update() ─► render()                │   │
│  │   }                                                       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
         │ mpsc::channel                  │ mpsc::channel
         ▼                               ▼
┌──────────────────┐            ┌──────────────────┐
│  File Finder     │            │  File Watcher    │
│  Thread          │            │  Thread          │
│                  │            │                  │
│  Walks directory │            │  Monitors file   │
│  Finds *.md      │            │  changes         │
└──────────────────┘            └──────────────────┘
```

## Performance Considerations

### Rendering
- Only render visible lines (viewport optimization)
- Cache rendered markdown until document changes
- Use ratatui's diffing for minimal updates

### File Operations
- Background thread for file discovery
- Incremental file loading via channels
- Lazy loading of document content

### Memory
- Documents loaded on demand, not all at once
- Rendered content cached but bounded
- File finder sends results incrementally
