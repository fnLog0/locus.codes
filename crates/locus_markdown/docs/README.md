# Glow Rust Documentation

Complete documentation for the Rust implementation of Glow, a terminal-based markdown renderer using ratatui and crossterm.

## Documentation Index

| Document | Description |
|----------|-------------|
| [Architecture](01-architecture.md) | System architecture, module structure, data flow |
| [Components](02-components.md) | UI components: Viewport, Paginator, TextInput, Spinner, StatusBar |
| [Markdown Rendering](03-markdown-rendering.md) | Markdown parsing and terminal rendering |
| [File System](04-file-system.md) | File discovery, gitignore, file watching |
| [Key Bindings](05-key-bindings.md) | Keyboard handling and key binding reference |
| [Testing](06-testing.md) | Unit tests, integration tests, test utilities |
| [Getting Started](07-getting-started.md) | Installation, usage, configuration |
| [API Reference](08-api-reference.md) | Complete API documentation |
| [Implementation Roadmap](09-implementation-roadmap.md) | Development phases and timeline |

## Quick Links

### For Users
- [Getting Started](07-getting-started.md) - Install and use glow-rs
- [Key Bindings](05-key-bindings.md#key-reference-tables) - Keyboard shortcuts

### For Developers
- [Architecture](01-architecture.md) - How the application is structured
- [Components](02-components.md) - Building blocks for the UI
- [API Reference](08-api-reference.md) - Function and type documentation
- [Implementation Roadmap](09-implementation-roadmap.md) - Development plan

## Overview

Glow-rs is a Rust port of the popular [Glow](https://github.com/charmbracelet/glow) markdown renderer. It provides:

- **Terminal UI** for browsing and viewing markdown files
- **Syntax Highlighting** for code blocks
- **Fuzzy Search** for filtering files
- **File Watching** for auto-reload
- **Editor Integration** for editing files

## Technology Stack

| Component | Library |
|-----------|---------|
| TUI Framework | [ratatui](https://github.com/ratatui-org/ratatui) |
| Terminal | [crossterm](https://github.com/crossterm-rs/crossterm) |
| CLI | [clap](https://github.com/clap-rs/clap) |
| Markdown | [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) |
| Syntax Highlighting | [syntect](https://github.com/trishume/syntect) |
| File Walking | [walkdir](https://github.com/BurntSushi/walkdir) |
| gitignore | [ignore](https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore) |
| File Watching | [notify](https://github.com/notify-rs/notify) |

## Project Structure

```
glow-rs/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library exports
│   ├── app/              # Application core
│   ├── config/           # Configuration
│   ├── ui/               # User interface
│   │   ├── components/   # Reusable widgets
│   │   ├── stash/        # File listing
│   │   └── pager/        # Document viewer
│   ├── markdown/         # Markdown processing
│   ├── file/             # File system operations
│   └── util/             # Utilities
├── tests/
│   ├── integration/      # Integration tests
│   └── fixtures/         # Test files
└── docs/                 # This documentation
```

## Comparison with Go Version

| Feature | Go (Original) | Rust |
|---------|---------------|------|
| TUI Framework | Bubble Tea | ratatui |
| Markdown | glamour | pulldown-cmark + syntect |
| File Discovery | gitcha | walkdir + ignore |
| File Watching | fsnotify | notify |
| Styling | lipgloss | ratatui::style |
| CLI | cobra | clap |
| Config | viper | config + toml |

## Contributing

1. Read the [Architecture](01-architecture.md) document
2. Check the [Implementation Roadmap](09-implementation-roadmap.md)
3. Write tests (see [Testing](06-testing.md))
4. Submit a PR

## License

MIT
