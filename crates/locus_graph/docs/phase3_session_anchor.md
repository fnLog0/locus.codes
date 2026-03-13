# Phase 3 — Session Anchor + Sessions

**Goal:** Introduce `session_anchor:{project_name}_{repo_hash}` and `session:{slug}_{session_id}` as the session lifecycle layer. Replace the stale `{hash}:sessions` pattern. Add session create, resume, and close operations.

This document is the concrete execution checklist for Phase 3. If it conflicts with `implementation_plan.md`, the main implementation plan wins.

**Hierarchy being built:**

```
project:{project_name}_{repo_hash}                    ← Phase 1 (exists)
  └── session_anchor:{project_name}_{repo_hash}       ← NEW
        └── session:{slug}_{session_id}               ← NEW
```

**Depends on:** Phase 1 (session_anchor extends project root).

---

## Reference: Session Lifecycle

```
CLI launch
  → check session_anchor → active_session?
    → YES: resume that session, fetch existing turn contexts
    → NO: create new session
  → user interacts...
  → session end (user quits / max turns / error)
    → close session, clear active_session
```

---

## What Exists Now

### Current session handling

In `runtime/mod.rs`:
- `session_slug: String` — set from first user message via `slugify()` (line 226 of agent_loop.rs)
- `session.id` — UUID set by `Session::new()` in locus_core

In `memory.rs`:
- `build_context_ids()` line 87: `format!("{}:sessions", repo_hash)` — stale placeholder, not a real context in LocusGraph
- `fetch_session_turns()` — calls `locus_graph.fetch_session_turns(session_slug)`

In `agent_loop.rs` `process_message()` (line ~225):
```rust
if self.session_slug.is_empty() {
    self.session_slug = Self::slugify(&message);
}
// Fetch existing turn contexts
let existing_turns = memory::fetch_session_turns(&self.locus_graph, &self.session_slug).await;
// ...
self.context_ids = memory::build_context_ids(
    &self.project_name,
    &self.repo_hash,
    &self.session_slug,
    &existing_turns,
);
```

**Problems:**
1. `{hash}:sessions` — not a real context_id in the graph. Should be `session_anchor:{project_name}_{hash}`.
2. No session event is stored in LocusGraph — the session exists only in-memory (`Session` struct).
3. No active_session tracking — can't resume sessions.
4. `fetch_session_turns` searches by slug but there's no session context_id to scope it.

### Available gRPC methods (via locus_proxy)

| Method | Used for Phase 3 |
|---|---|
| `store_event(CreateEventRequest)` | Creating session_anchor, session events |
| `search_contexts(query, type, page, page_size)` | Finding active session by searching session_anchor |
| `list_contexts_by_type(type, page, page_size)` | Listing all sessions |
| `retrieve_memories(query, options)` | Fetching session turns (already used) |

---

## Tasks

### Task 1: Add session_anchor_id and session_context_id helpers

**File:** `crates/locus_runtime/src/memory.rs`

```rust
/// Build the session anchor context_id.
/// Format: "session_anchor:{project_name}_{repo_hash}"
pub fn session_anchor_id(project_name: &str, repo_hash: &str) -> String {
    format!(
        "session_anchor:{}_{}",
        safe_context_name(project_name),
        safe_context_name(repo_hash)
    )
}

/// Build a session context_id.
/// Format: "session:{slug}_{session_id_short}"
/// session_id_short is first 8 chars of the UUID.
pub fn session_context_id(slug: &str, session_id: &str) -> String {
    let short_id = if session_id.len() > 8 {
        &session_id[..8]
    } else {
        session_id
    };
    format!(
        "session:{}_{}",
        safe_context_name(slug),
        safe_context_name(short_id)
    )
}
```

### Task 2: Add ensure_session_anchor function

**File:** `crates/locus_runtime/src/memory.rs`

```rust
/// Ensure the session anchor exists in LocusGraph.
/// Idempotent — same context_id = overwrite.
/// Called at Runtime::new() after ensure_project_anchor.
pub async fn ensure_session_anchor(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
) {
    use locus_graph::{CreateEventRequest, EventKind};

    let anchor_id = session_anchor_id(project_name, repo_hash);
    let project_anchor = project_anchor_id(project_name, repo_hash);

    let event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session_anchor",
            "data": {
                "project_name": project_name,
                "repo_hash": repo_hash,
                "active_session": null,
            }
        }),
    )
    .context_id(anchor_id)
    .extends(vec![project_anchor])
    .source("validator");

    locus_graph.store_event(event).await;
}
```

### Task 3: Add store_session_start function

**File:** `crates/locus_runtime/src/memory.rs`

Creates the session event and updates session_anchor with active_session.

```rust
/// Store a new session event in LocusGraph and mark it active.
/// Called when the first user message arrives (session_slug is known).
pub async fn store_session_start(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
    session_slug: &str,
    session_id: &str,
) {
    use locus_graph::{CreateEventRequest, EventKind};

    let session_ctx = session_context_id(session_slug, session_id);
    let anchor = session_anchor_id(project_name, repo_hash);

    // Create session event
    let session_event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session",
            "data": {
                "slug": session_slug,
                "session_id": session_id,
                "started_at": chrono::Utc::now().to_rfc3339(),
                "status": "active",
                "turn_count": 0,
            }
        }),
    )
    .context_id(session_ctx.clone())
    .extends(vec![anchor.clone()])
    .source("validator");

    locus_graph.store_event(session_event).await;

    // Update session_anchor with active_session
    let project_anchor = project_anchor_id(project_name, repo_hash);
    let anchor_update = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session_anchor",
            "data": {
                "project_name": project_name,
                "repo_hash": repo_hash,
                "active_session": session_ctx,
            }
        }),
    )
    .context_id(anchor)
    .extends(vec![project_anchor])
    .source("validator");

    locus_graph.store_event(anchor_update).await;
}
```

### Task 4: Add store_session_end function

**File:** `crates/locus_runtime/src/memory.rs`

```rust
/// Close a session in LocusGraph — update status, clear active_session.
/// Called at session shutdown.
pub async fn store_session_end(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
    session_slug: &str,
    session_id: &str,
    turn_count: u32,
) {
    use locus_graph::{CreateEventRequest, EventKind};

    let session_ctx = session_context_id(session_slug, session_id);
    let anchor = session_anchor_id(project_name, repo_hash);

    // Update session with closed status (same context_id = overwrite)
    let session_event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session",
            "data": {
                "slug": session_slug,
                "session_id": session_id,
                "status": "closed",
                "ended_at": chrono::Utc::now().to_rfc3339(),
                "turn_count": turn_count,
            }
        }),
    )
    .context_id(session_ctx)
    .extends(vec![anchor.clone()])
    .source("validator");

    locus_graph.store_event(session_event).await;

    // Clear active_session on session_anchor
    let project_anchor = project_anchor_id(project_name, repo_hash);
    let anchor_update = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session_anchor",
            "data": {
                "project_name": project_name,
                "repo_hash": repo_hash,
                "active_session": null,
            }
        }),
    )
    .context_id(anchor)
    .extends(vec![project_anchor])
    .source("validator");

    locus_graph.store_event(anchor_update).await;
}
```

### Task 5: Call ensure_session_anchor at startup

**File:** `crates/locus_runtime/src/runtime/mod.rs`

In `Runtime::new()`, after `ensure_project_anchor` but before `bootstrap_tools`:

```rust
// Ensure session anchor exists (idempotent)
memory::ensure_session_anchor(&locus_graph, &project_name, &repo_hash).await;
```

### Task 6: Call store_session_start in process_message

**File:** `crates/locus_runtime/src/runtime/agent_loop.rs`

In `process_message()`, after setting `session_slug` (line ~226):

```rust
if self.session_slug.is_empty() {
    self.session_slug = Self::slugify(&message);

    // Store session start in LocusGraph
    memory::store_session_start(
        &self.locus_graph,
        &self.project_name,
        &self.repo_hash,
        &self.session_slug,
        &self.session.id,
    )
    .await;
}
```

Note: `store_session_start` is async but should be fire-and-forget in production. For Phase 3, calling `.await` is fine. In Phase 11 (cache.db), this becomes a local write.

### Task 7: Call store_session_end in shutdown

**File:** `crates/locus_runtime/src/runtime/mod.rs`

In `Runtime::shutdown()`, before setting status:

```rust
pub async fn shutdown(&mut self) -> Result<(), RuntimeError> {
    info!("Shutting down runtime");

    // Close session in LocusGraph
    if !self.session_slug.is_empty() {
        memory::store_session_end(
            &self.locus_graph,
            &self.project_name,
            &self.repo_hash,
            &self.session_slug,
            &self.session.id,
            self.turn_sequence,
        )
        .await;
    }

    self.session.set_status(SessionStatus::Completed);
    let _ = self
        .event_tx
        .send(SessionEvent::status("Session ended"))
        .await;
    Ok(())
}
```

### Task 8: Update build_context_ids to use session_anchor_id

**File:** `crates/locus_runtime/src/memory.rs`

Current (line 87):
```rust
let mut ids = vec![format!("{}:sessions", repo_hash), tool_anchor_id(project_name, repo_hash)];
```

New:
```rust
let mut ids = vec![
    session_anchor_id(project_name, repo_hash),
    tool_anchor_id(project_name, repo_hash),
];
```

Note: `tool_anchor_id` comes from Phase 2. If Phase 2 isn't done yet, this line will still have the Phase 2 placeholder — just update the sessions part.

### Task 9: Update fetch_session_turns to use session context_id

**File:** `crates/locus_runtime/src/memory.rs`

The existing `fetch_session_turns` takes a `session_slug`. It should now scope to the session context_id for better precision.

Update signature to accept both slug and session_id:

```rust
pub async fn fetch_session_turns(
    locus_graph: &LocusGraphClient,
    session_slug: &str,
    _session_id: &str,  // For future use when scoping to specific session
) -> Vec<String> {
    locus_graph
        .fetch_session_turns(session_slug)
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to fetch session turns: {}", e);
            vec![]
        })
}
```

Update call site in `agent_loop.rs`:
```rust
let existing_turns =
    memory::fetch_session_turns(&self.locus_graph, &self.session_slug, &self.session.id).await;
```

### Task 10: Update tools.md (no changes needed for Phase 3)

`tools.md` is about tool bootstrap only. Session lifecycle doesn't affect it. **No changes to tools.md.**

### Task 11: Update tests

**File:** `crates/locus_runtime/src/memory.rs`

Update `test_build_context_ids` — the `{hash}:sessions` assertion changes:

```rust
#[test]
fn test_build_context_ids() {
    let turn_contexts: Vec<String> = vec!["turn:test-session_turn-1".to_string()];
    let ids = build_context_ids("locuscodes", "abc123", "test-session", &turn_contexts);

    assert!(ids.contains(&"session_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"tool_anchor:locuscodes_abc123".to_string())); // Phase 2
    assert!(ids.contains(&"turn:test-session_turn-1".to_string()));
}
```

Add tests for new helpers:

```rust
#[test]
fn test_session_anchor_id() {
    let id = session_anchor_id("locuscodes", "abc123");
    assert_eq!(id, "session_anchor:locuscodes_abc123");
}

#[test]
fn test_session_context_id() {
    let id = session_context_id("fix-jwt-bug", "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    assert_eq!(id, "session:fix-jwt-bug_a1b2c3d4");
}

#[test]
fn test_session_context_id_short() {
    let id = session_context_id("my-session", "abc123");
    assert_eq!(id, "session:my-session_abc123");
}
```

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
# Old pattern — should find ZERO hits in source code
grep -rn '".*:sessions"' crates/locus_runtime/src/

# New pattern — should exist
grep -rn "session_anchor:" crates/locus_runtime/src/memory.rs
grep -rn "store_session_start\|store_session_end" crates/locus_runtime/src/
```

### Runtime test (if environment is set up)

Start the runtime, send a message, then quit. Check LocusGraph for:
- `session_anchor:{project_name}_{repo_hash}` exists, extends `project:`
- `session:{slug}_{short_id}` exists, extends `session_anchor:`, status "active" during run
- After quit: session status is "closed", `active_session` on anchor is null
- Restart: new session created (different session_id), old session stays closed

---

## Files Changed (summary)

| File | Changes |
|---|---|
| `crates/locus_runtime/src/memory.rs` | Add `session_anchor_id()`, `session_context_id()`, `ensure_session_anchor()`, `store_session_start()`, `store_session_end()`. Update `build_context_ids()` to use `session_anchor_id`. Update `fetch_session_turns()` signature. Update tests. |
| `crates/locus_runtime/src/runtime/mod.rs` | Call `ensure_session_anchor()` at startup. Call `store_session_end()` in shutdown. |
| `crates/locus_runtime/src/runtime/agent_loop.rs` | Call `store_session_start()` when session_slug is set. Update `fetch_session_turns()` call. |

**Do NOT change:**
- `tools.md` — tool bootstrap only, not session lifecycle
- `hierarchy.md` — already uses new pattern
- `hooks.rs` — no session constants
- `tool_handler.rs` — not session-related

---

## Session Resume (Future Enhancement)

Phase 3 stores sessions and tracks `active_session`, but does **not** implement full resume logic yet. Full resume requires:

1. At CLI launch, check `session_anchor` → `active_session`
2. If set (unclean shutdown), prompt user: "Resume previous session?"
3. If yes, reload session state from LocusGraph events
4. If no, close the stale session, create new

This is a CLI-level feature. The Phase 3 foundation (store/close/active_session tracking) enables it, but the CLI integration is deferred until the CLI crate is more developed.

For now, every `Runtime::new()` always creates a new session. The `active_session` field exists in the anchor for when resume is implemented.

---

## What NOT to do in Phase 3

- Do NOT create `turn:` events — that's Phase 4
- Do NOT store per-turn events (snapshot, intent, action, etc.) — that's Phase 4
- Do NOT add a write buffer / cache.db — that's Phase 4/11
- Do NOT implement session resume in the CLI — foundation only
- Do NOT change tool bootstrap or tool_search — that's Phase 2
- Do NOT add knowledge_anchor or facts — that's Phase 5
- Do NOT inject session history into LLM context from LocusGraph — the existing `Session.turns` in-memory history is sufficient for now. Phase 4 will add cross-session history.

**Phase 3 is ONLY about the session lifecycle.** Create, track, close. Three events (anchor, start, end). Verify. Move on.
