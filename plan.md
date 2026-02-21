# locus.codes — Plan

High-level plan and priorities for the locus.codes app and repo. See [AGENTS.md](AGENTS.md) for commands and structure.

---

## Current state (Phase 0 kernel)

- **locus_toolbus** — Implemented: ToolBus API, tools (bash, create_file, edit_file, undo_edit, glob, grep, finder), edit history (WAL under `.locus/history/`).
- **locus_cli** — Entry point: `locus run`, `locus tui`, `locus config`, etc. Dotenv loaded; observability init per command.
- **locus_tui** — Interactive TUI: header, scrollable chat, input bar, shortcut line. Message types: user, AI, think, tool, meta-tool. Streaming display (current_ai_text / current_think_text with cursor). Spacing between chat items. No console logs when running TUI.
- **locus_runtime** — Orchestrator: session, LLM (Anthropic / OpenAI / Ollama / Zai), ToolBus, LocusGraph. Provider/model from env or CLI; ZAI default when `ZAI_API_KEY` set; Zai model default `glm-5` (or `ZAI_MODEL`).
- **locus_core** — Session events, sandbox policy, shared types.
- **locus_llms** — Multi-provider LLM client (Anthropic, Zai, etc.).
- **locus_graph** — LocusGraph SDK (MCP store_event, retrieve_memories, etc.).
- **Landing** — `apps/landing/` (React, Vite). Not in scope for this plan.

---

## Task list — TUI features, UX, visual polish

Each task is self-contained. An agent can pick any task by ID and implement it. Files are relative to `crates/locus_tui/src/`.

### P0 — Core UX (must-have, do first)

#### T01 · Auto-scroll on new content
> **Files:** `state.rs`, `run.rs`, `runtime_events.rs`
>
> **Current:** `scroll` offset is manual only. New messages arrive off-screen if user scrolled up.
>
> **Task:**
> 1. Add `auto_scroll: bool` field to `TuiState` (default `true`).
> 2. When `auto_scroll` is true, reset `scroll = 0` after every `push_*` call and after `flush_turn()`.
> 3. Set `auto_scroll = false` when user presses Up/PageUp (manual scroll).
> 4. Set `auto_scroll = true` when user scrolls to bottom (scroll == 0) or presses End.
> 5. In `run_loop`, after draining session events, if `auto_scroll` is true, set `scroll = 0`.

#### T02 · Inline error messages in chat
> **Files:** `state.rs`, `runtime_events.rs`, `messages/mod.rs` (new `error.rs`)
>
> **Current:** `SessionEvent::Error` only sets `state.status` (header bar). Errors disappear when next status arrives.
>
> **Task:**
> 1. Create `messages/error.rs` with `ErrorMessage { text: String, timestamp: Option<String> }`.
> 2. Add `error_message_lines()` — render with `✗` icon in `palette.danger`, text in `palette.danger`, wrap like AI message.
> 3. Add `ChatItem::Error(ErrorMessage)` variant to `state.rs`.
> 4. Add `push_error()` method to `TuiState`.
> 5. In `runtime_events.rs`, `SessionEvent::Error` → call `state.push_error(error)` in addition to setting `state.status`.
> 6. In `view.rs`, handle `ChatItem::Error` in the match arm.

#### T03 · Keyboard: Home/End, Ctrl+U/K, Delete, cursor Left/Right
> **Files:** `run.rs`, `state.rs`
>
> **Current:** Only Backspace and Char input. No cursor movement or line editing.
>
> **Task:**
> 1. `KeyCode::Left` → `state.input_cursor_left()` (move cursor back one char, UTF-8 safe).
> 2. `KeyCode::Right` → `state.input_cursor_right()`.
> 3. `KeyCode::Home` → `state.input_cursor = 0`.
> 4. `KeyCode::End` → `state.input_cursor = state.input_buffer.len()`.
> 5. `KeyCode::Delete` → delete char at cursor (forward delete).
> 6. `Ctrl+U` → clear input buffer entirely.
> 7. `Ctrl+K` → delete from cursor to end of line.
> 8. `KeyCode::End` in empty input → set `auto_scroll = true`, `scroll = 0` (scroll to bottom).

#### T04 · Input cursor rendering fix
> **Files:** `view.rs`
>
> **Current:** Cursor position uses char count but the buffer may have multi-byte UTF-8. Cursor can land mid-glyph.
>
> **Task:**
> 1. Use `unicode_width::UnicodeWidthStr` (add `unicode-width` crate) to compute display width for cursor positioning.
> 2. Or at minimum, ensure cursor column accounts for actual display width of input text before cursor.

---

### P1 — Visual polish (look & feel, make it sexy)

#### T05 · Refined dark palette — deeper blacks, softer accents
> **Files:** `theme/palette.rs`
>
> **Current:** Tokyo Night inspired. Background `(10,10,15)`, accent `(122,162,247)`. Functional but flat.
>
> **Task:** Tune the `locus_dark()` palette for more depth and contrast:
> 1. Background → `Rgb(8, 8, 12)` — true-black feel.
> 2. Surface → `Rgb(16, 17, 24)` — barely visible card separation.
> 3. Elevated surface → `Rgb(22, 23, 32)`.
> 4. Border → `Rgb(28, 30, 42)` — subtle, not distracting.
> 5. Accent → `Rgb(99, 148, 255)` — slightly more saturated blue.
> 6. Success → `Rgb(120, 220, 120)` — brighter green, reads better on dark.
> 7. Text → `Rgb(200, 210, 245)` — warmer white, less eye strain.
> 8. Text muted → `Rgb(70, 78, 110)` — clearly secondary but still readable.
> 9. Danger → `Rgb(255, 100, 120)` — punchier red.
> 10. Warning → `Rgb(240, 185, 100)`.
> 11. Info → `Rgb(100, 200, 255)`.
> 12. Add `pub scrollbar_thumb_active: Rgb` for active scrollbar state.

#### T06 · Header bar — gradient feel + status badge
> **Files:** `layouts/head.rs`, `view.rs`
>
> **Current:** Plain single-line header with left title and right status. No visual weight.
>
> **Task:**
> 1. Increase `HEADER_HEIGHT` to 2 (give breathing room).
> 2. First line: title in bold (`Modifier::BOLD`), separator dot `·`, model/provider in `text_muted`.
> 3. Second line: thin border (use `palette.border` with `Borders::BOTTOM`).
> 4. Status text: render with a colored dot prefix — `●` green when "Ready", `●` yellow when streaming, `●` red on error.
> 5. Update `render_header` and `HEADER_HEIGHT` constant.
> 6. Update `main_splits` to account for new header height.

#### T07 · Message bubbles — visual separation between speakers
> **Files:** `messages/user.rs`, `messages/ai_message.rs`, `view.rs`
>
> **Current:** All messages share the same background. Only the indicator icon differs.
>
> **Task:**
> 1. User messages: render with a subtle left-border accent (2-char wide `│` in `palette.accent` before each line) instead of just `»`.
> 2. AI messages: render with left-border in `palette.text_muted` (dimmer than user).
> 3. Add 1 blank line above each message block in `view.rs` (already done, but ensure consistent spacing with the new borders).
> 4. Optional: user messages right-aligned within chat width (like iMessage). If implemented, cap message width to 80% of viewport.

#### T08 · Tool call rendering — compact expandable cards
> **Files:** `messages/tool.rs`, `messages/meta_tool.rs`
>
> **Current:** Single-line per tool call. Running tools show `▶`, done shows `✓`/`✗`.
>
> **Task:**
> 1. Running state: add shimmer animation (use existing `Shimmer`) to the tool name while running.
> 2. Running state: show elapsed time live (add `started_at: Option<Instant>` to `ToolCallMessage`).
> 3. Done state: show tool name dimmed, duration in muted, summary in normal text. Compact single line.
> 4. Group consecutive tool calls visually: if 3+ tools in a row, show a "Tools ▸" header line with count, then indented tool lines.
> 5. Error state: show error text on a second line, indented, in `palette.danger`.

#### T09 · Thinking blocks — collapsible, distinct style
> **Files:** `messages/ai_think_message.rs`, `state.rs`, `view.rs`
>
> **Current:** Thinking text shown inline in `text_muted`. Can be long and noisy.
>
> **Task:**
> 1. Add `collapsed: bool` to `AiThinkMessage` (default `false`).
> 2. When collapsed, show single line: `⋯ Thinking (N lines)` in `text_muted`.
> 3. When expanded, show full text as today.
> 4. Add keybinding `t` (when input is empty) to toggle last thinking block collapse.
> 5. Thinking indicator: use `palette.info` instead of `text_muted` for the `⋯` icon to make it pop slightly.
> 6. Streaming thinking: show last 3 lines only + "…" above (auto-truncate during stream, expand on turn end).

#### T10 · Smooth streaming — character-level cursor animation
> **Files:** `animation/shimmer.rs`, `view.rs`, `run.rs`
>
> **Current:** Streaming cursor `▌` is static. No visual motion beyond text appearing.
>
> **Task:**
> 1. Make streaming cursor blink: alternate `▌` / ` ` every 500ms using a frame counter in `run_loop`.
> 2. Add `frame_count: u64` to `TuiState`, increment every loop iteration.
> 3. Pass `frame_count` to `ai_message_lines` and `think_message_lines` to determine cursor visibility.
> 4. Optional: fade the cursor color between `palette.accent` and `palette.text_muted` using lerp (reuse shimmer's lerp).

#### T11 · Input bar — focus glow + placeholder text
> **Files:** `layouts/input.rs`, `view.rs`
>
> **Current:** Static bordered box with `▸` icon. No visual feedback for focus state.
>
> **Task:**
> 1. When input is focused (always, for now), use `palette.border_focused` for the border instead of `palette.border`.
> 2. When input buffer is empty, show placeholder text: `"Ask anything…"` in `palette.text_placeholder`.
> 3. Input icon `▸` should use `palette.accent` when buffer is empty, `palette.success` when typing.
> 4. Add subtle `ROUNDED` border style (ratatui `BorderType::Rounded`) for the input block.

#### T12 · Scrollbar indicator
> **Files:** `view.rs`, `layouts/chats.rs`
>
> **Current:** No scrollbar. User has no visual indicator of position in chat.
>
> **Task:**
> 1. Calculate scrollbar thumb position and size from `scroll`, `content_height`, `viewport_height`.
> 2. Render a thin scrollbar (1 char wide) on the right edge of the chat body using `palette.scrollbar_thumb_background` (track) and `palette.scrollbar_thumb_hover_background` (thumb).
> 3. Only show scrollbar when content exceeds viewport (hide when everything fits).
> 4. Use block characters `▐` or `█` for thumb, `░` or space for track.

#### T13 · Shortcut bar — context-aware hints
> **Files:** `layouts/shortcut.rs`, `view.rs`, `state.rs`
>
> **Current:** Static hint string: `"shortcuts  ↑↓ PgUp/PgDn: scroll  q: quit"`.
>
> **Task:**
> 1. Make shortcuts dynamic based on state:
>    - When streaming: show `"Streaming…  Ctrl+C: cancel"`
>    - When input has text: show `"Enter: send  Ctrl+U: clear  Ctrl+C: quit"`
>    - When input empty: show `"↑↓: scroll  t: toggle thinking  q: quit  Ctrl+C: quit"`
> 2. Add `is_streaming: bool` to `TuiState` — set true on `TurnStart(Assistant)`, false on `TurnEnd`.
> 3. Render each shortcut key in `palette.text_muted`, description in `palette.text_disabled` for visual hierarchy.
> 4. Separate shortcuts with `·` (middle dot) instead of double-space.

---

### P2 — Performance & optimization

#### T14 · Line cache — avoid rebuilding every frame
> **Files:** `view.rs`, `state.rs`
>
> **Current:** Every `draw()` call rebuilds `all_lines` from scratch by iterating all messages. O(n) per frame.
>
> **Task:**
> 1. Add `cached_lines: Vec<Line<'static>>` and `cache_dirty: bool` to `TuiState`.
> 2. Set `cache_dirty = true` in all `push_*` methods and `flush_turn()`.
> 3. In `view.rs`, only rebuild `all_lines` from `state.messages` when `cache_dirty` is true; otherwise reuse `cached_lines`.
> 4. Always append streaming content (current_ai_text, current_think_text) fresh on top of cached lines.
> 5. Reset cache on terminal resize.

#### T15 · Reduce draw frequency — skip identical frames
> **Files:** `run.rs`
>
> **Current:** `terminal.draw()` called every 100ms regardless of changes.
>
> **Task:**
> 1. Add `needs_redraw: bool` to `TuiState` (default `true`).
> 2. Set `needs_redraw = true` on any state mutation (input, scroll, session event, resize).
> 3. In `run_loop`, only call `terminal.draw()` when `needs_redraw` is true. Reset after draw.
> 4. Always redraw when streaming (current_ai_text or current_think_text is non-empty) — animation needs continuous frames.
> 5. Handle `Event::Resize` to force redraw.

#### T16 · Smart scroll clamp
> **Files:** `state.rs`, `view.rs`
>
> **Current:** `scroll_up` can scroll past content. `scroll_down` can go negative (saturating_sub handles it, but logically wrong).
>
> **Task:**
> 1. After building `all_lines` in `view.rs`, clamp `state.scroll` to `0..=max_scroll` where `max_scroll = content_height.saturating_sub(viewport_height)`.
> 2. `scroll_up` and `scroll_down` should clamp immediately in `state.rs` — but since max depends on content + viewport, store `last_content_height` and `last_viewport_height` on state after each draw.

---

### P3 — Rich content rendering (medium-term)

#### T17 · Markdown rendering in AI messages
> **Files:** `messages/ai_message.rs`, add `messages/markdown.rs`
>
> **Current:** AI text is plain. Code, bold, italic, headers are not styled.
>
> **Task:**
> 1. Create `messages/markdown.rs` with a simple inline parser (no external crate needed for basics):
>    - `**bold**` → `Modifier::BOLD`
>    - `` `code` `` → `palette.accent` fg + `palette.element_background` bg
>    - `# Header` → `Modifier::BOLD` + `palette.text`
>    - `- list item` → `palette.text_muted` bullet + `palette.text` content
> 2. Fenced code blocks (` ``` `) → render with `palette.editor_background` bg, `palette.editor_foreground` fg, bordered with `palette.border_variant`.
> 3. Horizontal rule `---` → render as `─` repeated to fill width in `palette.border`.
> 4. Integrate into `ai_message_lines()`: detect markdown, parse, render styled spans.
> 5. Keep plain-text fallback (no markdown detected = current behavior).

#### T18 · Code block syntax highlighting (basic)
> **Files:** `messages/markdown.rs`
>
> **Current:** No code blocks at all.
>
> **Task:**
> 1. Detect language from fenced code block (e.g. ` ```rust `).
> 2. Basic keyword highlighting (use a small hardcoded keyword set for rust/python/js/ts):
>    - Keywords → `palette.accent`
>    - Strings → `palette.success`
>    - Comments → `palette.text_muted`
>    - Numbers → `palette.warning`
> 3. Line numbers on the left in `palette.editor_line_number`.
> 4. Wrap code block in a visual box: top/bottom border, left gutter for line numbers.

#### T19 · Copy support — yank last AI message
> **Files:** `run.rs`, `state.rs`
>
> **Current:** No copy/clipboard support.
>
> **Task:**
> 1. `Ctrl+Y` (when input empty) → copy the last AI message text to system clipboard.
> 2. Use `cli_clipboard` or OSC 52 escape sequence (works in most modern terminals without external deps).
> 3. Show brief flash status: `"Copied to clipboard"` in `state.status` for 2 seconds, then clear.
> 4. Add `status_timeout: Option<Instant>` to `TuiState` — auto-clear status after duration.

---

### P4 — Visual micro-polish (details that make it feel premium)

#### T20 · Typing indicator while AI is generating
> **Files:** `view.rs`, `state.rs`, `animation/shimmer.rs`
>
> **Current:** Only `current_ai_text` with cursor shows generation in progress. When the model is thinking before output, nothing visual happens.
>
> **Task:**
> 1. When `is_streaming` is true and both `current_ai_text` and `current_think_text` are empty, show a typing indicator.
> 2. Typing indicator: `⋯` with shimmer animation applied (reuse `Shimmer`).
> 3. Show below the last message in the chat body.
> 4. Remove indicator as soon as any text delta arrives.

#### T21 · Session separator line
> **Files:** `state.rs`, `view.rs`
>
> **Current:** No visual boundary when a new session/conversation starts.
>
> **Task:**
> 1. Add `ChatItem::Separator(String)` variant (label like "New session" or timestamp).
> 2. Render as a centered thin line: `── New session ──` in `palette.text_disabled`.
> 3. Push a separator on `SessionEvent::SessionEnd`.

#### T22 · Timestamp on AI messages
> **Files:** `messages/ai_message.rs`, `state.rs`, `runtime_events.rs`
>
> **Current:** User messages have optional timestamp. AI messages don't.
>
> **Task:**
> 1. Add `timestamp: Option<String>` to `AiMessage`.
> 2. Set timestamp in `flush_turn()` using current time (`chrono::Local::now().format("%H:%M")`).
> 3. Render after indicator, before text, in `text_muted` (same pattern as user messages).
> 4. Add `chrono` dependency to `Cargo.toml`.

#### T23 · Empty state — welcome screen
> **Files:** `view.rs`
>
> **Current:** Empty chat body on first launch. No guidance for the user.
>
> **Task:**
> 1. When `state.messages` is empty and not streaming, render a centered welcome:
>    ```
>    locus.codes
>    
>    Type a message to begin.
>    ```
> 2. `locus.codes` in `palette.text` with shimmer animation.
> 3. Subtitle in `palette.text_muted`.
> 4. Center vertically and horizontally in the chat body area.

#### T24 · Status bar timeout (auto-clear transient status)
> **Files:** `state.rs`, `run.rs`
>
> **Current:** Status text persists until overwritten by another event.
>
> **Task:**
> 1. Add `status_set_at: Option<Instant>` to `TuiState`.
> 2. In `run_loop`, if status is set and `Instant::now() - status_set_at > 5s`, clear status to empty.
> 3. `SessionEvent::Status` and `SessionEvent::Error` set `status_set_at`.
> 4. Permanent statuses (like "Session ended") should set `status_set_at = None` (never auto-clear).

---

### P5 — Tests

#### T25 · Message rendering tests
> **Files:** `messages/*.rs`
>
> **Current:** Basic tests exist (indicator present, wrapping works). Need coverage for edge cases.
>
> **Task:**
> 1. Test empty text, whitespace-only text, very long single word, Unicode (emoji, CJK).
> 2. Test streaming cursor appears when `streaming = true`, absent when `false`.
> 3. Test error message rendering (T02 — after implemented).
> 4. Test tool call all states: Running, Done(success), Done(fail), Error.
> 5. Test meta-tool all states and all kinds.

#### T26 · Layout tests
> **Files:** `layouts/*.rs`
>
> **Current:** Some tests in `chats.rs`, `panel.rs`, `split.rs`. Missing coverage.
>
> **Task:**
> 1. Test `main_splits` with very small terminal (height < HEADER_HEIGHT + FOOTER_HEIGHT).
> 2. Test `header_line` truncation when width is smaller than title + status.
> 3. Test `shortcut_inner_rect` with zero-width area.
> 4. Test `block_for_input_bordered` produces correct padding.

#### T27 · State tests
> **Files:** `state.rs`
>
> **Current:** No tests.
>
> **Task:**
> 1. Test `input_insert` with ASCII, multi-byte UTF-8, emoji.
> 2. Test `input_backspace` at position 0, mid-string, end, on multi-byte char.
> 3. Test `input_take` returns content and resets cursor.
> 4. Test `flush_turn` pushes accumulated text and clears buffers.
> 5. Test `scroll_up` / `scroll_down` with large deltas.
> 6. Test `push_*` methods add correct `ChatItem` variant.

---

## Implementation order (recommended)

```
Phase 1 — Core UX:        T01 → T02 → T03 → T04
Phase 2 — Visual:         T05 → T11 → T06 → T07 → T12 → T13
Phase 3 — Smooth:         T10 → T20 → T23 → T08 → T09
Phase 4 — Performance:    T14 → T15 → T16
Phase 5 — Rich content:   T17 → T18 → T19
Phase 6 — Micro-polish:   T21 → T22 → T24
Phase 7 — Tests:          T25 → T26 → T27
```

Each task should be verified with `cargo check -p locus-tui` and `cargo test -p locus-tui` after implementation. Run `cargo clippy -p locus-tui` before marking done.

---

## Out of scope for this plan

- Landing page feature work.
- Deployment, infra, or packaging (e.g. installers).
- Non–locus.codes products or repos.
- locus_runtime / locus_llms / locus_graph implementation (separate plan).

---

## Changelog (plan)

- **2026-02-21** — Expanded plan: 27 tasks (T01–T27) covering core UX, visual polish, performance, rich content, micro-polish, and tests. Replaced short/medium-term sections with detailed task list.
