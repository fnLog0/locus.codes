# LocusGraph — Session & Turn Context (Runtime Context Master)

LocusGraph is the **context master** — it preserves **everything** that happens during a session and turn (every file read, edit, tool call, decision, error, LLM call) so future sessions and turns can look back at the full record. For the LLM context window, it retrieves compressed summaries. For deep recall, it drills into the full timeline.

---

## Prerequisites

- `repo_hash` — deterministic hash of the repository.
- `project_name` — human-readable project name.
- `knowledge:{project_name}_{repo_hash}` — project anchor (created during tool bootstrap, see `tools.md`).

---

## Context ID Format

All context_ids follow the strict `{type}:{name}` two-layer format.

| Level | context_id | extends |
|---|---|---|
| **Sessions master** | `{repo_hash}:sessions` | `knowledge:{project_name}_{repo_hash}` |
| **Session** | `session:{slug}_{session_id}` | `{repo_hash}:sessions` |
| **Turn** | `turn:{session_id}_{turn_id}` | `session:{slug}_{session_id}` |
| **Turn events** | `{event_type}:{session_id}_{turn_id}_{seq}` | `turn:{session_id}_{turn_id}` |

### Event types as context_id types

Turn events use the **event category as the type prefix**:

| type | What it stores |
|---|---|
| `intent` | User message, intent summary |
| `action` | Tool call — name, args, result, duration |
| `file` | File change — path, diff, before/after content hashes |
| `snapshot` | Codebase state — git HEAD, dirty files, tree hash |
| `decision` | Agent decision — reasoning, alternatives |
| `error` | Error — type, message, resolution |
| `llm` | LLM call — model, tokens, duration |
| `feedback` | User feedback — approval, rejection, correction |

### Naming rules

- **session_id** — nanoid(8). Short, unique.
- **turn_id** — sequential within session (`001`, `002`, `003`).
- **seq** — global sequential within turn (`001`, `002`…). Single counter across all event types — guarantees chronological order.
- **slug** — kebab-case, max 30 chars, from user's first message. Fallback: `session-{session_id}` if message is too vague.

### Examples

```
abc123:sessions                           ← master
session:fix-jwt-refresh_a1b2c3d4          ← session
turn:a1b2c3d4_001                         ← turn 1 (created at START, updated at END)
  snapshot:a1b2c3d4_001_001               ←   codebase state at turn start
  intent:a1b2c3d4_001_002                 ←   user message
  action:a1b2c3d4_001_003                 ←   read src/auth/jwt.rs
  action:a1b2c3d4_001_004                 ←   read src/middleware/auth.rs
  decision:a1b2c3d4_001_005              ←   decided to check refresh logic
  llm:a1b2c3d4_001_006                    ←   LLM call record
turn:a1b2c3d4_002                         ← turn 2
  intent:a1b2c3d4_002_001                 ←   user message
  action:a1b2c3d4_002_002                 ←   edit src/auth/jwt.rs
  file:a1b2c3d4_002_003                   ←   file diff record
  decision:a1b2c3d4_002_004              ←   validate before refresh
  llm:a1b2c3d4_002_005                    ←   LLM call
  feedback:a1b2c3d4_002_006              ←   user: "looks good"
turn:a1b2c3d4_003                         ← turn 3
  intent:a1b2c3d4_003_001                 ←   user message
  action:a1b2c3d4_003_002                 ←   bash: cargo test
  error:a1b2c3d4_003_003                  ←   compile error
  action:a1b2c3d4_003_004                 ←   edit fix
  file:a1b2c3d4_003_005                   ←   file diff
  action:a1b2c3d4_003_006                 ←   bash: cargo test (pass)
  llm:a1b2c3d4_003_007                    ←   LLM call
```

### Two layers per turn

| Layer | What | When retrieved |
|---|---|---|
| **Turn anchor** (`turn:{sid}_{tid}`) | Created at START (minimal), updated at END (full summary) | Default — injected into LLM context for continuity |
| **Turn events** (`{type}:{sid}_{tid}_{seq}`) | Full detail: every tool call, diff, decision, in order | On demand — when agent needs to "look back" at specifics |

### Why event type as context_id type

Using `action:`, `decision:`, `file:`, etc. as the type prefix gives **free cross-cutting queries**:
- Filter by type `action` → all tool calls across all sessions
- Filter by type `decision` → all decisions ever made
- Filter by type `error` → every error across the project
- Filter by type `file` → every file change with diffs

The name part `{session_id}_{turn_id}_{seq}` encodes location + order. `extends` links to the parent turn for graph traversal.

---

## Step 1 — Sessions Master (Bootstrap)

Created once during cold start, alongside tool bootstrap. Check if `{repo_hash}:sessions` exists. If **not**, create it:

```json
{
  "context_id": "{repo_hash}:sessions",
  "event_kind": "fact",
  "source": "system",
  "payload": {
    "active_session": null,
    "total_sessions": 0
  },
  "extends": ["knowledge:{project_name}_{repo_hash}"]
}
```

---

## Step 2 — Session Creation

When the user starts a new interaction (CLI launch, new chat):

```rust
let session_id = nanoid(8);
let slug = slugify(&first_user_message, 30);
let slug = if slug.len() < 4 { format!("session-{}", session_id) } else { slug };
let session_ctx = format!("session:{}_{}", slug, session_id);

// 1. Store session event
locus_graph.store_event(Event {
    context_id: session_ctx.clone(),
    event_kind: "fact",
    source: "system",
    payload: json!({
        "title":      first_user_message,
        "slug":       slug,
        "session_id": session_id,
        "started_at": now_iso8601(),
        "status":     "active",
        "turn_count": 0,
        "totals": {
            "events":            0,
            "tool_calls":        0,
            "llm_calls":         0,
            "prompt_tokens":     0,
            "completion_tokens": 0,
            "files_modified":    [],
            "errors":            0,
            "errors_resolved":   0
        }
    }),
    extends: vec![format!("{}:sessions", repo_hash)],
});

// 2. Update sessions master
locus_graph.store_event(Event {
    context_id: format!("{}:sessions", repo_hash),
    event_kind: "fact",
    source: "system",
    payload: json!({
        "active_session": session_ctx,
        "total_sessions": previous_total + 1
    }),
    extends: vec![format!("knowledge:{}_{}", project_name, repo_hash)],
});
```

### Session Resume

On CLI launch, check `{repo_hash}:sessions` for `active_session`:

| Condition | Action |
|---|---|
| `active_session` is not null | Resume — retrieve session context, continue |
| `active_session` is null | Prompt for new session or list recent sessions |
| Explicit `--new` flag | Always create new session |

---

## Step 3 — Turn Start (Anchor + Context Retrieval)

**The turn anchor is created at turn START** — not at the end. This ensures every event has a real parent to `extend`. At turn END, the same `context_id` is overwritten with the full summary.

```rust
let turn_id = format!("{:03}", turn_sequence);
let turn_ctx = format!("turn:{}_{}", session_id, turn_id);

// 1. Create turn anchor FIRST — events will extend this
locus_graph.store_event(Event {
    context_id: turn_ctx.clone(),
    event_kind: "observation",
    source: "system",
    payload: json!({
        "turn_id":      turn_id,
        "sequence":     turn_sequence,
        "started_at":   now_iso8601(),
        "status":       "active",
        "user_message": user_message,
    }),
    extends: vec![session_ctx.clone()],
});

// 2. Capture codebase snapshot (seq 001 — always first event)
let git_head = git_rev_parse("HEAD")?;
let git_dirty = git_status_dirty()?;
store_turn_event("snapshot", session_id, &turn_id, &turn_ctx, 1,
    EventKind::Fact, "system",
    json!({
        "git_head":      git_head,
        "git_branch":    git_current_branch()?,
        "git_dirty":     git_dirty,
        "git_staged":    git_status_staged()?,
        "tree_hash":     git_tree_hash()?,
        "snapshot_type": "turn_start",
        "seq": 1
    }), None
).await;

// 3. Retrieve context from LocusGraph
let session = retrieve_context(&session_ctx_id);

let recent_turns = retrieve_memories(
    "recent turns",
    Some(RetrieveOptions::new()
        .context_type("turn")
        .limit(5))
).await?;

let relevant = retrieve_memories(
    &user_message,
    Some(RetrieveOptions::new().limit(10))
).await?;

// 4. Build context block for LLM
let context = ContextBlock {
    project:       project_knowledge,
    session_goal:  session.payload,
    recent_turns:  recent_turns.memories,
    relevant:      relevant.memories,
    current_input: user_message,
};
```

### Context Window Construction

```
System prompt
├── Project knowledge          (fact, always included)
├── Session goal               (fact, from session event)
├── Recent turn summaries      (last 3-5, from turn anchors)
├── Relevant memories          (semantic search, cross-session)
└── Current user message
```

---

## Step 4 — During Turn (Full Preservation)

**Every action** is stored with a global sequential counter. The event category IS the context_id type — strict `{type}:{name}`.

```rust
let turn_ctx = format!("turn:{}_{}", session_id, turn_id);
let mut seq = 0u32;

fn next_ctx(event_type: &str, session_id: &str, turn_id: &str, seq: &mut u32) -> String {
    *seq += 1;
    format!("{}:{}_{}_{:03}", event_type, session_id, turn_id, seq)
}
// → "action:a1b2c3d4_001_002"
```

### 4a — User Message

```json
{
  "context_id": "intent:a1b2c3d4_001_001",
  "event_kind": "observation",
  "source": "user",
  "payload": {
    "message":        "Fix the JWT refresh bug in auth middleware",
    "intent_summary": "Fix JWT token refresh validation",
    "seq": 1
  },
  "extends": ["turn:a1b2c3d4_001"]
}
```

### 4b — Tool Calls

```json
{
  "context_id": "action:a1b2c3d4_001_002",
  "event_kind": "action",
  "source": "executor",
  "payload": {
    "tool":        "read",
    "args":        { "path": "src/auth/jwt.rs" },
    "result":      { "lines": 245, "language": "rust" },
    "duration_ms": 12,
    "success":     true,
    "seq": 2
  },
  "extends": ["turn:a1b2c3d4_001"]
}
```

```json
{
  "context_id": "action:a1b2c3d4_002_002",
  "event_kind": "action",
  "source": "executor",
  "payload": {
    "tool":        "edit_file",
    "args":        { "path": "src/auth/jwt.rs", "old_str": "...", "new_str": "..." },
    "result":      { "lines_changed": 5 },
    "duration_ms": 45,
    "success":     true,
    "seq": 2
  },
  "extends": ["turn:a1b2c3d4_002"]
}
```

### 4c — File Changes (with content hashes)

Every file modification stores the diff in LocusGraph AND saves before/after content to `.locus/snapshots/{hash}`. The hashes let you reconstruct the exact file state at any point.

```json
{
  "context_id": "file:a1b2c3d4_002_003",
  "event_kind": "action",
  "source": "executor",
  "payload": {
    "path":           "src/auth/jwt.rs",
    "operation":      "edit",
    "diff":           "@@ -42,3 +42,8 @@\n+    if !validate_token(&refresh_token) {...}",
    "lines_added":    3,
    "lines_removed":  0,
    "description":    "Added validation check before token refresh",
    "before_hash":    "sha256:a3f2c8...",
    "after_hash":     "sha256:b7d1e4...",
    "after_size":     6820,
    "seq": 3
  },
  "extends":    ["turn:a1b2c3d4_002"],
  "related_to": ["editor"]
}
```

### 4d — Decisions

```json
{
  "context_id": "decision:a1b2c3d4_002_004",
  "event_kind": "decision",
  "source": "agent",
  "payload": {
    "decision":    "Add validation check before token refresh",
    "reasoning":   "Current code skips validation when refresh_token is present",
    "confidence":  0.9,
    "alternatives_considered": ["Reject all refresh tokens", "Add rate limiting"],
    "seq": 4
  },
  "extends":    ["turn:a1b2c3d4_002"],
  "related_to": ["decisions"]
}
```

### 4e — Errors

```json
{
  "context_id": "error:a1b2c3d4_003_003",
  "event_kind": "observation",
  "source": "system",
  "payload": {
    "error_type":  "compile_error",
    "message":     "mismatched types: expected &str, found String",
    "file":        "src/auth/jwt.rs",
    "line":        44,
    "tool":        "bash",
    "command":     "cargo check",
    "resolved":    true,
    "resolution":  "Added .as_str() to convert String to &str",
    "seq": 3
  },
  "extends":    ["turn:a1b2c3d4_003"],
  "related_to": ["errors"]
}
```

### 4f — LLM Calls

```json
{
  "context_id": "llm:a1b2c3d4_001_005",
  "event_kind": "action",
  "source": "system",
  "payload": {
    "model":             "claude-3-opus",
    "prompt_tokens":     2400,
    "completion_tokens": 800,
    "duration_ms":       3500,
    "context_memories":  5,
    "stop_reason":       "end_turn",
    "seq": 5
  },
  "extends": ["turn:a1b2c3d4_001"]
}
```

### 4g — User Feedback

```json
{
  "context_id": "feedback:a1b2c3d4_002_006",
  "event_kind": "feedback",
  "source": "user",
  "payload": {
    "type":      "approval",
    "message":   "That looks good, now run the tests",
    "sentiment": "positive",
    "seq": 6
  },
  "extends": ["turn:a1b2c3d4_002"]
}
```

### 4h — Codebase Snapshots

Stored at **turn start** (first event, seq 001). Captures the exact codebase state so any turn can be reconstructed.

```json
{
  "context_id": "snapshot:a1b2c3d4_001_001",
  "event_kind": "fact",
  "source": "system",
  "payload": {
    "git_head":       "abc123def456",
    "git_branch":     "main",
    "git_dirty":      ["src/auth/jwt.rs", "src/middleware/auth.rs"],
    "git_staged":     [],
    "tree_hash":      "sha256:e9f3a1...",
    "tracked_files":  142,
    "snapshot_type":  "turn_start",
    "seq": 1
  },
  "extends": ["turn:a1b2c3d4_001"]
}
```

For file-heavy turns, a second snapshot at turn end captures the final state:

```json
{
  "context_id": "snapshot:a1b2c3d4_001_007",
  "event_kind": "fact",
  "source": "system",
  "payload": {
    "git_head":       "abc123def456",
    "git_dirty":      [],
    "tree_hash":      "sha256:f2b8c7...",
    "files_changed":  2,
    "snapshot_type":  "turn_end",
    "seq": 7
  },
  "extends": ["turn:a1b2c3d4_001"]
}
```

### Local Storage — `.locus/snapshots/`

Full file content is **too large for LocusGraph events**. Instead, content is stored locally and referenced by hash:

```
.locus/
├── locus.db                    ← edit history + config
├── locus_graph_cache.db        ← LocusGraph cache/queue
└── snapshots/                  ← file content by hash
    ├── sha256_a3f2c8...        ← before content of jwt.rs
    ├── sha256_b7d1e4...        ← after content of jwt.rs
    └── sha256_e9f3a1...        ← tree manifests (optional)
```

**Storage flow**:
```rust
// On file edit:
let before_content = fs::read_to_string(&path)?;
let before_hash = sha256(&before_content);
fs::write(format!(".locus/snapshots/sha256_{}", &before_hash[..12]), &before_content)?;

// ... edit happens ...

let after_content = fs::read_to_string(&path)?;
let after_hash = sha256(&after_content);
fs::write(format!(".locus/snapshots/sha256_{}", &after_hash[..12]), &after_content)?;

// Store file event with hashes (content is in local storage, not LocusGraph)
store_turn_event("file", ..., json!({
    "path": path,
    "diff": diff,
    "before_hash": format!("sha256:{}", &before_hash[..12]),
    "after_hash":  format!("sha256:{}", &after_hash[..12]),
    ...
}));
```

**Reconstruction flow**:
```rust
// "Show me jwt.rs as it was at turn 2, event 3"
let file_event = retrieve("file:a1b2c3d4_002_003");
let hash = file_event.payload.after_hash;  // "sha256:b7d1e4..."
let content = fs::read_to_string(format!(".locus/snapshots/{}", hash))?;
// → exact file content at that point in time
```

**Cleanup**: Snapshots can be garbage-collected when sessions are archived. Content shared across events (same hash = same content) is stored once (content-addressable).

### What Gets Preserved (complete list)

| type prefix | event_kind | What's stored |
|---|---|---|
| `intent:` | `observation` | Full user message, intent summary |
| `action:` | `action` | Tool name, args, result, duration, success |
| `file:` | `action` | Path, diff, before/after content hashes → `.locus/snapshots/` |
| `snapshot:` | `fact` | Git HEAD, branch, dirty files, tree hash |
| `decision:` | `decision` | Decision, reasoning, alternatives |
| `error:` | `observation` | Error type, message, file, resolution |
| `llm:` | `action` | Model, tokens, duration, stop reason |
| `feedback:` | `feedback` | Type, message, sentiment |

---

## Step 5 — Turn End (Update Anchor with Summary)

At turn end, **overwrite the turn anchor** with the full summary. Same `context_id` = auto-override in LocusGraph.

```rust
let turn_ctx = format!("turn:{}_{}", session_id, turn_id);

// Overwrite turn anchor with full summary
locus_graph.store_event(Event {
    context_id: turn_ctx.clone(),
    event_kind: "observation",
    source: "agent",
    payload: json!({
        "turn_id":        turn_id,
        "sequence":       turn_sequence,
        "started_at":     started_at,
        "completed_at":   now_iso8601(),
        "status":         "completed",
        "title":          turn_summary_title,
        "user_request":   user_message_summary,
        "actions_taken":  ["read src/auth/jwt.rs", "edit src/auth/jwt.rs"],
        "outcome":        "Added validation check before token refresh. Tests pass.",
        "decisions":      ["Validate token before refresh"],
        "files_read":     ["src/auth/jwt.rs", "src/middleware/auth.rs"],
        "files_modified": ["src/auth/jwt.rs"],
        "event_count":    total_events_in_turn,
        "tokens_used":    total_tokens,
    }),
    extends: vec![session_ctx.clone()],
});

// Incrementally update session running totals
locus_graph.store_event(Event {
    context_id: session_ctx.clone(),
    event_kind: "fact",
    source: "system",
    payload: json!({
        "status":     "active",
        "turn_count": turn_sequence,
        "last_turn":  turn_ctx,
        "totals": {
            "events":            running.events + turn_events,
            "tool_calls":        running.tool_calls + turn_tool_calls,
            "llm_calls":         running.llm_calls + turn_llm_calls,
            "prompt_tokens":     running.prompt_tokens + turn_prompt_tokens,
            "completion_tokens": running.completion_tokens + turn_completion_tokens,
            "files_modified":    running.files_modified.union(turn_files),
            "errors":            running.errors + turn_errors,
            "errors_resolved":   running.errors_resolved + turn_resolved
        }
    }),
    extends: vec![format!("{}:sessions", repo_hash)],
});
```

---

## Step 6 — Session End

When session closes, finalize the session fact. No need to recompute — running totals are already up to date:

```rust
locus_graph.store_event(Event {
    context_id: session_ctx.clone(),
    event_kind: "fact",
    source: "system",
    payload: json!({
        "status":     "closed",
        "ended_at":   now_iso8601(),
        "turn_count": total_turns,
        "summary":    "Fixed JWT refresh validation bug. Added tests.",
        "totals":     running_totals,   // already accumulated
    }),
    extends: vec![format!("{}:sessions", repo_hash)],
});

// Clear active session
locus_graph.store_event(Event {
    context_id: format!("{}:sessions", repo_hash),
    event_kind: "fact",
    source: "system",
    payload: json!({
        "active_session": null,
        "total_sessions": total_sessions
    }),
    extends: vec![format!("knowledge:{}_{}", project_name, repo_hash)],
});
```

---

## Event Graph

Every `context_id` is strictly `{type}:{name}`:

```
knowledge:locuscodes_abc123                       ← project root
  ├── abc123:tools                                ← tools (see tools.md)
  │     ├── tools:bash
  │     └── ...
  └── abc123:sessions                             ← sessions master
        ├── session:fix-jwt-refresh_a1b2c3d4      ← session 1
        │     ├── turn:a1b2c3d4_001               ← turn 1 (anchor → summary)
        │     │     ├── snapshot:a1b2c3d4_001_001  ←   codebase state at start
        │     │     ├── intent:a1b2c3d4_001_002   ←   user message
        │     │     ├── action:a1b2c3d4_001_003   ←   read src/auth/jwt.rs
        │     │     ├── action:a1b2c3d4_001_004   ←   read src/middleware/auth.rs
        │     │     ├── decision:a1b2c3d4_001_005 ←   check refresh logic
        │     │     └── llm:a1b2c3d4_001_006      ←   LLM call
        │     ├── turn:a1b2c3d4_002               ← turn 2
        │     │     ├── snapshot:a1b2c3d4_002_001  ←   codebase state
        │     │     ├── intent:a1b2c3d4_002_002
        │     │     ├── action:a1b2c3d4_002_003   ←   edit src/auth/jwt.rs
        │     │     ├── file:a1b2c3d4_002_004     ←   diff + before/after hashes
        │     │     ├── decision:a1b2c3d4_002_005
        │     │     ├── llm:a1b2c3d4_002_006
        │     │     └── feedback:a1b2c3d4_002_007 ←   user: "looks good"
        │     └── turn:a1b2c3d4_003               ← turn 3
        │           ├── snapshot:a1b2c3d4_003_001  ←   codebase state
        │           ├── intent:a1b2c3d4_003_002
        │           ├── action:a1b2c3d4_003_003   ←   cargo test
        │           ├── error:a1b2c3d4_003_004    ←   compile error
        │           ├── action:a1b2c3d4_003_005   ←   edit fix
        │           ├── file:a1b2c3d4_003_006     ←   diff + hashes
        │           ├── action:a1b2c3d4_003_007   ←   cargo test (pass)
        │           └── llm:a1b2c3d4_003_008
        ├── session:add-mcp-support_e5f6g7h8      ← session 2
        │     └── ...
        └── session:refactor-ui_i9j0k1l2          ← session 3
              └── ...
```

### Cross-references (related_to)

```
file:a1b2c3d4_002_004     → related_to: ["editor"]        ← file edit tracker
decision:a1b2c3d4_002_005 → related_to: ["decisions"]     ← decision tracker
error:a1b2c3d4_003_004    → related_to: ["errors"]         ← error tracker
```

Query **both ways**:
- **By type**: filter `context_type: "action"` → all tool calls across all sessions
- **By turn**: extends `turn:a1b2c3d4_002` → all events in turn 2
- **By category**: `related_to: "errors"` → all errors across project
- **Semantic**: `retrieve_memories("JWT validation")` → surfaces relevant summaries + events

---

## Context Retrieval Strategies

Two modes: **summary mode** (for LLM context window) and **look-back mode** (for full detail).

### Summary Mode — building the LLM context

| Situation | What to retrieve |
|---|---|
| **New turn (same session)** | Session fact + last 3-5 turn anchors (type: `turn`) + semantic search |
| **Session resume** | Session fact + last turn anchor + semantic search |
| **New session (same project)** | Project knowledge + semantic search across all sessions |

### Look-Back Mode — drilling into full history

```rust
// "What exactly did we change in jwt.rs?"

// 1. Find relevant turn summaries
let turns = retrieve_memories("JWT validation fix", None).await?;
// → returns turn:a1b2c3d4_002

// 2. Get all events in that turn — filter by extends
let timeline = retrieve_memories(
    "turn 002 events",
    Some(RetrieveOptions::new()
        .extends("turn:a1b2c3d4_002")
        .limit(50))
).await?;
// → all events in order (intent, actions, file diffs, decisions)

// 3. Or filter by type across ALL turns
let all_decisions = retrieve_memories(
    "authentication decisions",
    Some(RetrieveOptions::new()
        .context_type("decision")
        .limit(20))
).await?;
// → every decision ever made about auth, across all sessions

// 4. Or filter file changes
let file_history = retrieve_memories(
    "src/auth/jwt.rs",
    Some(RetrieveOptions::new()
        .context_type("file")
        .limit(20))
).await?;
// → every diff ever made to jwt.rs
```

### Query Patterns Cheat Sheet

| Want to know | Query approach |
|---|---|
| What happened in turn 2? | `extends: "turn:a1b2c3d4_002"` |
| All decisions ever? | `context_type: "decision"` |
| All errors across sessions? | `context_type: "error"` or `related_to: "errors"` |
| What changed in `jwt.rs`? | semantic `"jwt.rs"` + `context_type: "file"` |
| Codebase state at turn 2? | `context_type: "snapshot"` + extends `turn:a1b2c3d4_002` |
| Exact file content at a point? | Get `file:` event → `after_hash` → read `.locus/snapshots/{hash}` |
| Full session history? | Session fact → extends chain → turns → extends chain → events |
| Total cost of a session? | Session fact → `totals.prompt_tokens` + `totals.completion_tokens` |

---

## When to Run

| Condition | Action |
|---|---|
| Cold start, `{repo_hash}:sessions` missing | Step 1 (create sessions master) |
| User starts interaction | Step 2 (create or resume session) |
| User sends a message | Step 3 (create anchor + retrieve) → Step 4 (store events) → Step 5 (update anchor) |
| User quits / timeout | Step 6 (close session) |
| Session exceeds turn limit (e.g., 50) | Step 6 (close) → Step 2 (new session with `extends` to old) |

---

## Hooks to Add

Following the pattern in `plan.md`, these hooks belong in `src/hooks.rs`:

| Hook | Purpose | context_id |
|---|---|---|
| `store_session_start()` | Create session event | `session:{slug}_{id}` |
| `store_session_end()` | Close session, finalize totals | `session:{slug}_{id}` |
| `store_turn_start()` | Create turn anchor at START | `turn:{sid}_{tid}` |
| `store_turn_end()` | Update turn anchor with summary at END | `turn:{sid}_{tid}` |
| `store_turn_event()` | Any event during a turn | `{event_type}:{sid}_{tid}_{seq}` |

```rust
pub const CONTEXT_SESSIONS: &str = "sessions";

/// Create turn anchor at turn START
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
            "turn_id": turn_id,
            "sequence": turn_sequence,
            "started_at": chrono::Utc::now().to_rfc3339(),
            "status": "active",
            "user_message": user_message,
        }),
    )
    .context_id(ctx)
    .extends(vec![session_ctx.to_string()])
    .source("system");

    self.store_event(event).await;
}

/// Store any event during a turn
pub async fn store_turn_event(
    &self,
    event_type: &str,        // "intent", "action", "file", "decision", "error", "llm", "feedback"
    session_id: &str,
    turn_id: &str,
    seq: u32,
    event_kind: EventKind,
    source: &str,
    payload: serde_json::Value,
    related_to: Option<Vec<String>>,
) {
    let ctx = format!("{}:{}_{}_{:03}", event_type, session_id, turn_id, seq);
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

/// Update turn anchor with summary at turn END
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
            "turn_id":        turn_id,
            "sequence":       turn_sequence,
            "status":         "completed",
            "title":          summary.title,
            "user_request":   summary.user_request,
            "actions_taken":  summary.actions_taken,
            "outcome":        summary.outcome,
            "decisions":      summary.decisions,
            "files_read":     summary.files_read,
            "files_modified": summary.files_modified,
            "event_count":    summary.event_count,
        }),
    )
    .context_id(ctx)
    .extends(vec![session_ctx.to_string()])
    .source("agent");

    self.store_event(event).await;
}
```

---

## Notes

- **Strict `{type}:{name}`** — every context_id follows the two-layer format. No multi-segment IDs.
- **Event type IS the context_id type** — `action:`, `decision:`, `file:`, `error:`, `llm:`, `intent:`, `feedback:`. Free cross-cutting queries by filtering on type.
- **Turn anchor at START, summary at END** — anchor created first so events have a real parent to `extend`. Same `context_id` overwritten at end with the full summary.
- **Global seq counter** — single monotonic counter per turn across all event types. `seq` in payload guarantees chronological reconstruction.
- **Running session totals** — session stats updated incrementally at each turn end. Session close just finalizes status, no recomputation.
- **Full preservation, selective retrieval** — everything stored, but LLM context only gets turn summaries. Full timeline retrieved on demand via `extends` traversal.
- **Codebase state preserved** — `snapshot:` events capture git HEAD, branch, dirty files at each turn start. `file:` events store diffs + before/after content hashes. Full file content lives in `.locus/snapshots/` (content-addressable, deduped by hash). You can reconstruct the exact codebase at any turn.
- **Cross-session memory** — semantic search surfaces turn summaries, file diffs, and decisions from any past session automatically.
