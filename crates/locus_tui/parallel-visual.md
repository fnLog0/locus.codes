# Parallel Tool Calls — Visual Plan

When the LLM fires multiple tool calls in a single turn (e.g. 3 `grep` + 1 `read` at once), they arrive as rapid-fire `ToolStart` events followed by `ToolDone` events in arbitrary order. The TUI currently renders them as separate sequential items with no visual grouping or parallel awareness.

**Goal:** Group concurrent tool calls into a single visual block with a header, show them updating in-place, and indicate they're running in parallel.

---

## How LLM parallel tool calls work (event flow)

```
TurnStart(Assistant)
ThinkingDelta...
TextDelta... "I'll check these files"
ToolStart { id: "t1", name: "grep", args: {pattern: "foo", path: "src/"} }
ToolStart { id: "t2", name: "grep", args: {pattern: "bar", path: "src/"} }
ToolStart { id: "t3", name: "read", args: {path: "README.md"} }
ToolDone  { tool_use_id: "t3", result: ... }     ← t3 finishes first
ToolDone  { tool_use_id: "t1", result: ... }     ← t1 finishes second
ToolDone  { tool_use_id: "t2", result: ... }     ← t2 finishes last
TextDelta... "Based on the results..."
TurnEnd
```

Key facts:
- Multiple `ToolStart` events arrive **back-to-back** with no `TextDelta`/`ThinkingDelta`/`TurnEnd` between them.
- `ToolDone` events arrive in **arbitrary order** (whatever finishes first).
- `ToolDone` references the tool by `tool_use_id` (matches `ToolUse.id`).
- After all tools complete, more `TextDelta`/`ToolStart` events may follow.

---

## Current rendering (broken)

```
│ ▸ I'll check these files

  ▶ grep  src/  123ms
                              ← separate items, no grouping
  ▶ grep  src/  120ms

  ▶ read  README.md  45ms

│ Based on the results...
```

Problems:
1. No visual indication that tools ran in parallel.
2. Each tool is a separate `ChatItem` with a spacer between them — looks sequential.
3. `ToolDone` uses `state.messages.last_mut()` — only updates the **last** tool, not the one matching `tool_use_id`. If t3 finishes first but t3 is the 3rd item, `last_mut()` points to t3 ✓. But if t1 finishes next, `last_mut()` still points to t3 (already Done) — t1 never gets updated.

---

## Target rendering

### While running (2 of 3 still going):
```
│ ▸ I'll check these files

  ⫘ 3 tools running
    ▶ grep  src/             120ms
    ▶ grep  src/             118ms
    ✓ read  README.md        45ms
```

### All done:
```
│ ▸ I'll check these files

  ⫘ 3 tools  450ms
    ✓ grep  src/             200ms
    ✓ grep  src/             180ms
    ✓ read  README.md        45ms

│ Based on the results...
```

### Single tool (not parallel — no group header):
```
  ✓ bash  ls  12ms
```

Design rules:
- **Group header** only when 2+ tools arrive in a batch (back-to-back `ToolStart` with no other event types between them).
- Group header shows `⫘ N tools running` while any tool is `Running`, or `⫘ N tools  <total>ms` when all done.
- Individual tools inside the group are **indented** (4 spaces instead of 2).
- Single tool calls render exactly as today (no group header).

---

## Implementation

### Step 1 — Add `tool_use_id` to `ToolCallMessage`

**File:** `src/messages/tool.rs`

Add `id` field:
```rust
pub struct ToolCallMessage {
    pub id: Option<String>,          // ← NEW: from ToolUse.id
    pub tool_name: String,
    pub status: ToolCallStatus,
    pub summary: Option<String>,
    pub started_at_ms: Option<u64>,
}
```

Update constructors:
```rust
impl ToolCallMessage {
    pub fn running(id: impl Into<String>, tool_name: impl Into<String>, summary: Option<String>) -> Self {
        let started_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as u64);
        Self {
            id: Some(id.into()),
            tool_name: tool_name.into(),
            status: ToolCallStatus::Running,
            summary,
            started_at_ms,
        }
    }

    pub fn done(id: Option<String>, tool_name: impl Into<String>, duration_ms: u64, success: bool, summary: Option<String>) -> Self {
        Self {
            id,
            tool_name: tool_name.into(),
            status: ToolCallStatus::Done { duration_ms, success },
            summary,
            started_at_ms: None,
        }
    }

    pub fn error(id: Option<String>, tool_name: impl Into<String>, message: impl Into<String>, summary: Option<String>) -> Self {
        Self {
            id,
            tool_name: tool_name.into(),
            status: ToolCallStatus::Error { message: message.into() },
            summary,
            started_at_ms: None,
        }
    }
}
```

### Step 2 — Add `ChatItem::ToolGroup` variant

**File:** `src/state.rs`

```rust
pub enum ChatItem {
    User(UserMessage),
    Ai(AiMessage),
    Think(AiThinkMessage),
    Tool(ToolCallMessage),
    ToolGroup(Vec<ToolCallMessage>),   // ← NEW
    MetaTool(MetaToolMessage),
    Error(ErrorMessage),
    Separator(String),
}
```

Add push helper:
```rust
impl TuiState {
    /// Push a tool call, grouping with the last ToolGroup if one is active.
    pub fn push_tool_grouped(&mut self, msg: ToolCallMessage) {
        match self.messages.last_mut() {
            Some(ChatItem::ToolGroup(group)) => {
                group.push(msg);
            }
            Some(ChatItem::Tool(existing)) => {
                // Second consecutive tool — upgrade to ToolGroup
                let first = std::mem::replace(existing, ToolCallMessage::running("", "", None));
                let idx = self.messages.len() - 1;
                self.messages[idx] = ChatItem::ToolGroup(vec![first, msg]);
            }
            _ => {
                // First tool in a potential group — push as single Tool
                self.messages.push(ChatItem::Tool(msg));
            }
        }
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Find a tool by id and update it (for ToolDone matching by tool_use_id).
    pub fn update_tool_by_id(&mut self, tool_use_id: &str, duration_ms: u64, success: bool) -> bool {
        for item in self.messages.iter_mut().rev() {
            match item {
                ChatItem::Tool(t) if t.id.as_deref() == Some(tool_use_id) => {
                    *t = ToolCallMessage::done(
                        t.id.clone(),
                        t.tool_name.clone(),
                        duration_ms,
                        success,
                        t.summary.clone(),
                    );
                    self.cache_dirty = true;
                    self.needs_redraw = true;
                    return true;
                }
                ChatItem::ToolGroup(group) => {
                    for t in group.iter_mut() {
                        if t.id.as_deref() == Some(tool_use_id) {
                            *t = ToolCallMessage::done(
                                t.id.clone(),
                                t.tool_name.clone(),
                                duration_ms,
                                success,
                                t.summary.clone(),
                            );
                            self.cache_dirty = true;
                            self.needs_redraw = true;
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}
```

### Step 3 — Update `runtime_events.rs` to use grouping + id-based matching

**File:** `src/runtime_events.rs`

`ToolStart`:
```rust
SessionEvent::ToolStart { tool_use } => {
    // Flush accumulated thinking/AI text before tools
    let think = std::mem::take(&mut state.current_think_text);
    if !think.is_empty() {
        state.push_think(think, false);
    }
    let ai = std::mem::take(&mut state.current_ai_text);
    if !ai.is_empty() {
        let ts = chrono::Local::now().format("%H:%M").to_string();
        state.push_ai(ai, Some(ts));
    }

    if let Some(kind) = MetaToolKind::from_name(&tool_use.name) {
        let detail = tool_detail(&tool_use);
        state.push_meta_tool(MetaToolMessage::running(kind, detail));
    } else {
        let summary = tool_summary(&tool_use);
        state.push_tool_grouped(ToolCallMessage::running(
            tool_use.id,
            tool_use.name,
            summary,
        ));
    }
}
```

`ToolDone` — replace `last_mut()` logic with id-based lookup:
```rust
SessionEvent::ToolDone { tool_use_id, result } => {
    // Try to find and update the tool by its id
    if !state.update_tool_by_id(&tool_use_id, result.duration_ms, !result.is_error) {
        // Fallback: update last MetaTool if applicable
        if let Some(ChatItem::MetaTool(m)) = state.messages.last_mut() {
            *m = MetaToolMessage::done(
                m.kind,
                result.duration_ms,
                !result.is_error,
                m.detail.clone(),
            );
            state.cache_dirty = true;
        }
    }
}
```

### Step 4 — Render `ToolGroup` in `view.rs`

**File:** `src/view.rs`

Add a match arm for `ChatItem::ToolGroup`:
```rust
ChatItem::ToolGroup(tools) => {
    let total = tools.len();
    let running_count = tools.iter().filter(|t| matches!(t.status, ToolCallStatus::Running)).count();
    let all_done = running_count == 0;

    // Group header line
    let header_text = if all_done {
        let total_ms: u64 = tools.iter().map(|t| match &t.status {
            ToolCallStatus::Done { duration_ms, .. } => *duration_ms,
            _ => 0,
        }).max().unwrap_or(0);
        format!("⫘ {} tools  {}", total, crate::utils::format_duration(std::time::Duration::from_millis(total_ms)))
    } else {
        format!("⫘ {} tools running", total)
    };

    let header_style = if all_done {
        crate::layouts::text_muted_style(palette.text_muted)
    } else {
        crate::layouts::text_style(palette.accent)
    };

    lines.push(Line::from(vec![
        ratatui::text::Span::raw(LEFT_PADDING),
        ratatui::text::Span::styled(header_text, header_style),
    ]));

    // Individual tools (indented, no spacer between them)
    for t in tools {
        let elapsed = t.started_at_ms
            .and_then(|s| now_ms.map(|n| n.saturating_sub(s)));
        let name_spans = if matches!(t.status, ToolCallStatus::Running) {
            state.tool_shimmer.as_ref().map(|sh| {
                sh.styled_spans_with_palette(&t.tool_name, palette)
            })
        } else {
            None
        };
        lines.extend(tool::tool_call_lines(t, palette, elapsed, name_spans, true));
    }
    i += 1;
}
```

### Step 5 — Render group header for `ToolGroup`

**File:** `src/messages/tool.rs`

The existing `tool_call_lines` with `in_group: true` already uses `TOOL_GROUP_INDENT` (4 spaces). No changes needed — the indent logic is already there.

### Step 6 — Update tests

**Files:** `src/state.rs`, `src/messages/tool.rs`

Add tests for:
```rust
#[test]
fn push_tool_grouped_single_stays_tool() {
    let mut s = TuiState::new();
    s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
    assert!(matches!(&s.messages[0], ChatItem::Tool(_)));
}

#[test]
fn push_tool_grouped_second_upgrades_to_group() {
    let mut s = TuiState::new();
    s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
    s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
    assert!(matches!(&s.messages[0], ChatItem::ToolGroup(g) if g.len() == 2));
}

#[test]
fn push_tool_grouped_third_appends_to_group() {
    let mut s = TuiState::new();
    s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
    s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
    s.push_tool_grouped(ToolCallMessage::running("t3", "read", None));
    assert!(matches!(&s.messages[0], ChatItem::ToolGroup(g) if g.len() == 3));
}

#[test]
fn update_tool_by_id_in_group() {
    let mut s = TuiState::new();
    s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
    s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
    let updated = s.update_tool_by_id("t1", 200, true);
    assert!(updated);
    if let ChatItem::ToolGroup(g) = &s.messages[0] {
        assert!(matches!(&g[0].status, ToolCallStatus::Done { success: true, .. }));
        assert!(matches!(&g[1].status, ToolCallStatus::Running));
    } else {
        panic!("expected ToolGroup");
    }
}

#[test]
fn update_tool_by_id_single_tool() {
    let mut s = TuiState::new();
    s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
    let updated = s.update_tool_by_id("t1", 100, true);
    assert!(updated);
    assert!(matches!(&s.messages[0], ChatItem::Tool(t) if matches!(t.status, ToolCallStatus::Done { .. })));
}

#[test]
fn non_consecutive_tools_are_separate() {
    let mut s = TuiState::new();
    s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
    s.push_ai("between".to_string(), None); // breaks the consecutive run
    s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
    assert_eq!(s.messages.len(), 3);
    assert!(matches!(&s.messages[0], ChatItem::Tool(_)));
    assert!(matches!(&s.messages[2], ChatItem::Tool(_)));
}
```

---

## Visual reference

### Single tool (no change from today):
```
  ✓ bash  ls  12ms
```

### 2 parallel tools:
```
  ⫘ 2 tools  200ms
    ✓ grep  src/foo.rs       150ms
    ✓ read  README.md         45ms
```

### 5 parallel tools (2 still running):
```
  ⫘ 5 tools running
    ✓ grep  src/             200ms
    ✓ grep  tests/           180ms
    ✓ read  README.md         45ms
    ▶ bash  cargo test       2s 300ms
    ▶ bash  cargo clippy     1s 800ms
```

### Mixed flow in a full turn:
```
│ ▸ Let me check your codebase.              14:32

  ⫘ 3 tools  250ms
    ✓ grep  src/main.rs      120ms
    ✓ grep  src/lib.rs       100ms
    ✓ read  Cargo.toml        30ms

│ ▸ I found the issue. Let me fix it.        14:32

  ✓ edit_file  src/main.rs   45ms

│ ▸ Done. The fix is applied.                14:32

── Turn complete ──
```

---

## Files to change (summary)

| File | Change |
|------|--------|
| `src/messages/tool.rs` | Add `id: Option<String>` field, update constructors |
| `src/state.rs` | Add `ChatItem::ToolGroup`, `push_tool_grouped()`, `update_tool_by_id()` |
| `src/runtime_events.rs` | Use `push_tool_grouped()`, replace `last_mut()` with `update_tool_by_id()` |
| `src/view.rs` | Add `ChatItem::ToolGroup` match arm with header + indented tools |
| `src/lib.rs` | No change (ChatItem re-export covers new variant) |

**Deps:** None. No new crates.

**Verify:** `cargo check -p locus-tui && cargo test -p locus-tui && cargo clippy -p locus-tui`

---

## Edge cases to handle

1. **MetaTool in parallel** — `MetaToolMessage` doesn't participate in tool groups. It stays as `ChatItem::MetaTool` (separate). Only `ToolCallMessage` groups.
2. **ToolDone for unknown id** — `update_tool_by_id` returns `false`. Fallback to MetaTool `last_mut()` check. If neither matches, silently ignore (tool already cleaned up or unknown).
3. **Single tool** — stays as `ChatItem::Tool`, never grouped. `push_tool_grouped` only upgrades when a second consecutive tool arrives.
4. **AI text between tools** — breaks the consecutive run. Next tool starts a new potential group.
5. **ToolGroup with all errors** — header shows `⫘ N tools` with no total time. Individual tools show `✗` with error on second line.
