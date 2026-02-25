# locus_graph — Session & Turn Context Plan

Add session/turn lifecycle hooks to `locus_graph` so the runtime can implement `context.md` (context master) alongside the existing `tools.md` support.

**Scope**: Only changes to `crates/locus_graph/`. No runtime changes.

---

## What Exists

- `src/client.rs` — `LocusGraphClient` wrapping locus-proxy gRPC. Has `store_event()`, `retrieve_memories()`, `generate_insights()`, `list_context_types()`, `search_contexts()`. Already enforces `{type}:{name}` via `sanitize_context_id()`.
- `src/types.rs` — `EventKind`, `CreateEventRequest`, `ContextResult`, `RetrieveOptions`, `EventLinks`
- `src/hooks.rs` — Existing hooks: `store_tool_run`, `store_file_edit`, `store_user_intent`, `store_error`, `store_decision`, `store_project_convention`, `store_skill`, `store_llm_call`, `store_test_run`, `store_git_op`, `store_tool_schema`, `store_tool_usage`
- `src/lib.rs` — Re-exports constants: `CONTEXT_DECISIONS`, `CONTEXT_EDITOR`, `CONTEXT_ERRORS`, `CONTEXT_TERMINAL`, `CONTEXT_TOOLS`, `CONTEXT_USER_INTENT`
- `src/error.rs` — `LocusGraphError`, `Result`

## What to Add

1. New constant: `CONTEXT_SESSIONS`
2. New type: `TurnSummary`
3. Fix `safe_context_name()` to allow hyphens (matches `sanitize_context_id()` in client.rs)
4. New hooks: `store_session_start`, `store_session_end`, `store_turn_start`, `store_turn_end`, `store_turn_event`, `store_snapshot`, `bootstrap_sessions_master`
5. Re-exports in `lib.rs`

### Important: context_id format alignment

Context.md uses semantic types as the type prefix:
- `{repo_hash}:sessions` — repo_hash IS the type
- `session:{slug}_{id}` — "session" is the type
- `turn:{sid}_{tid}` — "turn" is the type
- `action:{sid}_{tid}_{seq}` — "action" is the type

This is different from existing hooks which use event_kind as the prefix (`fact:tools`, `action:editor`). Both are valid `{type}:{name}`. The new session/turn hooks use the **context.md semantic types** — do NOT prepend `fact:` or `action:` to them.

### Important: `safe_context_name()` fix

The existing `safe_context_name()` strips hyphens, but `sanitize_context_id()` in client.rs allows them. Fix `safe_context_name()` to allow hyphens so slugs like `fix-jwt-refresh` stay as-is instead of becoming `fix_jwt_refresh`.

---

## Task 1: Add `CONTEXT_SESSIONS` constant

**File**: `src/hooks.rs`

Add alongside existing constants (keep alphabetical):

```rust
pub const CONTEXT_SESSIONS: &str = "fact:sessions";
```

**File**: `src/lib.rs`

Add to re-export:

```rust
pub use hooks::{
    CONTEXT_DECISIONS, CONTEXT_EDITOR, CONTEXT_ERRORS, CONTEXT_SESSIONS,
    CONTEXT_TERMINAL, CONTEXT_TOOLS, CONTEXT_USER_INTENT,
};
```

---

## Task 2: Add `TurnSummary` type

**File**: `src/types.rs`

Add after `EventLinks`:

```rust
/// Summary of a completed turn, stored at turn end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    /// Human-readable title for the turn
    pub title: String,
    /// Compressed user request
    pub user_request: String,
    /// High-level actions taken
    pub actions_taken: Vec<String>,
    /// Outcome description
    pub outcome: String,
    /// Key decisions made
    pub decisions: Vec<String>,
    /// Files that were read
    pub files_read: Vec<String>,
    /// Files that were modified
    pub files_modified: Vec<String>,
    /// Total events recorded in this turn
    pub event_count: u32,
}
```

**File**: `src/lib.rs`

Add `TurnSummary` to re-exports:

```rust
pub use types::{
    Context, ContextResult, ContextType, ContextTypeFilter, CreateEventRequest, EventKind,
    EventLinks, InsightResult, InsightsOptions, RetrieveOptions, TurnSummary,
};
```

---

## Task 3: Fix `safe_context_name()` to allow hyphens

**File**: `src/hooks.rs`

The existing function strips hyphens, but `sanitize_context_id()` in client.rs (line 37) allows them. Fix to match:

```rust
// BEFORE
fn safe_context_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

// AFTER
fn safe_context_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}
```

This ensures slugs like `fix-jwt-refresh` stay as `fix-jwt-refresh` instead of becoming `fix_jwt_refresh`.

---

## Task 4: Add session hooks

**File**: `src/hooks.rs`

Add to `impl LocusGraphClient`:

```rust
/// Create a session event at session start.
///
/// context_id: `session:{slug}_{session_id}`
/// extends: `{repo_hash}:sessions`
pub async fn store_session_start(
    &self,
    session_slug: &str,
    session_id: &str,
    title: &str,
    repo_hash: &str,
) {
    let ctx = format!("session:{}_{}", safe_context_name(session_slug), session_id);
    let sessions_master = format!("{}:sessions", safe_context_name(repo_hash));

    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "session_start",
            "data": {
                "title": title,
                "slug": session_slug,
                "session_id": session_id,
                "status": "active",
                "turn_count": 0,
                "totals": {
                    "events": 0,
                    "tool_calls": 0,
                    "llm_calls": 0,
                    "prompt_tokens": 0,
                    "completion_tokens": 0,
                    "files_modified": [],
                    "errors": 0,
                    "errors_resolved": 0
                }
            }
        }),
    )
    .context_id(ctx)
    .extends(vec![sessions_master])
    .source("system");

    self.store_event(event).await;
}

/// Close a session with final stats.
///
/// Same context_id as start — auto-overrides in LocusGraph.
pub async fn store_session_end(
    &self,
    session_slug: &str,
    session_id: &str,
    summary: &str,
    turn_count: u32,
    totals: serde_json::Value,
) {
    let ctx = format!("session:{}_{}", safe_context_name(session_slug), session_id);

    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "session_end",
            "data": {
                "status": "closed",
                "turn_count": turn_count,
                "summary": summary,
                "totals": totals,
            }
        }),
    )
    .context_id(ctx)
    .source("system");

    self.store_event(event).await;
}
```

---

## Task 5: Add turn hooks

**File**: `src/hooks.rs`

```rust
/// Create a turn anchor at turn START.
///
/// context_id: `turn:{session_id}_{turn_id}`
/// extends: `session:{slug}_{session_id}`
pub async fn store_turn_start(
    &self,
    session_id: &str,
    session_ctx: &str,
    turn_sequence: u32,
    user_message: &str,
) {
    let turn_id = format!("{:03}", turn_sequence);
    let ctx = format!("turn:{}_{}", session_id, turn_id);

    let event = CreateEventRequest::new(
        EventKind::Observation,
        json!({
            "kind": "turn_start",
            "data": {
                "turn_id": turn_id,
                "sequence": turn_sequence,
                "status": "active",
                "user_message": truncate_string(user_message, 1000),
            }
        }),
    )
    .context_id(ctx)
    .extends(vec![session_ctx.to_string()])
    .source("system");

    self.store_event(event).await;
}

/// Update turn anchor with summary at turn END.
///
/// Same context_id as start — auto-overrides.
pub async fn store_turn_end(
    &self,
    session_id: &str,
    session_ctx: &str,
    turn_sequence: u32,
    summary: TurnSummary,
) {
    let turn_id = format!("{:03}", turn_sequence);
    let ctx = format!("turn:{}_{}", session_id, turn_id);

    let event = CreateEventRequest::new(
        EventKind::Observation,
        json!({
            "kind": "turn_end",
            "data": {
                "turn_id": turn_id,
                "sequence": turn_sequence,
                "status": "completed",
                "title": summary.title,
                "user_request": summary.user_request,
                "actions_taken": summary.actions_taken,
                "outcome": summary.outcome,
                "decisions": summary.decisions,
                "files_read": summary.files_read,
                "files_modified": summary.files_modified,
                "event_count": summary.event_count,
            }
        }),
    )
    .context_id(ctx)
    .extends(vec![session_ctx.to_string()])
    .source("agent");

    self.store_event(event).await;
}

/// Store any event during a turn (the full timeline).
///
/// context_id: `{event_type}:{session_id}_{turn_id}_{seq}`
/// extends: `turn:{session_id}_{turn_id}`
pub async fn store_turn_event(
    &self,
    event_type: &str,
    session_id: &str,
    turn_id: &str,
    seq: u32,
    event_kind: EventKind,
    source: &str,
    payload: serde_json::Value,
    related_to: Option<Vec<String>>,
) {
    let ctx = format!(
        "{}:{}_{}_{:03}",
        safe_context_name(event_type),
        session_id,
        turn_id,
        seq
    );
    let turn_ctx = format!("turn:{}_{}", session_id, turn_id);

    let mut event = CreateEventRequest::new(event_kind, payload)
        .context_id(ctx)
        .extends(vec![turn_ctx])
        .source(source);

    if let Some(refs) = related_to {
        event = event.related_to(refs);
    }

    self.store_event(event).await;
}
```

---

## Task 6: Add snapshot hook

**File**: `src/hooks.rs`

```rust
/// Store a codebase snapshot at turn boundaries.
///
/// context_id: `snapshot:{session_id}_{turn_id}_{seq}`
pub async fn store_snapshot(
    &self,
    session_id: &str,
    turn_id: &str,
    seq: u32,
    git_head: &str,
    git_branch: &str,
    git_dirty: Vec<String>,
    git_staged: Vec<String>,
    snapshot_type: &str,  // "turn_start" or "turn_end"
) {
    let ctx = format!("snapshot:{}_{}_{:03}", session_id, turn_id, seq);
    let turn_ctx = format!("turn:{}_{}", session_id, turn_id);

    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "codebase_snapshot",
            "data": {
                "git_head": git_head,
                "git_branch": git_branch,
                "git_dirty": git_dirty,
                "git_staged": git_staged,
                "snapshot_type": snapshot_type,
                "seq": seq,
            }
        }),
    )
    .context_id(ctx)
    .extends(vec![turn_ctx])
    .source("system");

    self.store_event(event).await;
}
```

---

## Task 7: Add sessions master bootstrap hook

**File**: `src/hooks.rs`

```rust
/// Bootstrap the sessions master event (cold start).
///
/// context_id: `{repo_hash}:sessions`
pub async fn bootstrap_sessions_master(
    &self,
    repo_hash: &str,
    project_anchor: &str,
) {
    let ctx = format!("{}:sessions", safe_context_name(repo_hash));

    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "sessions_master",
            "data": {
                "active_session": null,
                "total_sessions": 0
            }
        }),
    )
    .context_id(ctx)
    .extends(vec![project_anchor.to_string()])
    .source("system");

    self.store_event(event).await;
}
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/hooks.rs` | Add `CONTEXT_SESSIONS`, `store_session_start`, `store_session_end`, `store_turn_start`, `store_turn_end`, `store_turn_event`, `store_snapshot`, `bootstrap_sessions_master` |
| `src/types.rs` | Add `TurnSummary` struct |
| `src/lib.rs` | Add `CONTEXT_SESSIONS` and `TurnSummary` to re-exports |

## Files NOT Changed

- `src/client.rs` — no new client methods
- `src/config.rs` — no config changes
- `src/error.rs` — no new errors

---

## Verify

```bash
cargo check -p locus-graph
cargo clippy -p locus-graph
cargo test -p locus-graph
```

---

## Notes

- All context_ids are strict `{type}:{name}` — enforced by existing `sanitize_context_id()` in client.rs
- All hooks follow the existing pattern: build `CreateEventRequest`, call `self.store_event()`, fire-and-forget
- `TurnSummary` is the only new type — keeps the type surface small
- The runtime is responsible for orchestration (when to call these hooks). This crate only provides the storage API.
- Future: `store_turn_event` handles dynamic tool events too — same API, just different `event_type` ("mcp_action", etc.)
