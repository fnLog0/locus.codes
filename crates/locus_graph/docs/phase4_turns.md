# Phase 4 — Turns + Turn Events

**Goal:** Record every agent action as a LocusGraph event within a turn lifecycle. Turn anchors are created at turn start, child events are buffered during the turn, and a summary overwrites the anchor at turn end. This enables cross-session semantic search over past agent behavior.

This document is the concrete execution checklist for Phase 4. If it conflicts with `implementation_plan.md`, the main implementation plan wins.

**Hierarchy being built:**

```
session:{slug}_{session_id}                          ← Phase 3 (exists)
  └── turn:{session_slug}_{turn_slug}                ← EXISTS (context_id only, no event stored)
        ├── action:{session_id}_{turn_id}_{seq}      ← NEW (tool call events)
        ├── llm:{session_id}_{turn_id}_{seq}         ← NEW (LLM call events)
        ├── intent:{session_id}_{turn_id}_{seq}      ← NEW (user intent)
        ├── decision:{session_id}_{turn_id}_{seq}    ← NEW (agent decisions)
        ├── error:{session_id}_{turn_id}_{seq}       ← NEW (tool/LLM errors)
        └── snapshot:{session_id}_{turn_id}_{seq}    ← NEW (context snapshot)
```

**Depends on:** Phase 3 (turns extend sessions).

---

## What Exists Now

### Current turn handling

In `runtime/mod.rs`:
- `turn_sequence: u32` — increments per turn (1-based)
- `event_seq: u32` — increments per event within a turn (1-based, resets per turn)
- `session_slug: String` — kebab-case from first user message
- `turn_slug: String` — kebab-case from each user message
- `turn_ctx()` — returns `"turn:{session_slug}_{turn_slug}"`
- `turn_id()` — returns zero-padded `"{:03}"` of turn_sequence (e.g., "001")
- `next_seq()` — increments `event_seq` and returns the new value
- `add_turn_context(turn_ctx)` — adds to `context_ids` list

In `agent_loop.rs` `process_message()` (line ~256):
```rust
self.turn_sequence += 1;
self.event_seq = 0;
self.turn_slug = Self::slugify_turn(&message);
let turn_ctx = self.turn_ctx();
self.add_turn_context(turn_ctx.clone());
```

In `tools.rs` `execute_tool_calls()` (line ~56):
```rust
let session_id = self.session.id.as_str().to_string();
let turn_id = self.turn_id();
let seq = self.next_seq();
// These are passed to handle_tool_call but UNUSED (_session_id, _turn_id, _seq)
```

In `tool_handler.rs` `handle_tool_call()`:
```rust
pub async fn handle_tool_call(
    tool: ToolUse,
    toolbus: &Arc<ToolBus>,
    locus_graph: Arc<LocusGraphClient>,
    event_tx: &mpsc::Sender<SessionEvent>,
    _session_id: String,   // ← UNUSED
    _turn_id: String,      // ← UNUSED
    _seq: u32,             // ← UNUSED
) -> Result<ToolResultData, RuntimeError>
```

In `llm.rs` `stream_llm_response()`:
- Tracks `prompt_tokens`, `completion_tokens`, `duration`
- No LocusGraph storage — these metrics are only accumulated on `Session`

**Problems:**
1. `turn_ctx()` is computed and added to `context_ids` but **no turn event is stored in LocusGraph**.
2. `session_id`, `turn_id`, `seq` are computed and passed around but **never used** — they're prefixed with `_`.
3. Tool call results are not stored as events — only emitted as `SessionEvent` for the TUI.
4. LLM call metadata (tokens, duration, model) is not stored as events.
5. No turn summary is stored at turn end.
6. No write buffer — each event would require a separate `store_event` call. Need at minimum a `Vec<CreateEventRequest>` buffer.

### Available infrastructure

| Piece | Status | Location |
|---|---|---|
| `turn_sequence`, `event_seq`, `next_seq()` | ✅ Exists | `runtime/mod.rs` |
| `turn_ctx()`, `turn_id()`, `turn_slug` | ✅ Exists | `runtime/mod.rs` |
| `session_id`, `turn_id`, `seq` passed to handlers | ✅ Exists (unused) | `tools.rs`, `tool_handler.rs` |
| `LocusGraphClient.store_event()` | ✅ Exists | `locus_graph/client.rs` |
| `TurnSummary` type | ✅ Exists | `locus_graph/types.rs` |
| `session_context_id()` | ✅ Exists | `memory/anchors.rs` |
| `chrono` dependency | ✅ Exists | `locus_runtime/Cargo.toml` |
| Write buffer | ❌ Missing | Need to add |
| Turn event storage functions | ❌ Missing | Need to add |

---

## Context ID Formats

| Event type | Context ID format | Example |
|---|---|---|
| Turn anchor | `turn:{session_slug}_{turn_slug}` | `turn:fix-jwt-bug_validate-token` |
| Action (tool call) | `action:{short_session_id}_{turn_id}_{seq}` | `action:a1b2c3d4_001_001` |
| LLM call | `llm:{short_session_id}_{turn_id}_{seq}` | `llm:a1b2c3d4_001_002` |
| Intent | `intent:{short_session_id}_{turn_id}_{seq}` | `intent:a1b2c3d4_001_003` |
| Decision | `decision:{short_session_id}_{turn_id}_{seq}` | `decision:a1b2c3d4_001_004` |
| Error | `error:{short_session_id}_{turn_id}_{seq}` | `error:a1b2c3d4_001_005` |
| Snapshot | `snapshot:{short_session_id}_{turn_id}_{seq}` | `snapshot:a1b2c3d4_001_006` |

`short_session_id` = first 8 chars of `session.id` UUID (same as `session_context_id` uses).

---

## Tasks

### Task 1: Add write buffer to Runtime

**File:** `crates/locus_runtime/src/runtime/mod.rs`

Add a `Vec<CreateEventRequest>` field to buffer turn events:

```rust
use locus_graph::CreateEventRequest;

pub struct Runtime {
    // ... existing fields ...
    /// Write buffer for turn events (flushed at turn end)
    turn_event_buffer: Vec<CreateEventRequest>,
}
```

Initialize as empty in `Runtime::new()`, `new_with_shared()`, and `new_continuing()`:

```rust
turn_event_buffer: Vec::new(),
```

Add helper methods:

```rust
/// Buffer a turn event for later flush.
fn buffer_event(&mut self, event: CreateEventRequest) {
    self.turn_event_buffer.push(event);
}

/// Flush all buffered turn events to LocusGraph.
/// Called at turn end. Events are sent fire-and-forget.
async fn flush_turn_events(&mut self) {
    let events: Vec<CreateEventRequest> = self.turn_event_buffer.drain(..).collect();
    if events.is_empty() {
        return;
    }
    let locus_graph = Arc::clone(&self.locus_graph);
    tokio::spawn(async move {
        for event in events {
            locus_graph.store_event(event).await;
        }
    });
}

/// Get the short session ID (first 8 chars).
fn short_session_id(&self) -> String {
    let id = self.session.id.as_str();
    if id.len() > 8 { id[..8].to_string() } else { id.to_string() }
}

/// Build an event context_id: "{event_type}:{short_session_id}_{turn_id}_{seq}"
fn event_ctx(&self, event_type: &str, seq: u32) -> String {
    format!("{}:{}_{}_{}",
        event_type,
        self.short_session_id(),
        self.turn_id(),
        format!("{:03}", seq)
    )
}
```

### Task 2: Add turn event storage functions

**File:** `crates/locus_runtime/src/memory/turns.rs` (NEW)

```rust
use locus_graph::{CreateEventRequest, EventKind, TurnSummary};

/// Build a turn start event.
///
/// Creates the turn anchor in LocusGraph with minimal payload.
/// At turn end, the same context_id is overwritten with the full summary.
pub fn build_turn_start(
    turn_ctx: &str,
    session_ctx: &str,
    user_message: &str,
    turn_sequence: u32,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Action,
        serde_json::json!({
            "kind": "turn",
            "data": {
                "status": "active",
                "turn_sequence": turn_sequence,
                "user_message": truncate(user_message, 500),
                "started_at": chrono::Utc::now().to_rfc3339(),
            }
        }),
    )
    .context_id(turn_ctx)
    .extends(vec![session_ctx.to_string()])
    .source("executor")
}

/// Build a turn end event (overwrites the turn anchor with summary).
pub fn build_turn_end(
    turn_ctx: &str,
    session_ctx: &str,
    summary: TurnSummary,
    turn_sequence: u32,
    duration_ms: u64,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Action,
        serde_json::json!({
            "kind": "turn",
            "data": {
                "status": "completed",
                "turn_sequence": turn_sequence,
                "title": summary.title,
                "user_request": summary.user_request,
                "actions_taken": summary.actions_taken,
                "outcome": summary.outcome,
                "decisions": summary.decisions,
                "files_read": summary.files_read,
                "files_modified": summary.files_modified,
                "event_count": summary.event_count,
                "duration_ms": duration_ms,
                "ended_at": chrono::Utc::now().to_rfc3339(),
            }
        }),
    )
    .context_id(turn_ctx)
    .extends(vec![session_ctx.to_string()])
    .source("executor")
}

/// Build an action event (tool call).
pub fn build_action_event(
    event_ctx: &str,
    turn_ctx: &str,
    tool_name: &str,
    tool_args: &serde_json::Value,
    result: &serde_json::Value,
    is_error: bool,
    duration_ms: u64,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Action,
        serde_json::json!({
            "kind": "tool_call",
            "data": {
                "tool": tool_name,
                "args_summary": truncate(&tool_args.to_string(), 300),
                "result_summary": truncate(&result.to_string(), 500),
                "is_error": is_error,
                "duration_ms": duration_ms,
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("executor")
}

/// Build an LLM call event.
pub fn build_llm_event(
    event_ctx: &str,
    turn_ctx: &str,
    model: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
    duration_ms: u64,
    has_tool_calls: bool,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Observation,
        serde_json::json!({
            "kind": "llm_call",
            "data": {
                "model": model,
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": prompt_tokens + completion_tokens,
                "duration_ms": duration_ms,
                "has_tool_calls": has_tool_calls,
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("executor")
}

/// Build an intent event (captures the user's request for the turn).
pub fn build_intent_event(
    event_ctx: &str,
    turn_ctx: &str,
    user_message: &str,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "intent",
            "data": {
                "message": truncate(user_message, 1000),
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("agent")
}

/// Build an error event.
pub fn build_error_event(
    event_ctx: &str,
    turn_ctx: &str,
    error_source: &str,
    error_message: &str,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Observation,
        serde_json::json!({
            "kind": "error",
            "data": {
                "source": error_source,
                "message": truncate(error_message, 500),
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("executor")
}

/// Truncate a string to max_len, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
```

### Task 3: Register turns module

**File:** `crates/locus_runtime/src/memory/mod.rs`

Add the new module and re-exports:

```rust
mod turns;

pub use turns::{
    build_turn_start, build_turn_end, build_action_event,
    build_llm_event, build_intent_event, build_error_event,
};
```

### Task 4: Store turn start event in process_message

**File:** `crates/locus_runtime/src/runtime/agent_loop.rs`

After `self.add_turn_context(turn_ctx.clone())` (line ~262), store the turn anchor and intent events:

```rust
// --- Phase 4: Store turn start in LocusGraph ---
let session_ctx = memory::session_context_id(&self.session_slug, self.session.id.as_str());

// Store turn anchor (sent immediately — not buffered)
let turn_start_event = memory::build_turn_start(
    &turn_ctx,
    &session_ctx,
    &message,
    self.turn_sequence,
);
self.locus_graph.store_event(turn_start_event).await;

// Buffer intent event
let intent_seq = self.next_seq();
let intent_event = memory::build_intent_event(
    &self.event_ctx("intent", intent_seq),
    &turn_ctx,
    &message,
);
self.buffer_event(intent_event);
```

**Important:** `turn_start_event` is sent immediately (not buffered) because it creates the anchor that child events extend. Intent is buffered because it's a child event.

### Task 5: Store action events in tool_handler

**File:** `crates/locus_runtime/src/tool_handler.rs`

The `_session_id`, `_turn_id`, and `_seq` parameters are already passed but unused. Remove the `_` prefix and use them to build action events.

Update `handle_tool_call` signature — remove underscores:

```rust
pub async fn handle_tool_call(
    tool: ToolUse,
    toolbus: &Arc<ToolBus>,
    locus_graph: Arc<LocusGraphClient>,
    event_tx: &mpsc::Sender<SessionEvent>,
    session_id: String,
    turn_id: String,
    seq: u32,
) -> Result<(ToolResultData, Option<CreateEventRequest>), RuntimeError>
```

**Return type changes:** Return both the `ToolResultData` and an optional `CreateEventRequest` for the caller to buffer.

After the tool call completes (after computing `tool_result`), build the action event:

```rust
// Build action event for LocusGraph
let short_id = if session_id.len() > 8 { &session_id[..8] } else { &session_id };
let event_ctx = format!("action:{}_{:03}_{:03}", short_id, turn_id, seq);
// turn_ctx needs to be passed in or computed — see alternative approach below
```

**Alternative approach (simpler):** Instead of changing `handle_tool_call`'s return type, have the caller (in `tools.rs`) build the action event using the result. This avoids changing the signature of a function with many callers.

**File:** `crates/locus_runtime/src/runtime/tools.rs`

In `execute_tool_calls()`, after each tool result:

```rust
for tool_use in regular_tools {
    // ... existing confirmation check ...

    let session_id = self.session.id.as_str().to_string();
    let turn_id = self.turn_id();
    let seq = self.next_seq();
    let result = match tool_handler::handle_tool_call(
        tool_use.clone(),
        &self.toolbus,
        Arc::clone(&self.locus_graph),
        &self.event_tx,
        session_id.clone(),
        turn_id.clone(),
        seq,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            // Buffer error event
            let error_event = memory::build_error_event(
                &self.event_ctx("error", seq),
                &self.turn_ctx(),
                &tool_use.name,
                &e.to_string(),
            );
            self.buffer_event(error_event);
            record_error(&e);
            return Err(e);
        }
    };

    // Buffer action event
    let action_event = memory::build_action_event(
        &self.event_ctx("action", seq),
        &self.turn_ctx(),
        &tool_use.name,
        &tool_use.args,
        &result.output,
        result.is_error,
        result.duration_ms,
    );
    self.buffer_event(action_event);

    // If tool returned an error, also buffer an error event
    if result.is_error {
        let err_seq = self.next_seq();
        let error_event = memory::build_error_event(
            &self.event_ctx("error", err_seq),
            &self.turn_ctx(),
            &tool_use.name,
            &result.output.to_string(),
        );
        self.buffer_event(error_event);
    }

    results.push((tool_use, result));
}
```

Same for task tools in the same function.

### Task 6: Store LLM call events

**File:** `crates/locus_runtime/src/runtime/llm.rs`

After `self.session.add_llm_usage(prompt_tokens, completion_tokens)` (line ~155), buffer an LLM event:

```rust
// Buffer LLM call event
let llm_seq = self.next_seq();
let llm_event = memory::build_llm_event(
    &self.event_ctx("llm", llm_seq),
    &self.turn_ctx(),
    &request.model,  // Need to capture model before consuming request
    prompt_tokens,
    completion_tokens,
    duration.as_millis() as u64,
    !tool_uses.is_empty(),  // Move this line — see note below
);
self.buffer_event(llm_event);
```

**Note:** The `model` must be captured before the `request` is consumed by `self.llm_client.stream(request)`. Add at the top of `stream_llm_response()`:

```rust
let model = request.model.clone();
```

And `has_tool_calls` is only known after parsing tool_calls. So the LLM event should be buffered **after** all tool calls are collected but **before** executing them:

```rust
// After building tool_uses vec (line ~185), before execute_tool_calls:
let llm_seq = self.next_seq();
let llm_event = memory::build_llm_event(
    &self.event_ctx("llm", llm_seq),
    &self.turn_ctx(),
    &model,
    prompt_tokens,
    completion_tokens,
    duration.as_millis() as u64,
    !tool_uses.is_empty(),
);
self.buffer_event(llm_event);
```

Also buffer error events on LLM failures. In the error match arms:

```rust
StreamEvent::Error { message } => {
    let err_seq = self.next_seq();
    let error_event = memory::build_error_event(
        &self.event_ctx("error", err_seq),
        &self.turn_ctx(),
        "llm",
        &message,
    );
    self.buffer_event(error_event);
    // ... existing error handling ...
}
```

### Task 7: Build turn summary and flush at turn end

**File:** `crates/locus_runtime/src/runtime/agent_loop.rs`

At the end of `process_message()`, before emitting `turn_end`, build a summary and flush:

```rust
// --- Phase 4: Store turn end in LocusGraph ---
let session_ctx = memory::session_context_id(&self.session_slug, self.session.id.as_str());
let turn_ctx = self.turn_ctx();
let summary = self.build_turn_summary(&message);
let turn_end_event = memory::build_turn_end(
    &turn_ctx,
    &session_ctx,
    summary,
    self.turn_sequence,
    0, // TODO: track turn duration when turn-level timing is added
);
self.buffer_event(turn_end_event);

// Flush all buffered events for this turn
self.flush_turn_events().await;

// Emit turn end to TUI
let _ = self.event_tx.send(SessionEvent::turn_end()).await;
```

### Task 8: Add build_turn_summary helper

**File:** `crates/locus_runtime/src/runtime/mod.rs`

Add a method on Runtime to build a `TurnSummary` from the current session state:

```rust
use locus_graph::TurnSummary;

impl Runtime {
    /// Build a TurnSummary from the current turn's activity.
    ///
    /// Scans recent session turns (since last user message) to extract
    /// actions taken, files modified, and decisions.
    fn build_turn_summary(&self, user_message: &str) -> TurnSummary {
        let mut actions_taken = Vec::new();
        let mut files_read = Vec::new();
        let mut files_modified = Vec::new();
        let mut decisions = Vec::new();

        // Scan turns from the current turn onwards (after the last user message)
        let turns_this_round: Vec<&Turn> = self.session.turns.iter().rev()
            .take_while(|t| t.role != Role::User || t == self.session.turns.iter().rev().next().unwrap_or(t))
            .collect();

        for turn in turns_this_round.iter().rev() {
            for block in &turn.blocks {
                match block {
                    ContentBlock::ToolUse { tool_use } => {
                        actions_taken.push(format!("{}()", tool_use.name));
                        // Track file paths
                        if let Some(path) = tool_use.args.get("path").and_then(|v| v.as_str()) {
                            match tool_use.name.as_str() {
                                "edit_file" | "create_file" => {
                                    if !files_modified.contains(&path.to_string()) {
                                        files_modified.push(path.to_string());
                                    }
                                }
                                "glob" | "grep" | "finder" => {
                                    if !files_read.contains(&path.to_string()) {
                                        files_read.push(path.to_string());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    ContentBlock::Text { text } if turn.role == Role::Assistant => {
                        // First sentence of assistant text as a "decision"
                        if let Some(first_line) = text.lines().next() {
                            let trimmed = first_line.trim();
                            if !trimmed.is_empty() && trimmed.len() < 200 {
                                decisions.push(trimmed.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Build title from user message
        let title = if user_message.len() > 80 {
            format!("{}...", &user_message[..77])
        } else {
            user_message.to_string()
        };

        // Build outcome from last assistant text
        let outcome = self.session.turns.iter().rev()
            .find(|t| t.role == Role::Assistant)
            .and_then(|t| t.blocks.iter().find_map(|b| {
                if let ContentBlock::Text { text } = b {
                    let trimmed = text.trim();
                    if trimmed.len() > 200 {
                        Some(format!("{}...", &trimmed[..197]))
                    } else {
                        Some(trimmed.to_string())
                    }
                } else {
                    None
                }
            }))
            .unwrap_or_else(|| "Turn completed".to_string());

        TurnSummary {
            title,
            user_request: if user_message.len() > 500 {
                format!("{}...", &user_message[..497])
            } else {
                user_message.to_string()
            },
            actions_taken,
            outcome,
            decisions,
            files_read,
            files_modified,
            event_count: self.event_seq,
        }
    }
}
```

### Task 9: Add imports

**File:** `crates/locus_runtime/src/runtime/agent_loop.rs`

Add import for `session_context_id`:

```rust
use crate::memory;
// session_context_id is already accessible via memory::session_context_id
```

**File:** `crates/locus_runtime/src/runtime/llm.rs`

Add import:

```rust
use crate::memory;
```

**File:** `crates/locus_runtime/src/runtime/tools.rs`

Add import:

```rust
use crate::memory;
```

### Task 10: Update build_context_ids to include session context

**File:** `crates/locus_runtime/src/memory/recall.rs`

The `build_context_ids` function currently does not include the session context_id itself, only `session_anchor`. Add it so turn events that extend `session:` are retrievable:

```rust
pub fn build_context_ids(
    project_name: &str,
    repo_hash: &str,
    session_slug: &str,
    session_id: &str,       // NEW parameter
    turn_contexts: &[String],
) -> Vec<String> {
    let mut ids = vec![
        session_anchor_id(project_name, repo_hash),
        tool_anchor_id(project_name, repo_hash),
    ];

    // Add session context_id if we have a slug
    if !session_slug.is_empty() && !session_id.is_empty() {
        ids.push(super::session_context_id(session_slug, session_id));
    }

    // Add all known turn contexts for this session
    for turn_ctx in turn_contexts {
        ids.push(turn_ctx.clone());
    }

    ids
}
```

### Task 11: Update build_context_ids call sites

**File:** `crates/locus_runtime/src/runtime/mod.rs`

Update all calls to `build_context_ids` to pass `session_id`:

In `Runtime::new()` (line ~136):
```rust
let context_ids = memory::build_context_ids(&project_name, &repo_hash, "", "", &[]);
```

In `Runtime::new_with_shared()` (line ~186):
```rust
let context_ids = memory::build_context_ids(&project_name, &repo_hash, "", "", &[]);
```

In `Runtime::new_continuing()` (line ~230):
```rust
let context_ids = memory::build_context_ids(&project_name, &repo_hash, "", "", &[]);
```

**File:** `crates/locus_runtime/src/runtime/agent_loop.rs`

In `process_message()` (line ~247):
```rust
self.context_ids = memory::build_context_ids(
    &self.project_name,
    &self.repo_hash,
    &self.session_slug,
    self.session.id.as_str(),
    &existing_turns,
);
```

### Task 12: Update tests

**File:** `crates/locus_runtime/src/memory/tests.rs`

Update `test_build_context_ids` to pass `session_id` and verify session context is included:

```rust
#[test]
fn test_build_context_ids() {
    let turn_contexts: Vec<String> = vec!["turn:test-session_turn-1".to_string()];
    let ids = build_context_ids("locuscodes", "abc123", "test-session", "a1b2c3d4-uuid", &turn_contexts);

    assert!(ids.contains(&"session_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"tool_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"session:test-session_a1b2c3d4".to_string()));
    assert!(ids.contains(&"turn:test-session_turn-1".to_string()));
}

#[test]
fn test_build_context_ids_empty_session() {
    let ids = build_context_ids("locuscodes", "abc123", "", "", &[]);

    assert!(ids.contains(&"session_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"tool_anchor:locuscodes_abc123".to_string()));
    assert_eq!(ids.len(), 2); // No session context when slug/id empty
}
```

Add tests for turn event builders:

```rust
#[test]
fn test_build_turn_start() {
    let event = turns::build_turn_start(
        "turn:fix-jwt_validate-token",
        "session:fix-jwt_a1b2c3d4",
        "validate the JWT token",
        1,
    );
    assert_eq!(event.context_id.as_deref(), Some("turn:fix-jwt_validate-token"));
    assert!(event.extends.as_ref().unwrap().contains(&"session:fix-jwt_a1b2c3d4".to_string()));
}

#[test]
fn test_build_action_event() {
    let event = turns::build_action_event(
        "action:a1b2c3d4_001_001",
        "turn:fix-jwt_validate-token",
        "bash",
        &serde_json::json!({"command": "cargo test"}),
        &serde_json::json!({"output": "ok"}),
        false,
        150,
    );
    assert_eq!(event.context_id.as_deref(), Some("action:a1b2c3d4_001_001"));
    assert!(event.extends.as_ref().unwrap().contains(&"turn:fix-jwt_validate-token".to_string()));
}

#[test]
fn test_build_llm_event() {
    let event = turns::build_llm_event(
        "llm:a1b2c3d4_001_002",
        "turn:fix-jwt_validate-token",
        "claude-sonnet-4",
        1000,
        200,
        3500,
        true,
    );
    assert_eq!(event.context_id.as_deref(), Some("llm:a1b2c3d4_001_002"));
    let data = event.payload.get("data").unwrap();
    assert_eq!(data.get("total_tokens").unwrap(), 1200);
}
```

### Task 13: Inject turn summaries from LocusGraph into LLM context

**File:** `crates/locus_runtime/src/context/messages.rs`

This is the first place where LocusGraph turn history adds cross-session value. Update `build_session_context` to accept an optional `turn_summaries` parameter:

**Defer this to late Phase 4 or Phase 4b.** The current `recall_memories` in `prepare_llm_call` already does semantic search which will surface turn events once they're stored. No context builder change is needed for the basic Phase 4 to work. Turn summaries will naturally appear in the `## Relevant Memories` block via semantic search.

The explicit turn summary injection (fetching recent `turn:` events by type and prepending them as structured context) is an optimization that can be added after the basic pipeline is proven.

### Task 14: Verify no stale patterns

After all changes, run:

```bash
grep -rn "_session_id" crates/locus_runtime/src/tool_handler.rs
grep -rn "_turn_id" crates/locus_runtime/src/tool_handler.rs
grep -rn "_seq" crates/locus_runtime/src/tool_handler.rs
```

The `_` prefixed parameters in `handle_tool_call` should remain unchanged (they're still unused in tool_handler itself — the action events are built in `tools.rs` where the Runtime is available). But verify they don't cause dead code warnings.

---

## Verification

### Automated

```bash
cargo check -p locus-graph
cargo check -p locus-runtime
cargo test -p locus-runtime
cargo test -p locus-graph
cargo clippy -p locus-runtime -p locus-graph
```

All must pass.

### Manual

```bash
# New patterns — should exist
grep -rn "build_turn_start\|build_turn_end\|build_action_event\|build_llm_event" crates/locus_runtime/src/
grep -rn "buffer_event\|flush_turn_events" crates/locus_runtime/src/runtime/
grep -rn "turn_event_buffer" crates/locus_runtime/src/runtime/mod.rs

# Turn event module exists
test -f crates/locus_runtime/src/memory/turns.rs && echo "OK" || echo "MISSING"
```

### Runtime test (if environment is set up)

Start the runtime, send a message that triggers tool calls (e.g., "list the files in src/"), then quit. Check LocusGraph for:

1. `turn:{session_slug}_{turn_slug}` exists, extends `session:{slug}_{short_id}`
2. `intent:...` event exists, extends the turn
3. `action:...` event(s) exist for each tool call, extend the turn
4. `llm:...` event exists with token counts and duration
5. Turn anchor was overwritten at end with `status: "completed"` and summary fields
6. If a tool errored: `error:...` event exists
7. Semantic search for "list files" returns the turn and action events

---

## Files Changed (summary)

| File | Changes |
|---|---|
| `crates/locus_runtime/src/memory/turns.rs` | **NEW** — `build_turn_start()`, `build_turn_end()`, `build_action_event()`, `build_llm_event()`, `build_intent_event()`, `build_error_event()` |
| `crates/locus_runtime/src/memory/mod.rs` | Add `mod turns`, re-export turn event builders |
| `crates/locus_runtime/src/memory/recall.rs` | Update `build_context_ids()` to accept `session_id` param, include `session:` context |
| `crates/locus_runtime/src/memory/tests.rs` | Update `test_build_context_ids`, add tests for turn event builders |
| `crates/locus_runtime/src/runtime/mod.rs` | Add `turn_event_buffer`, `buffer_event()`, `flush_turn_events()`, `short_session_id()`, `event_ctx()`, `build_turn_summary()`. Update `build_context_ids` calls. |
| `crates/locus_runtime/src/runtime/agent_loop.rs` | Store turn start + intent at turn begin. Build summary + flush at turn end. Update `build_context_ids` call. |
| `crates/locus_runtime/src/runtime/tools.rs` | Buffer action events after tool calls. Buffer error events on failures. |
| `crates/locus_runtime/src/runtime/llm.rs` | Buffer LLM call events after streaming. Buffer error events on LLM failures. |

**Do NOT change:**
- `tool_handler.rs` — keep `_session_id`/`_turn_id`/`_seq` as-is. Action events are built in `tools.rs` where `self` is available.
- `context/messages.rs` — cross-session turn injection is deferred. Semantic search already surfaces turn events.
- `tools.md` — tool bootstrap docs, not turn lifecycle.
- `hooks.rs` — no turn constants.
- `memory/session.rs` — session lifecycle, not turn lifecycle.
- `memory/anchors.rs` — anchor helpers, no turn helpers needed (turn_ctx is built inline on Runtime).

---

## Write Buffer Design Notes

Phase 4 uses a simple `Vec<CreateEventRequest>` buffer. This is intentional:

- **No persistence** — if the process crashes mid-turn, buffered events are lost. This is acceptable: turn events are supplemental context, not critical state. The session itself (Phase 3) is persisted at start/end.
- **No batching** — events are flushed one-by-one via `store_event`. LocusGraph proxy already queues locally when `queue_stores` is enabled.
- **Fire-and-forget flush** — `flush_turn_events` spawns a background task. The next turn doesn't wait for the flush to complete.
- **Phase 11 upgrade path** — when cache.db is implemented, the buffer moves to SQLite with WAL, giving crash durability and true batch writes.

Expected events per turn: 1 intent + 1-10 actions + 1-3 LLM calls + 0-2 errors + 1 summary = **4-17 events**. At ~200 bytes per event, this is <4KB per turn — no memory concern.

---

## What NOT to do in Phase 4

- Do NOT implement cache.db — use the simple Vec buffer. Phase 11 upgrades this.
- Do NOT change how tools are dispatched or executed — only add event recording around the existing flow.
- Do NOT store raw LLM request/response bodies — only metadata (tokens, duration, model). Raw bodies are huge and already in tracing logs.
- Do NOT store full tool output in action events — truncate to 500 chars. Full output is in tracing logs and SessionEvent.
- Do NOT add knowledge_anchor or facts — that's Phase 5.
- Do NOT add rules, constraints, or implicit engine — that's Phase 6.
- Do NOT change the `handle_tool_call` signature or return type — build events in the caller (`tools.rs`) where Runtime state is available.
- Do NOT inject structured turn summaries into the context builder — rely on semantic search for now. Structured injection is a Phase 4b optimization.

**Phase 4 is about recording.** Every action becomes an event. Buffer during turn, flush at end. Turn anchor gets overwritten with summary. Semantic search starts returning real history. This is the flywheel start.
