# locus_ui Plan

Terminal UI crate for locus.codes using Ratatui + Crossterm.

## Current State

**Implemented:**
- `locus_constant` crate — theme colors (light/dark), app metadata
- Components: `Loader`, `Spinner`, `Grid`
- Animation: `Shimmer` (moving highlight effect)
- Pixel font for "locus." branding (Geist Pixel style)
- Demo binaries: `loader-demo`, `grid-demo`

**Builds:** ✅ **Tests:** Minimal (Grid, Shimmer, Loader, pixel_font)

---

## Architecture

```
locus_ui/
├── src/
│   ├── lib.rs              # Public exports
│   ├── theme.rs            # Runtime theme (light/dark toggle)
│   ├── components/
│   │   ├── mod.rs
│   │   ├── loader.rs       # Loading screen ✅
│   │   ├── spinner.rs      # Inline spinner ✅
│   │   ├── grid.rs         # Layout grid ✅
│   │   ├── pixel_font.rs   # Block font ✅
│   │   ├── chat.rs         # Chat/conversation view
│   │   ├── message.rs      # Single message block
│   │   ├── input.rs        # Text input area
│   │   ├── status_bar.rs   # Bottom status line
│   │   ├── tool_view.rs    # Tool execution display
│   │   ├── thinking.rs     # Reasoning/thinking blocks
│   │   ├── scroll.rs       # Scrollable panel wrapper
│   │   └── help.rs         # Help overlay (keybindings)
│   └── animation/
│       ├── mod.rs
│       └── shimmer.rs      # Shimmer effect ✅
```

---

## Visual Design

### Design Principles

1. **No borders** — Use background color changes, spacing, and indentation for visual separation
2. **Minimal chrome** — Content first, UI chrome second
3. **High contrast** — Text should always be readable
4. **Consistent spacing** — 1 line between blocks, 2 lines between messages

---

### Screen Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ locus.codes                              master • ~/app • 14:32     │  ← Header
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  USER                                                               │  ← User message
│  Add error handling to the login function                           │
│                                                                     │
│  ASSISTANT                                                          │  ← Assistant message
│  I'll add proper error handling to the login function.              │
│                                                                     │
│  > Thinking...                                                      │  ← Thinking (collapsed, dim)
│                                                                     │
│  * read_file                                                        │  ← Tool (running)
│      src/auth/login.rs                                              │
│                                                                     │
│  + read_file • 23ms                                                 │  ← Tool (done)
│      src/auth/login.rs                                              │
│      ─────────────────────────────────────────────                  │
│      pub fn login(user: &str, pass: &str) -> Result<Token> {        │
│          // current implementation                                  │  ← Code block (different bg)
│      }                                                              │
│                                                                     │
│  * edit_file                                                        │  ← Tool (running)
│      src/auth/login.rs                                              │
│                                                                     │
│  + edit_file • 45ms                                                 │  ← Tool (done)
│      src/auth/login.rs                                              │
│      ─────────────────────────────────────────────                  │
│      - pub fn login(user: &str, pass: &str) -> Result<Token> {      │  ← Diff style
│      + pub fn login(user: &str, pass: &str) -> Result<Token, AuthE> │
│                                                                     │
│  Here's the updated login function with error handling:             │
│                                                                     │
│      pub fn login(user: &str, pass: &str) -> Result<Token, AuthE> { │
│          if user.is_empty() {                                       │
│              return Err(AuthE::EmptyCredentials);                   │
│          }                                                          │
│          // ...                                                     │
│      }                                                              │
│                                                                     │
│                                        [scroll down for more...]    │  ← Scroll indicator
├─────────────────────────────────────────────────────────────────────┤
│  > _                                                                │  ← Input (1-3 lines)
├─────────────────────────────────────────────────────────────────────┤
│  Enter: send  Ctrl+C: quit  Ctrl+L: theme  ?: help  ↑↓: scroll     │  ← Shortcuts bar
└─────────────────────────────────────────────────────────────────────┘
```

### Layout Sections

| Section | Height | Background | Notes |
|---------|--------|------------|-------|
| Header | 1 line | `BACKGROUND` | Left: locus.codes, Right: branch • dir • time |
| Chat | Flexible | `BACKGROUND` | Main scrollable area |
| Input | 1-3 lines | `INPUT` | Text input with prompt |
| Shortcuts | 1 line | `SECONDARY` | Keyboard shortcuts hints |

---

### Color Usage (Dark Theme - Tokyo Night Inspired)

| Element | Foreground | Background | Style |
|---------|------------|------------|-------|
| **Screen** | `FOREGROUND` | `BACKGROUND` (#0a0a0f) | — |
| **Header brand** | `ACCENT` (#7aa2f7) | `BACKGROUND` | Bold |
| **Header info** | `MUTED_FG` (#565f89) | `BACKGROUND` | — |
| **User message** | `FOREGROUND` | `BACKGROUND` | — |
| **Assistant message** | `FOREGROUND` | `BACKGROUND` | — |
| **Role label** | `MUTED_FG` | — | Bold, uppercase |
| **Timestamp** | `TIMESTAMP` (#3d4166) | — | — |
| **Code block** | `FOREGROUND` | `CODE_BG` (#16161e) | Indented 4 spaces |
| **Bash command** | `FOREGROUND` | `BASH_BG` (#1a1b26) | Indented 4 spaces |
| **Tool block** | `FOREGROUND` | `TOOL_BG` (#11111a) | — |
| **Thinking** | `MUTED_FG` | `THINK_BG` (#0f0f18) | Italic, collapsible |
| **Input field** | `FOREGROUND` | `INPUT` (#1a1a26) | — |
| **Input prompt** | `ACCENT` (#7aa2f7) | `INPUT` | — |
| **Input placeholder** | `MUTED_FG` | `INPUT` | — |
| **Shortcuts bar** | `MUTED_FG` | `SECONDARY` (#1f1f2e) | — |
| **Shortcut key** | `FOREGROUND` | `SECONDARY` | Bold |
| **Scroll indicator** | `MUTED_FG` | `BACKGROUND` | Dim, right-aligned |
| **File path** | `FILE_PATH` (#7aa2f7) | — | — |
| **Tool name** | `TOOL_NAME` (#565f89) | — | — |
| **Running (*)** | `PRIMARY` | — | — |
| **Success (+)** | `SUCCESS` (#9ece6a) | — | — |
| **Error (!)** | `DANGER` (#f7768e) | — | — |
| **Warning** | `WARNING` (#e0af68) | — | — |
| **Info** | `INFO` (#7dcfff) | — | — |
| **Diff added (+)** | `SUCCESS` | — | — |
| **Diff removed (-)** | `DANGER` | — | — |

---

### Header

```
locus.codes                              master • ~/app • 14:32
│──────────────────────────────│         │────────────────────────│
Left side (brand)                         Right side (context info)
```

- **Left**: `locus.codes` in accent color (blue #7aa2f7), bold
- **Right**: Git branch, current directory, current time (muted, dim)
  - Format: `branch • directory • HH:MM`
  - Directory shown as `~/` relative when in home, otherwise basename

---

### Shortcuts Bar

```
Enter: send  Ctrl+C: quit  Ctrl+L: theme  ?: help  ↑↓: scroll
```

- Background: `SECONDARY` (#1f1f2e)
- Keys (e.g., `Enter`, `Ctrl+C`): foreground, bold
- Actions (e.g., `send`, `quit`): muted foreground
- Separator: two spaces between shortcuts

---

### Scroll Behavior

- Chat area scrolls automatically to bottom on new messages
- Scroll indicator appears when content exceeds viewport:
  - At bottom: `[scroll up for more...]`
  - In middle: `[↑ scroll up • scroll down ↓]`
  - At top: `[scroll down for more...]`
- User can scroll with `↑`/`↓` arrow keys or `PgUp`/`PgDn`

---

### Message Styling

#### User Message
```
USER                                    ← dim, bold, uppercase
Add error handling to the login function
                                        ← blank line after
```

#### Assistant Message
```
ASSISTANT                               ← dim, bold, uppercase
I'll add proper error handling.         ← normal text

    // code here                        ← indented code block
```

---

### Tool Display

#### Read File (Running)
```
* read_file                            ← spinner + tool_name (dim)
    src/auth/login.rs                   ← file_path (blue), indented
```

#### Read File (Done)
```
+ read_file • 23ms                      ← checkmark (green) + tool_name + duration
    src/auth/login.rs                   ← file_path (blue)
    ─────────────────────────────       ← separator (dim)
    pub fn login(...) {                 ← code on TOOL_BG, indented
        ...
    }
```

#### Edit File (Running)
```
* edit_file
    src/auth/login.rs
```

#### Edit File (Done)
```
+ edit_file • 45ms
    src/auth/login.rs
    ─────────────────────────────
    - pub fn login(...) -> Result<Token> {      ← removed (red)
    + pub fn login(...) -> Result<Token, AuthE> {  ← added (green)
```

#### Create File (Done)
```
+ create_file • 12ms
    src/auth/mod.rs                      ← new file (blue)
    ─────────────────────────────
    pub mod login;                       ← code content
    pub mod session;
```

#### Bash Command (Running)
```
* bash
    cargo test --lib
```

#### Bash Command (Done - Success)
```
+ bash • 2.3s
    cargo test --lib
    ─────────────────────────────
    running 42 tests                     ← stdout
    test result: ok
```

#### Bash Command (Done - Error)
```
! bash • 0.5s                            ← ! mark (red)
    cargo build
    ─────────────────────────────
    error[E0433]: failed to resolve      ← stderr (red text)
```

---

### Thinking Block

#### Collapsed
```
> Thinking...                            > = expand icon, dim text
```

#### Expanded
```
v Thinking                               v = collapse icon
    I need to check the current implementation    ← dim, indented
    first to understand the structure...

    Then I'll add error handling for:
    - Empty credentials
    - Invalid format
```

---

### Code Block

No syntax highlighting needed initially (can add later with syntect).

```
    pub fn login(user: &str, pass: &str) -> Result<Token, AuthE> {
        if user.is_empty() {
            return Err(AuthE::EmptyCredentials);
        }
        // ...
    }
```

- Indented 4 spaces from left
- Background: `CODE_BG` (#16161e)
- Foreground: `FOREGROUND` (#c0caf5)

---

### Input Area

```
┌─────────────────────────────────────────┐
│ > Add tests for the error cases         │  ← user typing
└─────────────────────────────────────────┘
```

- Background: `INPUT` (#1a1a26)
- Prompt: `> ` (primary color)
- Cursor: `_` (primary color)
- Placeholder (when empty): `Type a message...` (muted)

---

### Status Bar

```
claude-3.5-sonnet • 2,847 tokens • 3 tools used • ~/projects/myapp
```

- Background: `SECONDARY` (#1f1f2e)
- Foreground: `MUTED_FG` (#565f89)
- Separator: ` • `

When running:
```
* running edit_file • claude-3.5-sonnet • 2,847 tokens
```

---

### Visual Hierarchy (No Borders)

```
USER                                        ← role label (dim, bold)
Add error handling                          ← text (normal)
                                            ← blank line

                                            ← no border, just space
* read_file                                ← tool header (running)
    src/auth/login.rs                       ← indented path
                                            ← blank line

                                            ← no border, just space
+ read_file • 23ms
    src/auth/login.rs
    ─────────────────────                   ← separator line (dim)
    pub fn login(...) {                     ← code (different bg)
    }
                                            ← blank line

ASSISTANT
Here's what I found...
```

---

### Spacing Rules

| Context | Spacing |
|---------|---------|
| Between messages | 1 blank line |
| Between role + text | 0 lines (immediate) |
| Between tool header + content | 0 lines (immediate) |
| Between tool content + code | 1 blank line |
| Before separator line | 0 lines |
| After separator line | 0 lines |
| Between assistant text + code | 1 blank line |

---

## Components to Implement

### 1. Theme System (`theme.rs`)

Runtime theme with light/dark toggle. Wraps `locus_constant::theme` with a switch.

```rust
pub enum ThemeMode { Light, Dark }

pub struct Theme {
    mode: ThemeMode,
    // Cached colors from locus_constant
    bg: Color,
    fg: Color,
    primary: Color,
    muted: Color,
    danger: Color,
    success: Color,
    // ...
}

impl Theme {
    pub fn dark() -> Self;
    pub fn light() -> Self;
    pub fn toggle(&mut self);
    pub fn from_env() -> Self;  // Check COLOR_TERM/NO_COLOR
}
```

### 2. Header (`header.rs`)

Top bar with brand on left, context info on right.

```rust
pub struct Header {
    pub branch: Option<String>,     // Git branch
    pub directory: String,          // Current directory (~/relative or basename)
    pub time: String,               // HH:MM format
}

impl Header {
    pub fn new() -> Self;
    pub fn update_time(&mut self);
    pub fn update_git(&mut self, branch: Option<String>);
    pub fn update_dir(&mut self, dir: PathBuf);
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme);
}
```

### 3. Chat View (`chat.rs`)

Main conversation panel. Holds a list of messages and handles scrolling.

```rust
pub struct Chat {
    messages: Vec<Message>,
    scroll_offset: usize,
    max_messages: usize,  // Memory limit
    auto_scroll: bool,    // Auto-scroll to bottom on new messages
}

impl Chat {
    pub fn new() -> Self;
    pub fn push(&mut self, msg: Message);
    pub fn scroll_up(&mut self, lines: u16);
    pub fn scroll_down(&mut self, lines: u16);
    pub fn scroll_to_bottom(&mut self);
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme);
}
```

### 4. Message (`message.rs`)

Single message block with role indicator (User/Assistant) and content.

```rust
pub struct Message {
    pub role: Role,  // User, Assistant, System
    pub content: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
    pub token_usage: Option<TokenUsage>,
}

pub enum ContentBlock {
    Text(String),
    Thinking { text: String, expanded: bool },
    ToolUse(ToolDisplay),
    ToolResult { tool_id: String, output: String, is_error: bool },
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self;
    pub fn assistant(blocks: Vec<ContentBlock>) -> Self;
    pub fn height(&self, width: u16) -> u16;  // For scroll calc
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme);
}
```

### 5. Input Area (`input.rs`)

Multiline text input with editing support.

```rust
pub struct Input {
    text: String,
    cursor: usize,
    history: Vec<String>,
    history_index: Option<usize>,
    placeholder: String,
}

impl Input {
    pub fn new() -> Self;
    pub fn insert(&mut self, ch: char);
    pub fn backspace(&mut self);
    pub fn enter(&mut self) -> Option<String>;  // Submit
    pub fn history_up(&mut self);
    pub fn history_down(&mut self);
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme);
}
```

### 6. Shortcuts Bar (`shortcuts.rs`)

Bottom bar showing keyboard shortcuts.

```rust
pub struct ShortcutsBar {
    shortcuts: Vec<(String, String)>,  // (key, action) pairs
}

impl ShortcutsBar {
    pub fn new() -> Self {
        Self {
            shortcuts: vec![
                ("Enter".into(), "send".into()),
                ("Ctrl+C".into(), "quit".into()),
                ("Ctrl+L".into(), "theme".into()),
                ("?".into(), "help".into()),
                ("↑↓".into(), "scroll".into()),
            ],
        }
    }
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme);
}
```

### 7. Tool View (`tool_view.rs`)

Display tool execution (bash, edit_file, etc.) with status.

```rust
pub struct ToolView {
    pub id: String,
    pub name: String,
    pub args: Value,
    pub status: ToolStatus,
    pub output: Option<String>,
    pub duration_ms: Option<u64>,
}

impl ToolView {
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme, collapsed: bool);
}
```

### 7. Thinking Block (`thinking.rs`)

Collapsible reasoning/thinking display.

```rust
pub struct ThinkingBlock {
    pub text: String,
    pub expanded: bool,
}

impl ThinkingBlock {
    pub fn toggle(&mut self);
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme);
}
```

### 8. Scrollable Panel (`scroll.rs`)

Generic scrollable wrapper for any content.

```rust
pub struct ScrollPanel {
    pub offset: usize,
    pub content_height: usize,
}

impl ScrollPanel {
    pub fn scroll_up(&mut self, lines: usize);
    pub fn scroll_down(&mut self, lines: usize);
    pub fn scroll_to_top(&mut self);
    pub fn scroll_to_bottom(&mut self);
}
```

### 9. Help Overlay (`help.rs`)

Modal overlay showing keybindings.

```rust
pub struct HelpOverlay {
    pub visible: bool,
}

impl HelpOverlay {
    pub fn toggle(&mut self);
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme);
}
```

---

## Main App Structure

The main app that ties components together (will live in `locus_cli` or `locus_runtime`):

```rust
pub struct App {
    theme: Theme,
    chat: Chat,
    input: Input,
    status_bar: StatusBar,
    help: HelpOverlay,
    // State
    running: bool,
    show_help: bool,
}

impl App {
    pub fn new() -> Self;
    pub fn handle_event(&mut self, event: Event) -> AppAction;
    pub fn render(&self, f: &mut Frame);
}

pub enum AppAction {
    None,
    Quit,
    SendMessage(String),
    ToggleHelp,
    ScrollUp,
    ScrollDown,
    // ...
}
```

---

## Keybindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `Enter` | Send message (or newline with Shift/Meta) |
| `↑` / `↓` | Scroll chat (or history when input focused) |
| `PgUp` / `PgDn` | Page scroll |
| `Ctrl+L` | Toggle theme (light/dark) |
| `?` / `Ctrl+H` | Toggle help overlay |
| `Escape` | Close overlay / cancel |
| `Tab` | Focus next element |

---

## Integration with locus_core

Types to use from `locus_core`:

- `Role` — for message role
- `ContentBlock` — for message content (map to UI ContentBlock)
- `Turn` — for conversation turns
- `TokenUsage` — for status bar
- `ToolUse`, `ToolStatus` — for tool display
- `SessionEvent` — drive UI updates from runtime events

---

## Build Order

1. **theme.rs** — Runtime theme with toggle (no deps)
2. **scroll.rs** — Generic scroll state (no deps)
3. **message.rs** — Message block (depends: theme)
4. **input.rs** — Text input (depends: theme)
5. **status_bar.rs** — Status line (depends: theme)
6. **tool_view.rs** — Tool display (depends: theme)
7. **thinking.rs** — Collapsible thinking (depends: theme)
8. **chat.rs** — Chat view (depends: message, scroll, theme)
9. **help.rs** — Help overlay (depends: theme)

---

## Testing Strategy

- Unit tests for each component (state changes, calculations)
- Snapshot tests for rendering (optional, using `insta`)
- Integration test: render a full conversation

---

## Dependencies

Current (keep):
- `ratatui` — TUI framework
- `crossterm` — Terminal control
- `tokio` — Async
- `anyhow` — Errors
- `textwrap` — Text wrapping
- `unicode-segmentation`, `unicode-width` — Unicode handling
- `regex` — Text processing
- `syntect` — Syntax highlighting (for code blocks)
- `chrono` — Timestamps

Add:
- `locus-core` — Shared types ✅ (already in Cargo.toml)

---

## Demos

Keep demo binaries for testing individual components:
- `loader-demo` ✅
- `grid-demo` ✅
- Add: `chat-demo` — Interactive chat view mock

---

## Notes

- **No IO in components** — All components are pure render + state. IO happens in runtime.
- **Theme from constants** — Use `locus_constant::theme` as source of truth.
- **Streaming ready** — Message component should handle incremental updates (text deltas).
- **Accessibility** — Use high contrast, support NO_COLOR env var.
