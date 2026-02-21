# Glow Rust - Getting Started

## Prerequisites

- Rust 1.70 or later
- A terminal with ANSI color support

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-org/glow-rs.git
cd glow-rs

# Build in release mode
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .
```

### From Crates.io

```bash
cargo install glow-rs
```

## Quick Start

### View a File

```bash
# View a markdown file
glow README.md

# View a file in TUI mode
glow --tui README.md

# View with pager
glow --pager README.md
```

### Browse Directory

```bash
# Browse current directory
glow

# Browse specific directory
glow ./docs

# Show all files including hidden
glow --all
```

### CLI Options

```
glow [OPTIONS] [SOURCE]

Arguments:
  [SOURCE]  File or directory to render

Options:
  -p, --pager              Display with pager
  -t, --tui                Display with TUI
  -s, --style <STYLE>      Style name or JSON path [default: auto]
  -w, --width <WIDTH>      Word-wrap at width [default: 0]
  -a, --all                Show system files and directories
  -l, --line-numbers       Show line numbers (TUI-mode only)
  -n, --preserve-new-lines Preserve newlines in output
  -m, --mouse              Enable mouse wheel (TUI-mode only)
      --config <CONFIG>    Config file path
  -h, --help               Print help
  -V, --version            Print version
```

## TUI Usage

### Navigation

| Key | Action |
|-----|--------|
| `j`/`↓` | Move down |
| `k`/`↑` | Move up |
| `g`/`Home` | Go to first item |
| `G`/`End` | Go to last item |
| `Enter` | Open document |
| `q`/`Esc` | Quit |

### Filtering

Press `/` to start filtering, then type to search. Press `Enter` to confirm or `Esc` to cancel.

### Document View

| Key | Action |
|-----|--------|
| `j`/`k` | Scroll line by line |
| `d`/`u` | Half page scroll |
| `g`/`G` | Go to top/bottom |
| `e` | Edit in $EDITOR |
| `c` | Copy to clipboard |
| `r` | Reload file |
| `?` | Toggle help |
| `Esc` | Back to file list |

## Configuration

### Config File Location

Config files are searched in this order:

1. `$GLOW_CONFIG_HOME/glow.toml`
2. `$XDG_CONFIG_HOME/glow/glow.toml`
3. `~/.config/glow/glow.toml`

### Example Config

```toml
# ~/.config/glow/glow.toml

# Style: auto, light, dark, dracula, pink, tokyo-night, or path to JSON
style = "auto"

# Word wrap width (0 = terminal width)
width = 120

# Show all files including hidden
all = false

# Show line numbers
showLineNumbers = true

# Preserve newlines
preserveNewLines = false

# Enable mouse support
mouse = true
```

### Custom Style

Create a JSON style file:

```json
{
  "document": {
    "block_prefix": "\n",
    "block_suffix": "\n",
    "margin": 2
  },
  "heading": {
    "block_prefix": "#",
    "color": "#04B575",
    "bold": true
  },
  "code_block": {
    "color": "#FFEE79",
    "background": "#2E2E2E"
  }
}
```

Use it with:

```bash
glow --style ~/.config/glow/my-style.json
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GLOW_CONFIG_HOME` | Override config directory |
| `GLOW_STYLE` | Default style |
| `GLOW_ENABLE_GLAMOUR` | Enable/disable rendering (true/false) |
| `EDITOR` | Editor for `e` command |
| `PAGER` | Pager for `--pager` mode |
| `COLOR_SCHEME` | Terminal color scheme (dark/light) |

## Editor Integration

The `e` key opens the current document in your editor. Set your editor:

```bash
# In ~/.bashrc or ~/.zshrc
export EDITOR=nvim
export VISUAL=code
```

## Project Structure

When you run `glow` in a directory, it:

1. Searches for markdown files (`.md`, `.markdown`, etc.)
2. Respects `.gitignore` rules
3. Displays files in a browsable list
4. Supports filtering with `/`

## Tips

### Integration with fzf

```bash
# Preview markdown files with glow
glow --pager $(fzf --preview 'glow {}')
```

### Git Integration

```bash
# View README from GitHub repo
glow github.com/user/repo

# View file from URL
glow https://raw.githubusercontent.com/user/repo/main/README.md
```

### Piping

```bash
# Read from stdin
echo "# Hello" | glow -

# Use with other tools
curl -sL https://example.com/readme.md | glow
```

## Troubleshooting

### Colors Not Displaying

Make sure your terminal supports 256 colors or true color:

```bash
echo $TERM
# Should be something like xterm-256color or screen-256color
```

### Performance Issues

For large directories:

```bash
# Use --all to skip gitignore processing
glow --all

# Reduce width for faster rendering
glow --width 80
```

### File Not Found

Check the path is correct:

```bash
# Absolute path
glow /full/path/to/file.md

# Relative path from current directory
glow ./docs/guide.md
```
