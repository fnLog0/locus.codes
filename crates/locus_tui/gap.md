# locus_tui — Gap List

Remaining gaps from `plan.md`. Fix each item, then verify with:
```bash
cargo check -p locus-tui && cargo test -p locus-tui && cargo clippy -p locus-tui
```

---

## G01 · Clippy warning — collapsible if in run.rs

**File:** `src/run.rs` lines 101–109

**Current:**
```rust
let _reader = std::thread::spawn(move || {
    loop {
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(ev) = event::read() {
                let _ = key_tx.send(ev);
            }
        }
    }
});
```

**Fix:** Collapse the nested `if` into one:
```rust
let _reader = std::thread::spawn(move || {
    loop {
        if event::poll(Duration::from_millis(50)).unwrap_or(false)
            && let Ok(ev) = event::read()
        {
            let _ = key_tx.send(ev);
        }
    }
});
```

**Verify:** `cargo clippy -p locus-tui` — zero warnings.

---

## G02 · Header redesign — bold title + status dot (T06 incomplete)

**Files:** `src/layouts/head.rs`, `src/layouts/style.rs`, `src/layouts/mod.rs`, `src/view.rs`

**Current:** `render_header` takes `(frame, area, palette, title, status)`. No bold, no colored dot, no streaming/error awareness.

### Steps

**1. Add `warning_style` to `src/layouts/style.rs`:**
```rust
/// Style for warning state.
pub fn warning_style(warning_rgb: Rgb) -> Style {
    Style::default().fg(rgb_to_color(warning_rgb))
}
```

**2. Re-export in `src/layouts/mod.rs`:**
```rust
pub use style::{ ..., warning_style };
```

**3. Change `render_header` signature in `src/layouts/head.rs`:**
```rust
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    palette: &LocusPalette,
    title: &str,
    status: &str,
    is_streaming: bool,
    has_error: bool,
)
```

Add imports:
```rust
use ratatui::style::Modifier;
use super::style::{success_style, warning_style, danger_style};
```

**4. Inside `render_header`, build status dot + bold title:**
```rust
let layout = HeadLayout::new(area);
let block = block_for_head(&layout, palette);
frame.render_widget(block, area);

let title_style = text_style(palette.text).add_modifier(Modifier::BOLD);

// Status dot: ● green=ready, yellow=streaming, red=error
let (dot, dot_style) = if has_error {
    ("● ", danger_style(palette.danger))
} else if is_streaming {
    ("● ", warning_style(palette.warning))
} else {
    ("● ", success_style(palette.success))
};

let right_style = text_muted_style(palette.text_muted);
let dot_len = dot.len() as u16;
let title_len = title.len() as u16;
let status_len = status.len() as u16;
let gap = layout.inner.width.saturating_sub(title_len + dot_len + status_len);

let line = Line::from(vec![
    Span::styled(title.to_string(), title_style),
    Span::raw(" ".repeat(gap as usize)),
    Span::styled(dot.to_string(), dot_style),
    Span::styled(status.to_string(), right_style),
]);

let bg = background_style(palette.status_bar_background);
frame.render_widget(Paragraph::new(line).style(bg), layout.inner);
```

**5. Update call site in `src/view.rs` `draw_main`:**

Replace:
```rust
render_header(frame, splits.header, palette, HEADER_TITLE, status);
```
With:
```rust
let has_error = state.status.to_lowercase().contains("error")
    || state.status.to_lowercase().contains("failed");
render_header(
    frame,
    splits.header,
    palette,
    HEADER_TITLE,
    status,
    state.is_streaming,
    has_error,
);
```

**Verify:**
- Header shows **bold** "locus.codes".
- Green `●` when idle, yellow `●` when streaming, red `●` on error status.

---

## G03 · Missing message tests (T25 gaps)

**File:** `src/messages/error.rs` — add `#[cfg(test)] mod tests`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_empty_text() {
        let msg = ErrorMessage { text: "".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(!lines.is_empty());
    }

    #[test]
    fn error_wraps_long_text() {
        let msg = ErrorMessage {
            text: "Connection refused: could not connect to provider endpoint after multiple retries".into(),
            timestamp: None,
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 30);
        assert!(lines.len() > 1);
    }

    #[test]
    fn error_with_timestamp() {
        let msg = ErrorMessage {
            text: "timeout".into(),
            timestamp: Some("14:30".into()),
        };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("14:30")));
    }

    #[test]
    fn error_has_danger_icon() {
        let msg = ErrorMessage { text: "fail".into(), timestamp: None };
        let palette = LocusPalette::locus_dark();
        let lines = error_message_lines(&msg, &palette, 40);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("✗")));
    }
}
```

**File:** `src/messages/ai_message.rs` — add to existing `mod tests`:

```rust
#[test]
fn ai_message_empty_text() {
    let msg = AiMessage { text: "".into(), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = ai_message_lines(&msg, &palette, 40, false, true);
    assert!(!lines.is_empty());
}

#[test]
fn ai_message_unicode_emoji() {
    let msg = AiMessage { text: "Hello 🎉 世界 done".into(), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = ai_message_lines(&msg, &palette, 40, false, true);
    assert!(!lines.is_empty());
}

#[test]
fn ai_message_streaming_cursor_shown() {
    let msg = AiMessage { text: "partial".into(), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = ai_message_lines(&msg, &palette, 40, true, true);
    let has_cursor = lines.iter().any(|l| {
        l.spans.iter().any(|s| s.content.as_ref() == STREAMING_CURSOR)
    });
    assert!(has_cursor);
}

#[test]
fn ai_message_no_cursor_when_not_streaming() {
    let msg = AiMessage { text: "done".into(), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = ai_message_lines(&msg, &palette, 40, false, true);
    let has_cursor = lines.iter().any(|l| {
        l.spans.iter().any(|s| s.content.as_ref() == STREAMING_CURSOR)
    });
    assert!(!has_cursor);
}

#[test]
fn ai_message_with_timestamp() {
    let msg = AiMessage { text: "hi".into(), timestamp: Some("10:30".into()) };
    let palette = LocusPalette::locus_dark();
    let lines = ai_message_lines(&msg, &palette, 40, false, true);
    assert!(lines[0].spans.iter().any(|s| s.content.contains("10:30")));
}

#[test]
fn ai_message_long_single_word() {
    let msg = AiMessage { text: "a".repeat(500), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = ai_message_lines(&msg, &palette, 20, false, true);
    assert!(!lines.is_empty());
}
```

**File:** `src/messages/ai_think_message.rs` — add to existing `mod tests`:

```rust
#[test]
fn think_empty_text() {
    let msg = AiThinkMessage { text: "".into(), collapsed: false };
    let palette = LocusPalette::locus_dark();
    let lines = think_message_lines(&msg, &palette, 40, false, true, None);
    assert!(!lines.is_empty());
}

#[test]
fn think_collapsed_shows_line_count() {
    let msg = AiThinkMessage {
        text: "line 1\nline 2\nline 3".into(),
        collapsed: true,
    };
    let palette = LocusPalette::locus_dark();
    let lines = think_message_lines(&msg, &palette, 40, false, true, None);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].spans.iter().any(|s| s.content.contains("3 lines")));
}

#[test]
fn think_streaming_cursor_shown() {
    let msg = AiThinkMessage { text: "thinking".into(), collapsed: false };
    let palette = LocusPalette::locus_dark();
    let lines = think_message_lines(&msg, &palette, 40, true, true, None);
    let has_cursor = lines.iter().any(|l| {
        l.spans.iter().any(|s| s.content.as_ref() == STREAMING_CURSOR)
    });
    assert!(has_cursor);
}

#[test]
fn think_streaming_truncated_shows_ellipsis() {
    let text = (0..20).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    let msg = AiThinkMessage { text, collapsed: false };
    let palette = LocusPalette::locus_dark();
    let lines = think_message_lines(&msg, &palette, 40, true, true, Some(3));
    assert!(lines[0].spans.iter().any(|s| s.content.as_ref() == "…"));
}

#[test]
fn think_unicode() {
    let msg = AiThinkMessage { text: "考虑方案 🤔 思考中".into(), collapsed: false };
    let palette = LocusPalette::locus_dark();
    let lines = think_message_lines(&msg, &palette, 40, false, true, None);
    assert!(!lines.is_empty());
}
```

**File:** `src/messages/user.rs` — add to existing `mod tests`:

```rust
#[test]
fn user_message_empty_text() {
    let msg = UserMessage { text: "".into(), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = user_message_lines(&msg, &palette, 40);
    assert!(!lines.is_empty());
}

#[test]
fn user_message_with_timestamp() {
    let msg = UserMessage { text: "hi".into(), timestamp: Some("09:15".into()) };
    let palette = LocusPalette::locus_dark();
    let lines = user_message_lines(&msg, &palette, 40);
    assert!(lines[0].spans.iter().any(|s| s.content.contains("09:15")));
}

#[test]
fn user_message_emoji() {
    let msg = UserMessage { text: "Hello 🌍🎉".into(), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = user_message_lines(&msg, &palette, 40);
    assert!(!lines.is_empty());
}

#[test]
fn user_message_has_left_border() {
    let msg = UserMessage { text: "hi".into(), timestamp: None };
    let palette = LocusPalette::locus_dark();
    let lines = user_message_lines(&msg, &palette, 40);
    assert!(lines[0].spans.iter().any(|s| s.content.contains("│")));
}
```

**File:** `src/messages/tool.rs` — add to existing `mod tests`:

```rust
#[test]
fn tool_call_done_success() {
    let msg = ToolCallMessage::done("edit_file", 150, true, Some("src/main.rs".into()));
    let palette = LocusPalette::locus_dark();
    let lines = tool_call_lines(&msg, &palette, None, None, false);
    assert!(!lines.is_empty());
    assert!(lines[0].spans.iter().any(|s| s.content.contains("✓")));
}

#[test]
fn tool_call_done_failure() {
    let msg = ToolCallMessage::done("bash", 300, false, None);
    let palette = LocusPalette::locus_dark();
    let lines = tool_call_lines(&msg, &palette, None, None, false);
    assert!(lines[0].spans.iter().any(|s| s.content.contains("✗")));
}

#[test]
fn tool_call_error_two_lines() {
    let msg = ToolCallMessage::error("grep", "file not found", None);
    let palette = LocusPalette::locus_dark();
    let lines = tool_call_lines(&msg, &palette, None, None, false);
    assert_eq!(lines.len(), 2);
}

#[test]
fn tool_call_running_with_elapsed() {
    let msg = ToolCallMessage::running("bash", Some("ls".into()));
    let palette = LocusPalette::locus_dark();
    let lines = tool_call_lines(&msg, &palette, Some(1234), None, false);
    assert!(lines[0].spans.iter().any(|s| s.content.contains("1s")));
}

#[test]
fn tool_call_grouped_indent() {
    let msg = ToolCallMessage::running("bash", None);
    let palette = LocusPalette::locus_dark();
    let lines = tool_call_lines(&msg, &palette, None, None, true);
    // Grouped tools use 4-space indent instead of 2-space
    assert!(lines[0].spans[0].content.starts_with("    "));
}
```

**File:** `src/messages/meta_tool.rs` — add to existing `mod tests`:

```rust
#[test]
fn meta_tool_done_success() {
    let msg = MetaToolMessage::done(MetaToolKind::ToolSearch, 200, true, Some("find files".into()));
    let palette = LocusPalette::locus_dark();
    let line = meta_tool_line(&msg, &palette);
    assert!(line.spans.iter().any(|s| s.content.contains("✓")));
}

#[test]
fn meta_tool_error_shows_message() {
    let msg = MetaToolMessage::error(MetaToolKind::Task, "timed out", None);
    let palette = LocusPalette::locus_dark();
    let line = meta_tool_line(&msg, &palette);
    assert!(line.spans.iter().any(|s| s.content.contains("timed out")));
}

#[test]
fn meta_tool_all_kinds_parse() {
    assert!(MetaToolKind::from_name("tool_search").is_some());
    assert!(MetaToolKind::from_name("tool_explain").is_some());
    assert!(MetaToolKind::from_name("task").is_some());
    assert!(MetaToolKind::from_name("unknown").is_none());
}
```

---

## G04 · Missing layout tests (T26 gaps)

**File:** `src/layouts/split.rs` — add to existing `mod tests`:

```rust
#[test]
fn main_splits_tiny_terminal() {
    let area = Rect::new(0, 0, 80, 3);
    let s = main_splits(area);
    // Body should collapse to 0 when terminal too small
    assert_eq!(s.body.height, 0);
    assert_eq!(s.header.height, HEADER_HEIGHT);
}

#[test]
fn main_splits_exact_minimum() {
    let area = Rect::new(0, 0, 80, HEADER_HEIGHT + FOOTER_HEIGHT);
    let s = main_splits(area);
    assert_eq!(s.body.height, 0);
}

#[test]
fn vertical_split_larger_than_area() {
    let area = Rect::new(0, 0, 80, 5);
    let (top, bottom) = vertical_split(area, 10);
    assert_eq!(top.height, 5);
    assert_eq!(bottom.height, 0);
}

#[test]
fn horizontal_split_zero_width() {
    let area = Rect::new(0, 0, 0, 24);
    let (left, right) = horizontal_split(area, 10);
    assert_eq!(left.width, 0);
    assert_eq!(right.width, 0);
}
```

**File:** `src/layouts/shortcut.rs` — add `#[cfg(test)] mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_inner_rect_zero_width() {
        let area = Rect::new(0, 0, 0, 1);
        let inner = shortcut_inner_rect(area);
        assert_eq!(inner.width, 0);
    }

    #[test]
    fn shortcut_inner_rect_small_width() {
        let area = Rect::new(0, 0, 4, 1);
        let inner = shortcut_inner_rect(area);
        assert!(inner.width <= area.width);
    }

    #[test]
    fn shortcut_line_streaming() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, true, false);
        assert!(line.spans.iter().any(|s| s.content.contains("Streaming")));
    }

    #[test]
    fn shortcut_line_typing() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, false, true);
        assert!(line.spans.iter().any(|s| s.content.contains("Enter")));
    }

    #[test]
    fn shortcut_line_idle() {
        let palette = LocusPalette::locus_dark();
        let line = shortcut_line(&palette, false, false);
        assert!(line.spans.iter().any(|s| s.content.contains("scroll")));
    }
}
```

**File:** `src/layouts/chats.rs` — add to existing `mod tests`:

```rust
#[test]
fn chats_layout_zero_size() {
    let area = Rect::new(0, 0, 0, 0);
    let layout = ChatsLayout::new(area);
    assert_eq!(layout.inner.width, 0);
    assert_eq!(layout.inner.height, 0);
}

#[test]
fn chat_scroll_offset_no_overflow() {
    // Content fits in viewport — offset should be 0
    assert_eq!(chat_scroll_offset(5, 10, 20), 0);
}

#[test]
fn chat_scroll_offset_clamped() {
    // Scroll beyond content — clamp to max
    let offset = chat_scroll_offset(100, 50, 20);
    assert!(offset <= 50);
}
```

---

## G05 · State tests — cursor movement + new methods (T27 gaps)

**File:** `src/state.rs` — add to existing `mod tests`:

```rust
#[test]
fn input_cursor_left_right() {
    let mut s = TuiState::new();
    s.input_insert('a');
    s.input_insert('b');
    s.input_insert('c');
    assert_eq!(s.input_cursor, 3);
    s.input_cursor_left();
    assert_eq!(s.input_cursor, 2);
    s.input_cursor_left();
    assert_eq!(s.input_cursor, 1);
    s.input_cursor_right();
    assert_eq!(s.input_cursor, 2);
}

#[test]
fn input_cursor_left_at_zero() {
    let mut s = TuiState::new();
    s.input_cursor_left();
    assert_eq!(s.input_cursor, 0);
}

#[test]
fn input_cursor_right_at_end() {
    let mut s = TuiState::new();
    s.input_insert('x');
    s.input_cursor_right();
    assert_eq!(s.input_cursor, 1); // already at end, no change
}

#[test]
fn input_cursor_home_end() {
    let mut s = TuiState::new();
    s.input_insert('a');
    s.input_insert('b');
    s.input_insert('c');
    s.input_cursor_home();
    assert_eq!(s.input_cursor, 0);
    s.input_cursor_end();
    assert_eq!(s.input_cursor, 3);
}

#[test]
fn input_delete_forward() {
    let mut s = TuiState::new();
    s.input_buffer = "abc".to_string();
    s.input_cursor = 1; // cursor after 'a'
    s.input_delete();
    assert_eq!(s.input_buffer, "ac");
    assert_eq!(s.input_cursor, 1);
}

#[test]
fn input_delete_at_end_no_op() {
    let mut s = TuiState::new();
    s.input_buffer = "x".to_string();
    s.input_cursor = 1;
    s.input_delete();
    assert_eq!(s.input_buffer, "x");
}

#[test]
fn input_clear_line() {
    let mut s = TuiState::new();
    s.input_buffer = "hello".to_string();
    s.input_cursor = 3;
    s.input_clear_line();
    assert!(s.input_buffer.is_empty());
    assert_eq!(s.input_cursor, 0);
}

#[test]
fn input_kill_to_end() {
    let mut s = TuiState::new();
    s.input_buffer = "hello world".to_string();
    s.input_cursor = 5;
    s.input_kill_to_end();
    assert_eq!(s.input_buffer, "hello");
    assert_eq!(s.input_cursor, 5);
}

#[test]
fn input_cursor_multibyte() {
    let mut s = TuiState::new();
    s.input_insert('你');
    s.input_insert('好');
    // cursor at end of "你好"
    s.input_cursor_left();
    // should be between 你 and 好
    assert_eq!(s.input_cursor, "你".len());
    s.input_cursor_left();
    assert_eq!(s.input_cursor, 0);
    s.input_cursor_right();
    assert_eq!(s.input_cursor, "你".len());
}

#[test]
fn input_delete_multibyte() {
    let mut s = TuiState::new();
    s.input_buffer = "你好".to_string();
    s.input_cursor = 0;
    s.input_delete();
    assert_eq!(s.input_buffer, "好");
}

#[test]
fn push_separator_adds_item() {
    let mut s = TuiState::new();
    s.push_separator("test".to_string());
    assert_eq!(s.messages.len(), 1);
    assert!(matches!(&s.messages[0], ChatItem::Separator(l) if l == "test"));
}

#[test]
fn auto_scroll_off_preserves_scroll() {
    let mut s = TuiState::new();
    s.auto_scroll = false;
    s.scroll = 10;
    s.push_user("hi".to_string(), None);
    assert_eq!(s.scroll, 10); // should NOT reset when auto_scroll is off
}

#[test]
fn cache_dirty_on_push() {
    let mut s = TuiState::new();
    s.cache_dirty = false;
    s.push_ai("test".to_string(), None);
    assert!(s.cache_dirty);
}

#[test]
fn needs_redraw_on_input() {
    let mut s = TuiState::new();
    s.needs_redraw = false;
    s.input_insert('x');
    assert!(s.needs_redraw);
}

#[test]
fn trace_lines_capped() {
    let mut s = TuiState::new();
    for i in 0..2500 {
        s.push_trace_line(format!("line {}", i));
    }
    assert!(s.trace_lines.len() <= 2000);
}
```

---

## G06 · Markdown tests — edge cases (T17/T18 gaps)

**File:** `src/messages/markdown.rs` — add to existing `mod tests`:

```rust
#[test]
fn parse_blocks_horizontal_rule() {
    let blocks = parse_blocks("above\n---\nbelow");
    assert!(blocks.iter().any(|b| matches!(b, Block::HorizontalRule)));
}

#[test]
fn parse_blocks_list_items() {
    let blocks = parse_blocks("- one\n- two\n- three");
    let list_count = blocks.iter().filter(|b| matches!(b, Block::ListItem(_))).count();
    assert_eq!(list_count, 3);
}

#[test]
fn parse_blocks_empty_code_block() {
    let blocks = parse_blocks("```\n```");
    assert!(matches!(&blocks[0], Block::CodeBlock { code, .. } if code.is_empty()));
}

#[test]
fn parse_blocks_unclosed_code_block() {
    let blocks = parse_blocks("```rust\nfn main() {}");
    assert!(matches!(&blocks[0], Block::CodeBlock { .. }));
}

#[test]
fn inline_markdown_no_markers() {
    let palette = LocusPalette::locus_dark();
    let spans = parse_inline_markdown("plain text here", &palette);
    assert_eq!(spans.len(), 1);
}

#[test]
fn inline_markdown_unclosed_backtick() {
    let palette = LocusPalette::locus_dark();
    let spans = parse_inline_markdown("use `Option", &palette);
    // Should not panic, should render something
    assert!(!spans.is_empty());
}

#[test]
fn inline_markdown_unclosed_bold() {
    let palette = LocusPalette::locus_dark();
    let spans = parse_inline_markdown("this is **bold", &palette);
    assert!(!spans.is_empty());
}

#[test]
fn has_block_markdown_false_for_plain() {
    assert!(!has_block_markdown("just plain text"));
}

#[test]
fn has_block_markdown_true_for_header() {
    assert!(has_block_markdown("# Title"));
}

#[test]
fn highlight_code_line_rust_keyword() {
    let palette = LocusPalette::locus_dark();
    let spans = highlight_code_line("fn main() {}", "rust", &palette);
    assert!(spans.len() > 1); // should split into keyword + rest
}

#[test]
fn highlight_code_line_string() {
    let palette = LocusPalette::locus_dark();
    let spans = highlight_code_line("let s = \"hello\";", "rust", &palette);
    assert!(spans.len() > 1);
}

#[test]
fn highlight_code_line_comment() {
    let palette = LocusPalette::locus_dark();
    let spans = highlight_code_line("// comment", "rust", &palette);
    assert!(!spans.is_empty());
}

#[test]
fn highlight_code_line_unknown_lang() {
    let palette = LocusPalette::locus_dark();
    let spans = highlight_code_line("some code", "brainfuck", &palette);
    assert!(!spans.is_empty()); // should still render without panic
}
```

Note: `highlight_code_line` is currently private. Either:
- Add `#[cfg(test)] pub(crate)` visibility, or
- Call it indirectly via `render_blocks_to_lines` with a `CodeBlock`, or
- Move these tests inside the `markdown.rs` file where they have access.

Since the tests are inside `markdown.rs`'s own `mod tests`, they already have access to private functions.

---

## Summary

| Gap | What | Files | Est. effort |
|-----|------|-------|-------------|
| G01 | Clippy fix | `run.rs` | 1 min |
| G02 | Header redesign (T06) | `head.rs`, `style.rs`, `mod.rs`, `view.rs` | 15 min |
| G03 | Message tests (T25) | `error.rs`, `ai_message.rs`, `ai_think_message.rs`, `user.rs`, `tool.rs`, `meta_tool.rs` | 20 min |
| G04 | Layout tests (T26) | `split.rs`, `shortcut.rs`, `chats.rs` | 10 min |
| G05 | State tests (T27) | `state.rs` | 15 min |
| G06 | Markdown tests (T17/T18) | `markdown.rs` | 10 min |

**Total: ~70 min of work. After all gaps filled: 0 clippy warnings, ~70+ tests, all 27 tasks complete.**
