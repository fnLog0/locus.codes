# locus_tui — Detailed Task Plan

Agent-ready task list for `crates/locus_tui/`. Each task is self-contained with exact file paths, current code, what to change, and verification steps.

**Verify every task:** `cargo check -p locus-tui && cargo test -p locus-tui && cargo clippy -p locus-tui`

---

## File map

```
crates/locus_tui/
├── Cargo.toml                          # Dependencies
├── src/
│   ├── lib.rs                          # Public API, module declarations
│   ├── run.rs                          # Event loop (keyboard, session events, draw)
│   ├── state.rs                        # TuiState, ChatItem enum, input buffer
│   ├── view.rs                         # Main draw function (header, body, footer)
│   ├── runtime_events.rs              # SessionEvent → TuiState mapping
│   ├── animation/
│   │   ├── mod.rs
│   │   └── shimmer.rs                  # Shimmer animation (lerp, styled spans)
│   ├── layouts/
│   │   ├── mod.rs                      # Re-exports
│   │   ├── head.rs                     # Header bar (title + status)
│   │   ├── chats.rs                    # Chat body area
│   │   ├── input.rs                    # Input bar (bordered block)
│   │   ├── shortcut.rs                 # Shortcut hint line
│   │   ├── split.rs                    # Screen splitting (header/body/footer)
│   │   ├── style.rs                    # Rgb → ratatui Style helpers
│   │   └── panel.rs                    # Bordered panel layout
│   ├── messages/
│   │   ├── mod.rs                      # Module declarations
│   │   ├── user.rs                     # User message rendering (» indicator)
│   │   ├── ai_message.rs              # AI message rendering (▸ indicator)
│   │   ├── ai_think_message.rs        # Thinking message rendering (⋯ indicator)
│   │   ├── tool.rs                     # Tool call status lines (▶/✓/✗)
│   │   └── meta_tool.rs              # Meta-tool status lines
│   ├── theme/
│   │   ├── mod.rs
│   │   ├── appearance.rs              # Dark/Light enum
│   │   ├── palette.rs                 # LocusPalette (all semantic colors)
│   │   └── rgb.rs                     # Rgb tuple type
│   └── utils/
│       ├── mod.rs                      # Re-exports
│       ├── constants.rs               # Spacing, padding constants
│       ├── format.rs                  # Duration, truncation, word-wrap
│       └── layout.rs                  # Rect padding, scroll buffer, right-align
```

---

## T01 · Auto-scroll on new content

### Problem
`state.scroll` is manual only. When AI responds, new messages appear below the viewport. User sees nothing unless they scroll down manually.

### Files to change
- `src/state.rs`
- `src/run.rs`
- `src/runtime_events.rs`

### Steps

**1. Add `auto_scroll` field to `TuiState` in `src/state.rs`:**

```rust
// In TuiState struct, add after `current_think_text`:
/// Whether to auto-scroll to bottom on new content.
pub auto_scroll: bool,
```

Set default to `true` in `impl Default for TuiState`:
```rust
auto_scroll: true,
```

**2. Reset scroll in push methods in `src/state.rs`:**

Add this private helper:
```rust
/// Reset scroll to bottom if auto_scroll is on.
fn maybe_scroll_bottom(&mut self) {
    if self.auto_scroll {
        self.scroll = 0;
    }
}
```

Call `self.maybe_scroll_bottom()` at the end of: `push_user`, `push_ai`, `push_think`, `push_tool`, `push_meta_tool`, `flush_turn`.

**3. Disable auto_scroll on manual scroll up in `src/run.rs`:**

In the `KeyCode::Up` arm:
```rust
KeyCode::Up => {
    state.scroll_up(1);
    state.auto_scroll = false;
}
```

Same for `KeyCode::PageUp`:
```rust
KeyCode::PageUp => {
    state.scroll_up(5);
    state.auto_scroll = false;
}
```

**4. Re-enable auto_scroll when user scrolls to bottom in `src/run.rs`:**

In `KeyCode::Down` and `KeyCode::PageDown`:
```rust
KeyCode::Down => {
    state.scroll_down(1);
    if state.scroll == 0 {
        state.auto_scroll = true;
    }
}
```

**5. Add End key to jump to bottom in `src/run.rs`:**

```rust
KeyCode::End if state.input_buffer.is_empty() => {
    state.scroll = 0;
    state.auto_scroll = true;
}
```

**6. Auto-scroll after draining session events in `src/run.rs`:**

After the `while let Ok(event) = rx.try_recv()` block:
```rust
if state.auto_scroll {
    state.scroll = 0;
}
```

### Verify
- Run TUI, send a message, scroll up with ↑, then send another message → should NOT auto-scroll (user scrolled away).
- Press End → should jump to bottom and re-enable auto-scroll.
- New messages after End should appear at bottom automatically.

---

## T02 · Inline error messages in chat

### Problem
`SessionEvent::Error` only sets `state.status` (header text). Errors vanish when next status arrives. User never sees what went wrong.

### Files to change
- `src/messages/error.rs` (NEW)
- `src/messages/mod.rs`
- `src/state.rs`
- `src/runtime_events.rs`
- `src/view.rs`

### Steps

**1. Create `src/messages/error.rs`:**

```rust
//! Error message rendering.
//!
//! Shown with ✗ icon in danger color. Wraps like AI messages.

use ratatui::text::{Line, Span};

use crate::layouts::{danger_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::{wrap_lines, LEFT_PADDING};

/// Error message for display.
#[derive(Debug, Clone)]
pub struct ErrorMessage {
    pub text: String,
}

/// Indicator for error messages.
pub const ERROR_INDICATOR: &str = "✗";

/// Build lines for an error message: indicator + wrapped text, all in danger color.
pub fn error_message_lines(msg: &ErrorMessage, palette: &LocusPalette, width: usize) -> Vec<Line<'static>> {
    let indent_len = LEFT_PADDING.len();
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let wrapped = wrap_lines(msg.text.trim(), wrap_width);
    let style = danger_style(palette.danger);

    if wrapped.is_empty() {
        return vec![Line::from(vec![
            Span::styled(ERROR_INDICATOR.to_string(), style),
            Span::raw(" "),
            Span::styled("Unknown error".to_string(), style),
        ])];
    }

    let mut lines = Vec::with_capacity(wrapped.len());
    lines.push(Line::from(vec![
        Span::styled(ERROR_INDICATOR.to_string(), style),
        Span::raw(" "),
        Span::styled(wrapped[0].clone(), style),
    ]));

    for seg in wrapped.iter().skip(1) {
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(seg.clone(), style),
        ]));
    }
    lines
}
```

**2. Add module to `src/messages/mod.rs`:**

```rust
pub mod error;
```

**3. Add `ChatItem::Error` variant in `src/state.rs`:**

Add import:
```rust
use crate::messages::error::ErrorMessage;
```

Add variant to `ChatItem` enum:
```rust
Error(ErrorMessage),
```

Add method to `TuiState`:
```rust
/// Push an error message.
pub fn push_error(&mut self, text: String) {
    self.messages.push(ChatItem::Error(ErrorMessage { text }));
    self.maybe_scroll_bottom();
}
```

**4. Update `src/runtime_events.rs`:**

Change the `SessionEvent::Error` arm from:
```rust
SessionEvent::Error { error } => {
    state.status = error;
}
```
To:
```rust
SessionEvent::Error { error } => {
    state.push_error(error.clone());
    state.status = error;
}
```

**5. Update `src/view.rs`:**

Add import:
```rust
use crate::messages::error;
```

Add match arm in the `for item in &state.messages` loop:
```rust
ChatItem::Error(m) => all_lines.extend(error::error_message_lines(m, palette, width)),
```

**6. Update `src/lib.rs`:**

Add to the `pub use state::` line:
```rust
pub use state::{ChatItem, TuiState};
// ChatItem now includes Error variant — no API change needed
```

### Verify
- `cargo test -p locus-tui` — existing tests must pass.
- Error messages should appear in chat with red `✗` icon.

---

## T03 · Keyboard: cursor movement + line editing

### Problem
Only Backspace and Char input work. Can't move cursor left/right, jump to start/end, or delete forward.

### Files to change
- `src/state.rs`
- `src/run.rs`

### Steps

**1. Add cursor movement methods to `TuiState` in `src/state.rs`:**

```rust
/// Move cursor left one character (UTF-8 safe).
pub fn input_cursor_left(&mut self) {
    if self.input_cursor == 0 {
        return;
    }
    // Walk back to previous char boundary
    let mut pos = self.input_cursor - 1;
    while pos > 0 && !self.input_buffer.is_char_boundary(pos) {
        pos -= 1;
    }
    self.input_cursor = pos;
}

/// Move cursor right one character (UTF-8 safe).
pub fn input_cursor_right(&mut self) {
    if self.input_cursor >= self.input_buffer.len() {
        return;
    }
    let mut pos = self.input_cursor + 1;
    while pos < self.input_buffer.len() && !self.input_buffer.is_char_boundary(pos) {
        pos += 1;
    }
    self.input_cursor = pos;
}

/// Move cursor to start of input.
pub fn input_cursor_home(&mut self) {
    self.input_cursor = 0;
}

/// Move cursor to end of input.
pub fn input_cursor_end(&mut self) {
    self.input_cursor = self.input_buffer.len();
}

/// Delete character at cursor (forward delete, UTF-8 safe).
pub fn input_delete(&mut self) {
    if self.input_cursor >= self.input_buffer.len() {
        return;
    }
    let mut end = self.input_cursor + 1;
    while end < self.input_buffer.len() && !self.input_buffer.is_char_boundary(end) {
        end += 1;
    }
    self.input_buffer.drain(self.input_cursor..end);
}

/// Clear entire input buffer (Ctrl+U).
pub fn input_clear(&mut self) {
    self.input_buffer.clear();
    self.input_cursor = 0;
}

/// Delete from cursor to end of line (Ctrl+K).
pub fn input_kill_to_end(&mut self) {
    self.input_buffer.truncate(self.input_cursor);
}
```

**2. Also fix `input_backspace` to use `is_char_boundary` instead of manual byte walking:**

Replace the current `input_backspace` body with:
```rust
pub fn input_backspace(&mut self) {
    if self.input_cursor == 0 {
        return;
    }
    let mut start = self.input_cursor - 1;
    while start > 0 && !self.input_buffer.is_char_boundary(start) {
        start -= 1;
    }
    self.input_buffer.drain(start..self.input_cursor);
    self.input_cursor = start;
}
```

**3. Add key handlers in `src/run.rs` inside the `match e.code` block:**

```rust
KeyCode::Left => state.input_cursor_left(),
KeyCode::Right => state.input_cursor_right(),
KeyCode::Home => state.input_cursor_home(),
KeyCode::End => {
    if state.input_buffer.is_empty() {
        // Scroll to bottom when input is empty
        state.scroll = 0;
        state.auto_scroll = true;
    } else {
        state.input_cursor_end();
    }
}
KeyCode::Delete => state.input_delete(),
KeyCode::Char('u') if e.modifiers.contains(KeyModifiers::CONTROL) => state.input_clear(),
KeyCode::Char('k') if e.modifiers.contains(KeyModifiers::CONTROL) => state.input_kill_to_end(),
```

**Important:** The `Char('u')` and `Char('k')` with CONTROL must come BEFORE the generic `KeyCode::Char(c)` arm. The `Left`/`Right`/`Home`/`End`/`Delete` arms must also come before the existing scroll arms or be ordered correctly.

Reorder the match arms:
```rust
match e.code {
    KeyCode::Char('c') if e.modifiers.contains(KeyModifiers::CONTROL) => break,
    KeyCode::Char('u') if e.modifiers.contains(KeyModifiers::CONTROL) => state.input_clear(),
    KeyCode::Char('k') if e.modifiers.contains(KeyModifiers::CONTROL) => state.input_kill_to_end(),
    KeyCode::Char('q') if state.input_buffer.is_empty() => break,
    KeyCode::Left => state.input_cursor_left(),
    KeyCode::Right => state.input_cursor_right(),
    KeyCode::Home => state.input_cursor_home(),
    KeyCode::End => {
        if state.input_buffer.is_empty() {
            state.scroll = 0;
            state.auto_scroll = true;
        } else {
            state.input_cursor_end();
        }
    }
    KeyCode::Delete => state.input_delete(),
    KeyCode::Up => { state.scroll_up(1); state.auto_scroll = false; }
    KeyCode::Down => {
        state.scroll_down(1);
        if state.scroll == 0 { state.auto_scroll = true; }
    }
    KeyCode::PageUp => { state.scroll_up(5); state.auto_scroll = false; }
    KeyCode::PageDown => {
        state.scroll_down(5);
        if state.scroll == 0 { state.auto_scroll = true; }
    }
    KeyCode::Enter => { /* existing code */ }
    KeyCode::Backspace => state.input_backspace(),
    KeyCode::Char(c) => state.input_insert(c),
    _ => {}
}
```

### Verify
- Type text, press Left/Right — cursor moves correctly.
- Home jumps to start, End to end.
- Delete removes char at cursor.
- Ctrl+U clears line.
- Ctrl+K kills to end.
- Test with emoji: type "hello 🎉 world", navigate around it.

---

## T04 · Input cursor rendering fix

### Problem
`view.rs` line 102-106 uses `chars().count()` for cursor positioning. Multi-byte and wide characters (CJK, emoji) make the cursor land at the wrong column.

### Files to change
- `Cargo.toml`
- `src/view.rs`

### Steps

**1. Add `unicode-width` to `Cargo.toml`:**

```toml
unicode-width = "0.2"
```

**2. Update cursor positioning in `src/view.rs`:**

Add import at top:
```rust
use unicode_width::UnicodeWidthStr;
```

Replace the cursor calculation block (lines ~101-107):
```rust
// OLD:
let icon_len = INPUT_ICON.chars().count() as u16;
let char_offset = state.input_buffer[..state.input_cursor.min(state.input_buffer.len())]
    .chars()
    .count() as u16;
let cursor_col = (inner.x + icon_len + char_offset).min(inner.x + inner.width);

// NEW:
let icon_width = UnicodeWidthStr::width(INPUT_ICON) as u16;
let text_before_cursor = &state.input_buffer[..state.input_cursor.min(state.input_buffer.len())];
let text_width = UnicodeWidthStr::width(text_before_cursor) as u16;
let cursor_col = (inner.x + icon_width + text_width).min(inner.x + inner.width);
```

### Verify
- Type CJK characters (e.g. "你好") — cursor should be 2 columns wide per character.
- Type emoji — cursor should position correctly after them.

---

## T05 · Refined dark palette

### Problem
Current palette is functional but flat. Needs more depth, contrast, and vibrancy.

### Files to change
- `src/theme/palette.rs`

### Steps

Replace the `locus_dark()` method body with these tuned values:

```rust
pub fn locus_dark() -> Self {
    Self {
        background: Rgb(8, 8, 12),
        surface_background: Rgb(16, 17, 24),
        elevated_surface_background: Rgb(22, 23, 32),
        border: Rgb(28, 30, 42),
        border_variant: Rgb(22, 24, 34),
        border_focused: Rgb(99, 148, 255),
        border_selected: Rgb(99, 148, 255),
        border_disabled: Rgb(50, 54, 80),
        element_background: Rgb(22, 23, 32),
        element_hover: Rgb(32, 36, 52),
        element_active: Rgb(32, 36, 52),
        element_selected: Rgb(32, 36, 52),
        element_disabled: Rgb(22, 23, 32),
        ghost_element_background: Rgb(0, 0, 0),
        ghost_element_hover: Rgb(28, 30, 42),
        ghost_element_selected: Rgb(32, 36, 52),
        ghost_element_disabled: Rgb(22, 24, 34),
        text: Rgb(200, 210, 245),
        text_muted: Rgb(70, 78, 110),
        text_placeholder: Rgb(55, 62, 90),
        text_disabled: Rgb(50, 54, 80),
        text_accent: Rgb(99, 148, 255),
        icon: Rgb(200, 210, 245),
        icon_muted: Rgb(70, 78, 110),
        icon_disabled: Rgb(50, 54, 80),
        icon_accent: Rgb(99, 148, 255),
        accent: Rgb(99, 148, 255),
        danger: Rgb(255, 100, 120),
        success: Rgb(120, 220, 120),
        warning: Rgb(240, 185, 100),
        info: Rgb(100, 200, 255),
        status_bar_background: Rgb(16, 17, 24),
        tab_bar_background: Rgb(16, 17, 24),
        tab_inactive_background: Rgb(16, 17, 24),
        tab_active_background: Rgb(8, 8, 12),
        panel_background: Rgb(16, 17, 24),
        panel_focused_border: Rgb(99, 148, 255),
        scrollbar_thumb_background: Rgb(50, 54, 80),
        scrollbar_thumb_hover_background: Rgb(70, 78, 110),
        scrollbar_track_background: Rgb(12, 12, 18),
        pane_focused_border: Rgb(99, 148, 255),
        editor_background: Rgb(8, 8, 12),
        editor_foreground: Rgb(200, 210, 245),
        editor_line_number: Rgb(70, 78, 110),
    }
}
```

### Verify
- All existing tests pass (palette values are not tested by equality).
- Visual: run `cargo run --bin locus -- tui` — darker background, more saturated accent blue, readable muted text.

---

## T06 · Header bar redesign — status badge with colored dot

### Problem
Header is a plain single line. No visual weight, no status indication color.

### Files to change
- `src/layouts/head.rs`
- `src/layouts/split.rs`
- `src/layouts/style.rs`
- `src/view.rs`
- `src/state.rs`

### Steps

**1. Add `is_streaming` field to `TuiState` in `src/state.rs`:**

```rust
/// Whether the AI is currently generating output.
pub is_streaming: bool,
```

Default to `false`.

**2. Set `is_streaming` in `src/runtime_events.rs`:**

In `SessionEvent::TurnStart`:
```rust
SessionEvent::TurnStart { role } => {
    if role == Role::Assistant {
        state.is_streaming = true;
        state.current_ai_text.clear();
        state.current_think_text.clear();
    }
}
```

In `SessionEvent::TurnEnd`:
```rust
SessionEvent::TurnEnd => {
    state.is_streaming = false;
    state.flush_turn();
}
```

**3. Add `warning_style` helper to `src/layouts/style.rs`:**

```rust
/// Style for warning state.
pub fn warning_style(warning_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(warning_rgb))
}
```

Also add re-export in `src/layouts/mod.rs`:
```rust
pub use style::{ ..., warning_style };
```

**4. Update `render_header` in `src/layouts/head.rs`:**

Add imports:
```rust
use ratatui::style::Modifier;
use super::style::{success_style, warning_style, danger_style};
```

Replace `render_header`:
```rust
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    palette: &LocusPalette,
    title: &str,
    status: &str,
    is_streaming: bool,
    has_error: bool,
) {
    let layout = HeadLayout::new(area);
    let block = block_for_head(&layout, palette);
    frame.render_widget(block, area);

    // Status dot: green=ready, yellow=streaming, red=error
    let (dot_char, dot_style) = if has_error {
        ("● ", danger_style(palette.danger))
    } else if is_streaming {
        ("● ", warning_style(palette.warning))
    } else {
        ("● ", success_style(palette.success))
    };

    let title_style = text_style(palette.text).add_modifier(Modifier::BOLD);
    let right_style = text_muted_style(palette.text_muted);

    // Build: "locus.codes" (bold)    "● Ready" (dot colored + muted text)
    let status_spans = vec![
        ratatui::text::Span::styled(dot_char.to_string(), dot_style),
        ratatui::text::Span::styled(status.to_string(), right_style),
    ];

    let gap = layout.inner.width.saturating_sub(
        title.len() as u16 + dot_char.len() as u16 + status.len() as u16
    );

    let line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(title.to_string(), title_style),
        ratatui::text::Span::raw(" ".repeat(gap as usize)),
        status_spans[0].clone(),
        status_spans[1].clone(),
    ]);

    let bg = background_style(palette.status_bar_background);
    frame.render_widget(ratatui::widgets::Paragraph::new(line).style(bg), layout.inner);
}
```

**5. Update call site in `src/view.rs`:**

Change:
```rust
render_header(frame, splits.header, palette, HEADER_TITLE, status);
```
To:
```rust
let has_error = state.status.contains("error") || state.status.contains("Error") || state.status.contains("failed");
render_header(frame, splits.header, palette, HEADER_TITLE, status, state.is_streaming, has_error);
```

### Verify
- Header shows bold title, colored status dot.
- Green dot when idle, yellow when streaming, red on error.

---

## T07 · Message visual separation — left border accent

### Problem
All messages share same background. Only icon differs. Hard to scan.

### Files to change
- `src/messages/user.rs`
- `src/messages/ai_message.rs`

### Steps

**1. User messages — accent left border in `src/messages/user.rs`:**

Change `USER_INDICATOR` from `"»"` to `"│"` and use `palette.accent` for it:
```rust
pub const USER_INDICATOR: &str = "│";
```

This gives a subtle vertical bar. First line:
```rust
Span::styled("│ ".to_string(), text_style(palette.accent)),
```

Continuation lines:
```rust
Span::styled("│ ".to_string(), text_style(palette.accent)),
Span::styled(seg.clone(), text_style(palette.text)),
```

Replace `LEFT_PADDING` in continuation lines with `"│ "` styled in accent.

**2. AI messages — muted left border in `src/messages/ai_message.rs`:**

Change `AI_INDICATOR` from `"▸"` to `"│"` with `palette.text_muted`:
```rust
pub const AI_INDICATOR: &str = "│";
```

First line:
```rust
Span::styled("│ ".to_string(), text_muted_style(palette.text_muted)),
```

Continuation lines use same muted bar instead of `LEFT_PADDING`.

### Verify
- User messages have blue `│` left border.
- AI messages have gray `│` left border.
- Clear visual distinction between speakers.

---

## T08 · Tool call — shimmer on running + live elapsed time

### Problem
Running tools show static `▶`. Done tools have no visual flair. No elapsed time while running.

### Files to change
- `src/messages/tool.rs`
- `src/state.rs`
- `src/view.rs`

### Steps

**1. Add `started_at` to `ToolCallMessage` in `src/messages/tool.rs`:**

```rust
use std::time::Instant;

pub struct ToolCallMessage {
    pub tool_name: String,
    pub status: ToolCallStatus,
    pub summary: Option<String>,
    pub started_at: Option<Instant>,
}
```

Update `running()` constructor:
```rust
pub fn running(tool_name: impl Into<String>, summary: Option<String>) -> Self {
    Self {
        tool_name: tool_name.into(),
        status: ToolCallStatus::Running,
        summary,
        started_at: Some(Instant::now()),
    }
}
```

Update `done()` and `error()` to set `started_at: None`.

**2. Show elapsed time for running tools in `tool_call_line`:**

In the `ToolCallStatus::Running` arm:
```rust
ToolCallStatus::Running => {
    spans.push(Span::styled("▶ ", text_style(palette.accent)));
    spans.push(Span::styled(msg.tool_name.clone(), text_style(palette.text)));
    if let Some(s) = &msg.summary {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(s.clone(), text_muted_style(palette.text_muted)));
    }
    if let Some(started) = msg.started_at {
        let elapsed = format_duration(started.elapsed());
        spans.push(Span::raw("  "));
        spans.push(Span::styled(elapsed, text_muted_style(palette.text_muted)));
    } else {
        spans.push(Span::raw(" …"));
    }
    return Line::from(spans);
}
```

(Remove the existing `Running` arm at the bottom of the function and handle it early like Done/Error.)

### Verify
- Running tools show `▶ bash ls  123ms` with live-updating elapsed time.
- Done tools show `✓ bash ls  450ms` as before.

---

## T09 · Collapsible thinking blocks

### Problem
Thinking text can be very long. No way to collapse/expand. Noisy.

### Files to change
- `src/messages/ai_think_message.rs`
- `src/state.rs`
- `src/run.rs`
- `src/view.rs`

### Steps

**1. Add `collapsed` to `AiThinkMessage` in `src/messages/ai_think_message.rs`:**

```rust
pub struct AiThinkMessage {
    pub text: String,
    pub collapsed: bool,
}
```

**2. Update `push_think` in `src/state.rs`:**

```rust
pub fn push_think(&mut self, text: String) {
    self.messages.push(ChatItem::Think(AiThinkMessage { text, collapsed: false }));
    self.maybe_scroll_bottom();
}
```

**3. Add collapsed rendering in `think_message_lines`:**

At the top of the function:
```rust
if msg.collapsed {
    let line_count = msg.text.lines().count();
    let summary = format!("⋯ Thinking ({} lines) ▸", line_count);
    return vec![Line::from(vec![
        Span::raw(LEFT_PADDING),
        Span::styled(summary, text_muted_style(palette.text_muted)),
    ])];
}
```

**4. Add toggle keybinding in `src/run.rs`:**

```rust
KeyCode::Char('t') if state.input_buffer.is_empty() => {
    // Toggle last thinking block
    for item in state.messages.iter_mut().rev() {
        if let ChatItem::Think(ref mut t) = item {
            t.collapsed = !t.collapsed;
            break;
        }
    }
}
```

Place this BEFORE the `KeyCode::Char(c)` arm.

**5. Update shortcut hint to mention `t` in `src/layouts/shortcut.rs`:**

```rust
pub const SHORTCUT_HINT: &str = "↑↓ PgUp/PgDn: scroll · t: toggle thinking · q: quit";
```

### Verify
- Press `t` when input is empty → last thinking block collapses to `⋯ Thinking (N lines) ▸`.
- Press `t` again → expands back.

---

## T10 · Blinking streaming cursor

### Problem
Streaming cursor `▌` is static. No visual motion.

### Files to change
- `src/state.rs`
- `src/run.rs`
- `src/messages/ai_message.rs`
- `src/messages/ai_think_message.rs`
- `src/view.rs`

### Steps

**1. Add frame counter to `TuiState` in `src/state.rs`:**

```rust
/// Frame counter for animations (incremented every loop iteration).
pub frame_count: u64,
```

Default to `0`.

**2. Increment in `src/run.rs` — at the start of each loop iteration:**

```rust
loop {
    state.frame_count = state.frame_count.wrapping_add(1);
    terminal.draw(|f| view::draw(f, state, f.area()))?;
    // ...
}
```

**3. Update `ai_message_lines` signature in `src/messages/ai_message.rs`:**

```rust
pub fn ai_message_lines(
    msg: &AiMessage,
    palette: &LocusPalette,
    width: usize,
    streaming: bool,
    frame_count: u64,
) -> Vec<Line<'static>> {
```

Replace cursor rendering:
```rust
// Instead of always showing STREAMING_CURSOR, blink it:
if streaming && /* is_last_line */ {
    let show_cursor = (frame_count / 5) % 2 == 0; // blink every ~500ms at 100ms poll
    if show_cursor {
        seg_spans.push(Span::styled(STREAMING_CURSOR.to_string(), text_style(palette.accent)));
    }
}
```

**4. Same for `think_message_lines` in `src/messages/ai_think_message.rs`.**

**5. Update call sites in `src/view.rs`:**

Pass `state.frame_count` to both `ai_message_lines` and `think_message_lines`.

### Verify
- While AI is streaming, cursor blinks on/off every ~500ms.
- When streaming stops, no cursor shown.

---

## T11 · Input bar — focus glow + placeholder + rounded border

### Problem
Input bar uses plain `palette.border`. No placeholder when empty. No visual invitation.

### Files to change
- `src/layouts/input.rs`
- `src/view.rs`

### Steps

**1. Update `block_for_input_bordered` in `src/layouts/input.rs`:**

```rust
use ratatui::widgets::BorderType;

pub fn block_for_input_bordered(palette: &LocusPalette, focused: bool) -> Block<'static> {
    let border_color = if focused {
        border_focused_style(palette.border_focused)
    } else {
        border_style(palette.border)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_color)
        .style(background_style(palette.surface_background))
        .padding(Padding::new(INPUT_PADDING_H, INPUT_PADDING_H, 0, 0))
}
```

Add import for `border_focused_style` from `super::style`.

**2. Update call in `src/view.rs`:**

```rust
let block = block_for_input_bordered(palette, true); // always focused for now
```

**3. Add placeholder text in `src/view.rs`:**

Replace the input line rendering:
```rust
let input_line = if state.input_buffer.is_empty() {
    ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(INPUT_ICON.to_string(), text_style(palette.accent)),
        ratatui::text::Span::styled("Ask anything…".to_string(), text_style(palette.text_placeholder)),
    ])
} else {
    ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(INPUT_ICON.to_string(), text_style(palette.success)),
        ratatui::text::Span::styled(state.input_buffer.as_str(), text_style(palette.text)),
    ])
};
```

Note: `text_style(palette.text_placeholder)` — add a `text_placeholder` parameter. Since `text_style` is generic (takes any `Rgb`), just pass `palette.text_placeholder`.

### Verify
- Input has rounded border (`╭╮╰╯`).
- Border is accent blue color.
- When empty, shows dim "Ask anything…" placeholder.
- When typing, icon turns green, text is bright.

---

## T12 · Scrollbar indicator

### Problem
No scrollbar. User has no idea where they are in chat history.

### Files to change
- `src/view.rs`

### Steps

**1. After rendering chat content in `src/view.rs`, render scrollbar:**

```rust
// Only show scrollbar when content exceeds viewport
if content_height > viewport_height {
    let max_scroll = content_height.saturating_sub(viewport_height);
    let scroll_ratio = if max_scroll > 0 {
        offset as f64 / max_scroll as f64
    } else {
        0.0
    };

    // Thumb size: proportional to viewport/content ratio, min 1 line
    let thumb_height = ((viewport_height as f64 / content_height as f64) * viewport_height as f64)
        .max(1.0) as u16;

    // Thumb position
    let scrollbar_track_height = chat.inner.height;
    let thumb_y = (scroll_ratio * (scrollbar_track_height.saturating_sub(thumb_height)) as f64) as u16;

    let scrollbar_x = chat.inner.x + chat.inner.width; // right edge

    // Render track and thumb
    for row in 0..scrollbar_track_height {
        let y = chat.inner.y + row;
        let (ch, style) = if row >= thumb_y && row < thumb_y + thumb_height {
            ("▐", text_muted_style(palette.scrollbar_thumb_hover_background))
        } else {
            (" ", background_style(palette.scrollbar_track_background))
        };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(ch).style(style),
            Rect::new(scrollbar_x.min(chat.area.x + chat.area.width - 1), y, 1, 1),
        );
    }
}
```

**2. Reduce chat inner width by 1 to make room for scrollbar:**

In the chat layout section, when content exceeds viewport:
```rust
let chat_width = if content_height > viewport_height {
    chat.inner.width.saturating_sub(1)
} else {
    chat.inner.width
};
let width = chat_width as usize;
```

### Verify
- Scrollbar appears only when chat overflows.
- Thumb position reflects scroll position.
- Scrollbar disappears when all content fits.

---

## T13 · Context-aware shortcut hints

### Problem
Static shortcut string. Shows "q: quit" while user is typing (confusing).

### Files to change
- `src/layouts/shortcut.rs`
- `src/view.rs`

### Steps

**1. Replace static shortcut with dynamic function in `src/layouts/shortcut.rs`:**

```rust
/// Build context-aware shortcut line.
pub fn shortcut_line_dynamic(
    palette: &LocusPalette,
    is_streaming: bool,
    has_input: bool,
) -> Line<'static> {
    let muted = text_muted_style(palette.text_muted);
    let dim = text_muted_style(palette.text_disabled);
    let sep = Span::styled(" · ", dim);

    let spans = if is_streaming {
        vec![
            Span::styled("Streaming…", muted),
            sep,
            Span::styled("Ctrl+C", muted),
            Span::styled(": cancel", dim),
        ]
    } else if has_input {
        vec![
            Span::styled("Enter", muted),
            Span::styled(": send", dim),
            sep.clone(),
            Span::styled("Ctrl+U", muted),
            Span::styled(": clear", dim),
            sep,
            Span::styled("Ctrl+C", muted),
            Span::styled(": quit", dim),
        ]
    } else {
        vec![
            Span::styled("↑↓", muted),
            Span::styled(": scroll", dim),
            sep.clone(),
            Span::styled("t", muted),
            Span::styled(": thinking", dim),
            sep.clone(),
            Span::styled("q", muted),
            Span::styled(": quit", dim),
            sep,
            Span::styled("Ctrl+C", muted),
            Span::styled(": quit", dim),
        ]
    };

    Line::from(spans)
}
```

**2. Update `src/view.rs`:**

Replace:
```rust
frame.render_widget(Paragraph::new(shortcut_line(palette)), shortcut_inner);
```
With:
```rust
frame.render_widget(
    Paragraph::new(shortcut_line_dynamic(palette, state.is_streaming, !state.input_buffer.is_empty())),
    shortcut_inner,
);
```

Add import for `shortcut_line_dynamic` in layouts mod.rs re-exports.

### Verify
- When streaming: shows "Streaming… · Ctrl+C: cancel"
- When typing: shows "Enter: send · Ctrl+U: clear · Ctrl+C: quit"
- When idle: shows "↑↓: scroll · t: thinking · q: quit · Ctrl+C: quit"

---

## T14 · Line cache — avoid rebuilding every frame

### Problem
`view.rs` rebuilds all lines from all messages every frame. O(n) per 100ms.

### Files to change
- `src/state.rs`
- `src/view.rs`

### Steps

**1. Add cache fields to `TuiState` in `src/state.rs`:**

```rust
use ratatui::text::Line;

/// Cached rendered lines from committed messages (not streaming content).
pub cached_lines: Vec<Line<'static>>,
/// Whether cache needs rebuild.
pub cache_dirty: bool,
```

Default: `cached_lines: Vec::new()`, `cache_dirty: true`.

Set `cache_dirty = true` in: `push_user`, `push_ai`, `push_think`, `push_tool`, `push_meta_tool`, `push_error`, `flush_turn`.

**2. Update `view.rs` to use cache:**

```rust
if state.cache_dirty {
    // Rebuild from state.messages
    let mut all_lines: Vec<Line> = Vec::new();
    let spacer = Line::from("");
    for item in &state.messages {
        if !all_lines.is_empty() {
            all_lines.push(spacer.clone());
        }
        match item {
            // ... existing match arms ...
        }
    }
    state.cached_lines = all_lines;
    state.cache_dirty = false;
}

// Start with cached lines, append streaming content
let mut all_lines = state.cached_lines.clone();
// ... append current_think_text, current_ai_text as before ...
```

**Note:** `state` parameter in `draw` must change from `&TuiState` to `&mut TuiState` for cache mutation.

Update the `draw` signature:
```rust
pub fn draw(frame: &mut Frame, state: &mut TuiState, area: Rect) {
```

Update call in `run.rs`:
```rust
terminal.draw(|f| view::draw(f, state, f.area()))?;
// This already works since state is &mut
```

### Verify
- Correctness: all messages render identically to before.
- Performance: with 1000 messages, only streaming content is rebuilt per frame.

---

## T15 · Skip identical frames

### Problem
`terminal.draw()` runs every 100ms even when nothing changed.

### Files to change
- `src/state.rs`
- `src/run.rs`

### Steps

**1. Add `needs_redraw` to `TuiState` in `src/state.rs`:**

```rust
pub needs_redraw: bool,
```

Default to `true`.

Set `needs_redraw = true` in: all `push_*`, `input_insert`, `input_backspace`, `input_cursor_left/right/home/end`, `input_delete`, `input_clear`, `input_kill_to_end`, `input_take`, `scroll_up`, `scroll_down`, `flush_turn`.

**2. Update `src/run.rs`:**

```rust
loop {
    state.frame_count = state.frame_count.wrapping_add(1);

    // Always redraw when streaming (animations need continuous frames)
    let is_animating = !state.current_ai_text.is_empty() || !state.current_think_text.is_empty() || state.is_streaming;

    if state.needs_redraw || is_animating {
        terminal.draw(|f| view::draw(f, state, f.area()))?;
        state.needs_redraw = false;
    }

    // Drain session events — mark redraw if any received
    if let Some(ref mut rx) = event_rx {
        while let Ok(event) = rx.try_recv() {
            apply_session_event(state, event);
            state.needs_redraw = true;
        }
    }

    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(e) => { /* existing — each handler already sets needs_redraw via methods */ }
            Event::Resize(_, _) => { state.needs_redraw = true; }
            _ => {}
        }
    }
}
```

### Verify
- Idle TUI doesn't redraw (check with debug counter if needed).
- Typing, scrolling, streaming all redraw immediately.
- Terminal resize triggers redraw.

---

## T16 · Smart scroll clamp

### Problem
`scroll_up` can scroll indefinitely past content. User sees blank space.

### Files to change
- `src/state.rs`
- `src/view.rs`

### Steps

**1. Add viewport tracking to `TuiState` in `src/state.rs`:**

```rust
/// Last known content height (lines). Updated after each draw.
pub last_content_height: usize,
/// Last known viewport height (lines). Updated after each draw.
pub last_viewport_height: usize,
```

Default both to `0`.

**2. Update `scroll_up` to clamp:**

```rust
pub fn scroll_up(&mut self, delta: usize) {
    let max_scroll = self.last_content_height.saturating_sub(self.last_viewport_height);
    self.scroll = self.scroll.saturating_add(delta).min(max_scroll);
    self.needs_redraw = true;
}
```

**3. In `src/view.rs`, after computing `content_height` and `viewport_height`:**

```rust
state.last_content_height = content_height;
state.last_viewport_height = viewport_height;
```

### Verify
- Scroll up stops at the first message (can't scroll past).
- Scroll down stops at bottom (scroll == 0).

---

## T20 · Typing indicator while AI is thinking (no output yet)

### Problem
When model is processing before emitting text, UI shows nothing. Looks frozen.

### Files to change
- `src/view.rs`
- `src/animation/shimmer.rs`
- `src/state.rs`

### Steps

**1. Add a `Shimmer` instance to `TuiState` in `src/state.rs`:**

```rust
use crate::animation::Shimmer;

pub shimmer: Shimmer,
```

Default: `shimmer: Shimmer::new()`.

**2. In `src/view.rs`, after streaming content, if still empty:**

```rust
if state.is_streaming && state.current_ai_text.is_empty() && state.current_think_text.is_empty() {
    if !all_lines.is_empty() {
        all_lines.push(spacer.clone());
    }
    state.shimmer.tick();
    let indicator_spans = state.shimmer.styled_spans_with_palette("Thinking…", palette);
    all_lines.push(Line::from(vec![
        Span::raw(LEFT_PADDING),
    ].into_iter().chain(indicator_spans.into_iter()).collect::<Vec<_>>()));
}
```

### Verify
- When AI is called but no tokens arrived yet, "Thinking…" shimmers across the screen.
- Disappears as soon as any text delta arrives.

---

## T21 · Session separator line

### Problem
No visual boundary between conversation turns / sessions.

### Files to change
- `src/state.rs`
- `src/view.rs`
- `src/runtime_events.rs`

### Steps

**1. Add `ChatItem::Separator(String)` to `src/state.rs`:**

```rust
Separator(String),
```

Add method:
```rust
pub fn push_separator(&mut self, label: String) {
    self.messages.push(ChatItem::Separator(label));
}
```

**2. Render in `src/view.rs`:**

```rust
ChatItem::Separator(label) => {
    let dashes = "─".repeat(((width as i32 - label.len() as i32 - 4).max(0) / 2) as usize);
    let line_text = format!("{} {} {}", dashes, label, dashes);
    all_lines.push(Line::from(vec![
        Span::styled(line_text, text_muted_style(palette.text_disabled)),
    ]));
}
```

**3. Push separator on session end in `src/runtime_events.rs`:**

```rust
SessionEvent::SessionEnd { .. } => {
    state.flush_turn();
    state.push_separator("Session ended".to_string());
    state.status = "Session ended".to_string();
}
```

### Verify
- Session end shows centered `──── Session ended ────` in dim text.

---

## T22 · Timestamps on AI messages

### Problem
AI messages have no timestamp. Can't tell when they were generated.

### Files to change
- `Cargo.toml`
- `src/messages/ai_message.rs`
- `src/state.rs`

### Steps

**1. Add `chrono` to `Cargo.toml`:**

```toml
chrono = "0.4"
```

**2. Add `timestamp` to `AiMessage` in `src/messages/ai_message.rs`:**

```rust
pub struct AiMessage {
    pub text: String,
    pub timestamp: Option<String>,
}
```

Update rendering — add timestamp after indicator on first line (same pattern as user.rs):
```rust
if let Some(t) = &msg.timestamp {
    first_spans.push(Span::styled(format!("{} ", t), text_muted_style(palette.text_muted)));
}
```

**3. Update `flush_turn` in `src/state.rs`:**

```rust
use chrono::Local;

pub fn flush_turn(&mut self) {
    let timestamp = Some(Local::now().format("%H:%M").to_string());
    let think = std::mem::take(&mut self.current_think_text);
    if !think.is_empty() {
        self.push_think(think);
    }
    let ai = std::mem::take(&mut self.current_ai_text);
    if !ai.is_empty() {
        self.messages.push(ChatItem::Ai(AiMessage { text: ai, timestamp }));
        self.maybe_scroll_bottom();
    }
    self.cache_dirty = true;
    self.needs_redraw = true;
}
```

**4. Update `push_ai` in `src/state.rs`:**

```rust
pub fn push_ai(&mut self, text: String) {
    self.messages.push(ChatItem::Ai(AiMessage { text, timestamp: None }));
    self.maybe_scroll_bottom();
}
```

### Verify
- AI messages from runtime show timestamp like `14:32` in muted text.
- Manual `push_ai` (echo mode) has no timestamp.

---

## T23 · Welcome screen (empty state)

### Problem
Empty chat on first launch. No guidance.

### Files to change
- `src/view.rs`

### Steps

In `view.rs`, before the content rendering, check for empty state:

```rust
if state.messages.is_empty() && !state.is_streaming {
    // Render centered welcome
    let title = "locus.codes";
    let subtitle = "Type a message to begin.";

    state.shimmer.tick();
    let title_spans = state.shimmer.styled_spans_with_palette(title, palette);

    let mid_y = chat.inner.y + chat.inner.height / 2;
    let title_x = chat.inner.x + (chat.inner.width.saturating_sub(title.len() as u16)) / 2;
    let sub_x = chat.inner.x + (chat.inner.width.saturating_sub(subtitle.len() as u16)) / 2;

    frame.render_widget(
        Paragraph::new(Line::from(title_spans)),
        Rect::new(title_x, mid_y.saturating_sub(1), chat.inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(subtitle.to_string(), text_muted_style(palette.text_muted)),
        ])),
        Rect::new(sub_x, mid_y.saturating_add(1), chat.inner.width, 1),
    );
} else {
    // ... existing message rendering ...
}
```

Import `LEFT_PADDING` if not already imported, and import `text_muted_style` at the view level.

### Verify
- Launch TUI with no messages → centered "locus.codes" with shimmer + dim subtitle.
- Send a message → welcome disappears, chat renders normally.

---

## T24 · Status bar auto-clear timeout

### Problem
Status text persists forever until overwritten.

### Files to change
- `src/state.rs`
- `src/run.rs`

### Steps

**1. Add timeout tracking to `TuiState` in `src/state.rs`:**

```rust
use std::time::Instant;

/// When the current status was set (None = permanent, don't auto-clear).
pub status_set_at: Option<Instant>,
```

Default to `None`.

**2. Add a set_status helper:**

```rust
/// Set a transient status (auto-clears after timeout).
pub fn set_status(&mut self, message: String) {
    self.status = message;
    self.status_set_at = Some(Instant::now());
    self.needs_redraw = true;
}

/// Set a permanent status (won't auto-clear).
pub fn set_status_permanent(&mut self, message: String) {
    self.status = message;
    self.status_set_at = None;
    self.needs_redraw = true;
}
```

**3. Update `src/runtime_events.rs` to use helpers:**

```rust
SessionEvent::Status { message } => {
    state.set_status(message);
}
SessionEvent::Error { error } => {
    state.push_error(error.clone());
    state.set_status(error);
}
SessionEvent::SessionEnd { .. } => {
    state.flush_turn();
    state.push_separator("Session ended".to_string());
    state.set_status_permanent("Session ended".to_string());
}
```

**4. Auto-clear in `src/run.rs` — at the start of each loop:**

```rust
// Auto-clear transient status after 5 seconds
if let Some(set_at) = state.status_set_at {
    if set_at.elapsed() > Duration::from_secs(5) {
        state.status.clear();
        state.status_set_at = None;
        state.needs_redraw = true;
    }
}
```

### Verify
- Status messages ("Ready", errors) clear after 5 seconds.
- "Session ended" stays forever.

---

## T25 · Message rendering tests

### Files to change
- `src/messages/user.rs` (add tests)
- `src/messages/ai_message.rs` (add tests)
- `src/messages/ai_think_message.rs` (add tests)
- `src/messages/tool.rs` (add tests)
- `src/messages/error.rs` (add tests after T02)

### Tests to add

```rust
// In each file's #[cfg(test)] mod tests:

// Empty text
#[test]
fn empty_text_still_renders() {
    let msg = /* empty text */;
    let lines = /* render */;
    assert!(!lines.is_empty());
}

// Unicode / emoji
#[test]
fn unicode_text_renders() {
    let msg = /* "Hello 🎉 世界" */;
    let lines = /* render */;
    assert!(!lines.is_empty());
}

// Very long single word (no wrap points)
#[test]
fn long_word_no_panic() {
    let msg = /* "a".repeat(500) */;
    let lines = /* render with width=20 */;
    assert!(lines.len() >= 1);
}

// Streaming cursor present/absent
#[test]
fn streaming_cursor_shown() {
    // streaming=true → last span contains STREAMING_CURSOR
}

#[test]
fn no_streaming_cursor_when_not_streaming() {
    // streaming=false → no STREAMING_CURSOR in spans
}
```

---

## T26 · Layout tests

### Files to change
- `src/layouts/split.rs` (add tests)
- `src/layouts/head.rs` (add tests)
- `src/layouts/shortcut.rs` (add tests)

### Tests to add

```rust
// Tiny terminal (smaller than header + footer)
#[test]
fn main_splits_tiny_terminal() {
    let area = Rect::new(0, 0, 80, 3); // smaller than HEADER + FOOTER
    let s = main_splits(area);
    assert_eq!(s.body.height, 0); // body collapses gracefully
}

// Zero-width area
#[test]
fn shortcut_inner_rect_zero_width() {
    let area = Rect::new(0, 0, 0, 1);
    let inner = shortcut_inner_rect(area);
    assert_eq!(inner.width, 0);
}
```

---

## T27 · State tests

### Files to change
- `src/state.rs` (add `#[cfg(test)] mod tests`)

### Tests to add

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_insert_ascii() {
        let mut s = TuiState::new();
        s.input_insert('a');
        s.input_insert('b');
        assert_eq!(s.input_buffer, "ab");
        assert_eq!(s.input_cursor, 2);
    }

    #[test]
    fn input_insert_emoji() {
        let mut s = TuiState::new();
        s.input_insert('🎉');
        assert_eq!(s.input_buffer, "🎉");
        assert_eq!(s.input_cursor, 4); // 🎉 is 4 bytes
    }

    #[test]
    fn input_backspace_at_zero() {
        let mut s = TuiState::new();
        s.input_backspace(); // should not panic
        assert_eq!(s.input_cursor, 0);
    }

    #[test]
    fn input_backspace_multibyte() {
        let mut s = TuiState::new();
        s.input_insert('你');
        s.input_insert('好');
        s.input_backspace();
        assert_eq!(s.input_buffer, "你");
    }

    #[test]
    fn input_take_resets() {
        let mut s = TuiState::new();
        s.input_insert('x');
        let taken = s.input_take();
        assert_eq!(taken, "x");
        assert_eq!(s.input_buffer, "");
        assert_eq!(s.input_cursor, 0);
    }

    #[test]
    fn flush_turn_pushes_and_clears() {
        let mut s = TuiState::new();
        s.current_ai_text = "hello".to_string();
        s.current_think_text = "thinking".to_string();
        s.flush_turn();
        assert!(s.current_ai_text.is_empty());
        assert!(s.current_think_text.is_empty());
        assert_eq!(s.messages.len(), 2); // think + ai
    }

    #[test]
    fn scroll_up_down() {
        let mut s = TuiState::new();
        s.last_content_height = 100;
        s.last_viewport_height = 20;
        s.scroll_up(5);
        assert_eq!(s.scroll, 5);
        s.scroll_down(3);
        assert_eq!(s.scroll, 2);
        s.scroll_down(10);
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn auto_scroll_resets_on_push() {
        let mut s = TuiState::new();
        s.scroll = 10;
        s.auto_scroll = true;
        s.push_user("hi".to_string(), None);
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn push_variants() {
        let mut s = TuiState::new();
        s.push_user("u".to_string(), None);
        s.push_ai("a".to_string());
        s.push_think("t".to_string());
        assert_eq!(s.messages.len(), 3);
    }
}
```

---

## Implementation order

```
Batch 1 (Core UX):       T01 → T02 → T03 → T04
Batch 2 (Visual):        T05 → T11 → T06 → T07 → T12 → T13
Batch 3 (Smooth):        T10 → T20 → T23 → T08 → T09
Batch 4 (Performance):   T14 → T15 → T16
Batch 5 (Polish):        T21 → T22 → T24
Batch 6 (Tests):         T25 → T26 → T27
```

After each task: `cargo check -p locus-tui && cargo test -p locus-tui && cargo clippy -p locus-tui`

---

## Dependencies to add to `Cargo.toml`

```toml
# T04 — cursor width
unicode-width = "0.2"
# T22 — timestamps
chrono = "0.4"
```

No other external crates needed. Everything else uses existing ratatui, crossterm, tokio, anyhow.
