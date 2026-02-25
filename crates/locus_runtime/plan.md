# locus_runtime — Context Master Integration Plan

Wire LocusGraph session/turn lifecycle into the runtime. The runtime becomes the **orchestrator** that calls `locus_graph` hooks at the right moments — bootstrap, session start/end, turn start/events/end, and context retrieval.

**Scope**: Only changes to `crates/locus_runtime/`. Depends on `locus_graph/plan.md` being completed first.

**Depends on**: `locus-graph` having `CONTEXT_SESSIONS`, `TurnSummary`, `store_session_start`, `store_session_end`, `store_turn_start`, `store_turn_end`, `store_turn_event`, `store_snapshot`, `bootstrap_sessions_master`.

---

## What Exists

- `src/runtime/mod.rs` — `Runtime` struct with `session`, `locus_graph`, `toolbus`, `llm_client`, `event_tx`, `config`, `context_ids`, `active_tools`. Init registers tool schemas in LocusGraph.
- `src/runtime/agent_loop.rs` — `run()`, `process_message()`, `process_tool_results()`, `prepare_llm_call()`
- `src/runtime/llm.rs` — LLM streaming
- `src/runtime/tools.rs` — tool execution, sub-agent tasks
- `src/memory.rs` — `recall_memories()`, `store_*()` fire-and-forget helpers, `CORE_TOOLS`, `get_active_tools()`, `build_context_ids()`
- `src/context/` — prompt building, messages, window, file extraction
- `src/tool_handler.rs` — meta-tool handling (`tool_search`, `tool_explain`)
- `src/config.rs` — `RuntimeConfig`

### Current flow

```
Runtime::new()
  → register tool schemas in LocusGraph (tools.md)
  → cache context_ids + active_tools

run(user_message)
  → process_message()
    → recall_memories(query, context_ids)
    → build_system_prompt()
    → build_generate_request()
    → stream_llm_response()
  → process_tool_results() (loop)
    → handle_tool_call() → store_tool_run()
    → recall_memories() again
    → stream_llm_response()
```

### Target flow

```
Runtime::new()
  → bootstrap (tools.md + context.md Step 1)
  → session_start (context.md Step 2)

run(user_message)
  → turn_start (context.md Step 3 — anchor + snapshot + context retrieval)
  → process_message()
    → store intent event (e:001)
    → recall_memories() using session-aware context
    → stream_llm_response()
    → store llm event
  → process_tool_results() (loop)
    → handle_tool_call() → store action event + file event
    → recall_memories()
    → stream_llm_response()
    → store llm event
  → turn_end (context.md Step 5 — summary + update session totals)

shutdown()
  → session_end (context.md Step 6)
```

---

## Task 1: Add session/turn state to Runtime

**File**: `src/runtime/mod.rs`

Add fields to `Runtime`:

```rust
pub struct Runtime {
    // ... existing fields ...

    /// Session slug for LocusGraph context_id
    session_slug: String,
    /// Session ID for LocusGraph (nanoid, not locus_core SessionId)
    graph_session_id: String,
    /// Session context_id: "session:{slug}_{id}"
    graph_session_ctx: String,
    /// Current turn sequence number (1-based)
    turn_sequence: u32,
    /// Global event seq counter within current turn
    turn_event_seq: std::sync::atomic::AtomicU32,
    /// Repo hash for LocusGraph namespacing
    repo_hash: String,
}
```

Add helper methods:

```rust
impl Runtime {
    /// Get the current turn_id as zero-padded string
    fn turn_id(&self) -> String {
        format!("{:03}", self.turn_sequence)
    }

    /// Get the current turn context_id
    fn turn_ctx(&self) -> String {
        format!("turn:{}_{}", self.graph_session_id, self.turn_id())
    }

    /// Get next event seq and increment counter
    fn next_event_seq(&self) -> u32 {
        self.turn_event_seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }

    /// Reset event seq for new turn
    fn reset_event_seq(&self) {
        self.turn_event_seq.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}
```

---

## Task 2: Add bootstrap to Runtime::new()

**File**: `src/runtime/mod.rs`

In `Runtime::new()`, after tool schema registration, add session bootstrap:

```rust
// --- Bootstrap (tools.md is already done above) ---

// context.md Step 1 — sessions master (idempotent)
let project_anchor = format!("knowledge:{}_{}", safe_slug(&config.project_name), &repo_hash);
{
    let g = Arc::clone(&locus_graph);
    let rh = repo_hash.clone();
    let pa = project_anchor.clone();
    tokio::spawn(async move {
        g.bootstrap_sessions_master(&rh, &pa).await;
    });
}

// context.md Step 2 — session start
let graph_session_id = memory::generate_session_id();
let session_slug = memory::slugify("new-session", 30); // updated after first user message via store_session_start
let graph_session_ctx = format!("session:{}_{}", &session_slug, &graph_session_id);

{
    let g = Arc::clone(&locus_graph);
    let slug = session_slug.clone();
    let sid = graph_session_id.clone();
    let rh = repo_hash.clone();
    tokio::spawn(async move {
        g.store_session_start(&slug, &sid, "New session", &rh).await;
    });
}
```

---

## Task 3: Add `TurnRecorder` to track events within a turn

**File**: `src/turn_recorder.rs` (NEW)

This is a lightweight struct that tracks what happens during a turn for the summary:

```rust
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use locus_graph::{EventKind, LocusGraphClient, TurnSummary};
use serde_json::json;
use tokio::sync::Mutex;

/// Records all events within a turn for the summary at turn end.
pub struct TurnRecorder {
    session_id: String,
    turn_id: String,
    session_ctx: String,
    seq: AtomicU32,
    graph: Arc<LocusGraphClient>,
    // Accumulate for summary
    actions: Mutex<Vec<String>>,
    decisions: Mutex<Vec<String>>,
    files_read: Mutex<Vec<String>>,
    files_modified: Mutex<Vec<String>>,
    errors: Mutex<u32>,
    errors_resolved: Mutex<u32>,
    user_message: Mutex<String>,
    tool_calls: Mutex<u32>,
    llm_calls: Mutex<u32>,
    prompt_tokens: Mutex<u64>,
    completion_tokens: Mutex<u64>,
}

impl TurnRecorder {
    pub fn new(
        session_id: String,
        turn_id: String,
        session_ctx: String,
        graph: Arc<LocusGraphClient>,
    ) -> Self {
        Self {
            session_id, turn_id, session_ctx, graph,
            seq: AtomicU32::new(0),
            actions: Mutex::new(Vec::new()),
            decisions: Mutex::new(Vec::new()),
            files_read: Mutex::new(Vec::new()),
            files_modified: Mutex::new(Vec::new()),
            errors: Mutex::new(0),
            errors_resolved: Mutex::new(0),
            user_message: Mutex::new(String::new()),
            tool_calls: Mutex::new(0),
            llm_calls: Mutex::new(0),
            prompt_tokens: Mutex::new(0),
            completion_tokens: Mutex::new(0),
        }
    }

    fn next_seq(&self) -> u32 {
        self.seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Record user intent (first event after snapshot)
    pub async fn record_intent(&self, message: &str, summary: &str) {
        *self.user_message.lock().await = summary.to_string();
        let seq = self.next_seq();
        self.graph.store_turn_event(
            "intent", &self.session_id, &self.turn_id, seq,
            EventKind::Observation, "user",
            json!({ "kind": "intent", "data": { "message": message, "intent_summary": summary, "seq": seq } }),
            None,
        ).await;
    }

    /// Record a tool call
    pub async fn record_action(&self, tool: &str, args: &serde_json::Value, result: &serde_json::Value, duration_ms: u64, success: bool) {
        let seq = self.next_seq();
        self.actions.lock().await.push(format!("{} {}", tool, truncate(args)));
        *self.tool_calls.lock().await += 1;
        self.graph.store_turn_event(
            "action", &self.session_id, &self.turn_id, seq,
            EventKind::Action, "executor",
            json!({ "kind": "tool_call", "data": { "tool": tool, "args": args, "result_preview": result, "duration_ms": duration_ms, "success": success, "seq": seq } }),
            None,
        ).await;
    }

    /// Record a file change with content hashes
    pub async fn record_file_change(&self, path: &str, operation: &str, diff: &str, before_hash: &str, after_hash: &str) {
        let seq = self.next_seq();
        self.files_modified.lock().await.push(path.to_string());
        self.graph.store_turn_event(
            "file", &self.session_id, &self.turn_id, seq,
            EventKind::Action, "executor",
            json!({ "kind": "file_change", "data": { "path": path, "operation": operation, "diff": diff, "before_hash": before_hash, "after_hash": after_hash, "seq": seq } }),
            Some(vec!["action:editor".to_string()]),
        ).await;
    }

    /// Record a decision
    pub async fn record_decision(&self, decision: &str, reasoning: &str) {
        let seq = self.next_seq();
        self.decisions.lock().await.push(decision.to_string());
        self.graph.store_turn_event(
            "decision", &self.session_id, &self.turn_id, seq,
            EventKind::Decision, "agent",
            json!({ "kind": "decision", "data": { "decision": decision, "reasoning": reasoning, "seq": seq } }),
            Some(vec!["decision:decisions".to_string()]),
        ).await;
    }

    /// Record an error
    pub async fn record_error(&self, error_type: &str, message: &str, resolved: bool) {
        let seq = self.next_seq();
        *self.errors.lock().await += 1;
        if resolved { *self.errors_resolved.lock().await += 1; }
        self.graph.store_turn_event(
            "error", &self.session_id, &self.turn_id, seq,
            EventKind::Observation, "system",
            json!({ "kind": "error", "data": { "error_type": error_type, "message": message, "resolved": resolved, "seq": seq } }),
            Some(vec!["observation:errors".to_string()]),
        ).await;
    }

    /// Record an LLM call
    pub async fn record_llm_call(&self, model: &str, prompt_tokens: u64, completion_tokens: u64, duration_ms: u64) {
        let seq = self.next_seq();
        *self.llm_calls.lock().await += 1;
        *self.prompt_tokens.lock().await += prompt_tokens;
        *self.completion_tokens.lock().await += completion_tokens;
        self.graph.store_turn_event(
            "llm", &self.session_id, &self.turn_id, seq,
            EventKind::Action, "system",
            json!({ "kind": "llm_call", "data": { "model": model, "prompt_tokens": prompt_tokens, "completion_tokens": completion_tokens, "duration_ms": duration_ms, "seq": seq } }),
            None,
        ).await;
    }

    /// Record user feedback
    pub async fn record_feedback(&self, feedback_type: &str, message: &str) {
        let seq = self.next_seq();
        self.graph.store_turn_event(
            "feedback", &self.session_id, &self.turn_id, seq,
            EventKind::Feedback, "user",
            json!({ "kind": "feedback", "data": { "type": feedback_type, "message": message, "seq": seq } }),
            None,
        ).await;
    }

    /// Build the TurnSummary from accumulated data
    pub async fn build_summary(&self, title: &str, outcome: &str) -> TurnSummary {
        TurnSummary {
            title: title.to_string(),
            user_request: self.user_message.lock().await.clone(),
            actions_taken: self.actions.lock().await.clone(),
            outcome: outcome.to_string(),
            decisions: self.decisions.lock().await.clone(),
            files_read: self.files_read.lock().await.clone(),
            files_modified: self.files_modified.lock().await.clone(),
            event_count: self.seq.load(Ordering::SeqCst),
        }
    }

    /// Get running totals for session update
    pub async fn running_totals(&self) -> serde_json::Value {
        json!({
            "events": self.seq.load(Ordering::SeqCst),
            "tool_calls": *self.tool_calls.lock().await,
            "llm_calls": *self.llm_calls.lock().await,
            "prompt_tokens": *self.prompt_tokens.lock().await,
            "completion_tokens": *self.completion_tokens.lock().await,
            "files_modified": *self.files_modified.lock().await,
            "errors": *self.errors.lock().await,
            "errors_resolved": *self.errors_resolved.lock().await,
        })
    }
}

fn truncate(v: &serde_json::Value) -> String {
    let s = v.to_string();
    if s.len() > 100 { format!("{}...", &s[..97]) } else { s }
}
```

---

## Task 4: Add `snapshot` module for git state capture

**File**: `src/snapshot.rs` (NEW)

```rust
use std::path::Path;
use std::process::Command;

pub struct GitSnapshot {
    pub head: String,
    pub branch: String,
    pub dirty: Vec<String>,
    pub staged: Vec<String>,
}

/// Capture current git state. Returns None if not a git repo.
pub fn capture_git_state(repo_root: &Path) -> Option<GitSnapshot> {
    let head = run_git(repo_root, &["rev-parse", "HEAD"])?;
    let branch = run_git(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| "detached".to_string());
    let dirty = run_git(repo_root, &["diff", "--name-only"])
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    let staged = run_git(repo_root, &["diff", "--cached", "--name-only"])
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default();

    Some(GitSnapshot { head, branch, dirty, staged })
}

fn run_git(repo_root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// --- File content snapshot storage (.locus/snapshots/) ---

use std::io::Write;
use sha2::{Sha256, Digest};

/// Compute SHA-256 hash of content (first 12 hex chars)
pub fn content_hash(content: &[u8]) -> String {
    let hash = Sha256::digest(content);
    format!("sha256_{}", hex::encode(&hash[..6]))  // 12 hex chars
}

/// Save file content to `.locus/snapshots/{hash}` (content-addressable, deduped)
/// Returns the hash string. Does nothing if hash already exists.
pub fn save_snapshot(repo_root: &Path, content: &[u8]) -> std::io::Result<String> {
    let hash = content_hash(content);
    let dir = repo_root.join(".locus").join("snapshots");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&hash);
    if !path.exists() {
        let mut f = std::fs::File::create(&path)?;
        f.write_all(content)?;
    }
    Ok(hash)
}

/// Read file content from `.locus/snapshots/{hash}`
pub fn load_snapshot(repo_root: &Path, hash: &str) -> std::io::Result<Vec<u8>> {
    let path = repo_root.join(".locus").join("snapshots").join(hash);
    std::fs::read(&path)
}
```

Add `sha2` and `hex` to `Cargo.toml`:
```toml
sha2 = "0.10"
hex = "0.4"
```

---

## Task 5: Integrate turn lifecycle into agent_loop

**File**: `src/runtime/agent_loop.rs`

### In `run()` — wrap the agent loop with turn start/end:

```rust
pub async fn run(&mut self, user_message: &str) -> Result<(), RuntimeError> {
    // --- Turn Start (context.md Step 3) ---
    self.turn_sequence += 1;
    self.reset_event_seq();

    let turn_id = self.turn_id();
    let recorder = Arc::new(TurnRecorder::new(
        self.graph_session_id.clone(),
        turn_id.clone(),
        self.graph_session_ctx.clone(),
        Arc::clone(&self.locus_graph),
    ));

    // Create turn anchor
    self.locus_graph.store_turn_start(
        &self.graph_session_id,
        &self.graph_session_ctx,
        self.turn_sequence,
        user_message,
    ).await;

    // Capture codebase snapshot (seq 001)
    if let Some(snap) = snapshot::capture_git_state(&self.config.repo_root) {
        self.locus_graph.store_snapshot(
            &self.graph_session_id, &turn_id, 1,
            &snap.head, &snap.branch, snap.dirty, snap.staged, "turn_start",
        ).await;
    }

    // Record user intent (seq 002)
    let intent_summary = summarize_intent(user_message);
    recorder.record_intent(user_message, &intent_summary).await;

    // --- Existing agent loop (pass recorder to tool handler) ---
    // process_message() and process_tool_results() use recorder
    // to record actions, decisions, errors, llm calls

    // ... existing loop code, but now recording events via recorder ...

    // --- Turn End (context.md Step 5) ---
    let summary = recorder.build_summary(&intent_summary, &outcome).await;
    self.locus_graph.store_turn_end(
        &self.graph_session_id,
        &self.graph_session_ctx,
        self.turn_sequence,
        summary,
    ).await;

    // Update session running totals
    let totals = recorder.running_totals().await;
    // ... update session event with accumulated totals ...

    Ok(())
}
```

### In `process_tool_results()` — record each tool action:

```rust
// After successful tool call:
recorder.record_action(
    &tool.name, &tool.args, &result.output, result.duration_ms, !result.is_error
).await;

// If tool was edit_file or create_file, also record file change with content snapshots:
if tool.name == "edit_file" || tool.name == "create_file" {
    let path = tool.args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let full_path = repo_root.join(path);
    // before_hash was captured BEFORE the tool call (read file, save_snapshot)
    // after_hash: read the file NOW (after edit), save_snapshot
    let after_content = std::fs::read(&full_path).unwrap_or_default();
    let after_hash = snapshot::save_snapshot(&repo_root, &after_content).unwrap_or_default();
    recorder.record_file_change(path, &tool.name, &diff, &before_hash, &after_hash).await;
}
```

### In LLM response handling — record LLM calls:

```rust
// After stream completes:
recorder.record_llm_call(&model, prompt_tokens, completion_tokens, duration_ms).await;
```

---

## Task 6: Integrate session end into shutdown

**File**: `src/runtime/mod.rs`

In `shutdown()`:

```rust
pub async fn shutdown(&mut self) -> Result<(), RuntimeError> {
    info!("Shutting down runtime");

    // context.md Step 6 — close session
    self.locus_graph.store_session_end(
        &self.session_slug,
        &self.graph_session_id,
        &self.current_task(),
        self.turn_sequence,
        self.session_totals.clone(), // accumulated from all turns
    ).await;

    self.session.set_status(SessionStatus::Completed);
    let _ = self.event_tx.send(SessionEvent::status("Session ended")).await;

    Ok(())
}
```

---

## Task 7: Update context retrieval for session-aware memory

**File**: `src/memory.rs`

Update `build_context_ids()` to include session and turn context:

```rust
pub fn build_context_ids(
    repo_hash: &str,
    session_id: &locus_core::SessionId,
    graph_session_ctx: Option<&str>,
    turn_ctx: Option<&str>,
) -> Vec<String> {
    let mut ids = vec![
        format!("project:{}", repo_hash),
        CONTEXT_DECISIONS.to_string(),
        CONTEXT_ERRORS.to_string(),
        CONTEXT_USER_INTENT.to_string(),
        CONTEXT_TOOLS.to_string(),
        CONTEXT_SESSIONS.to_string(),
        format!("session:{}", session_id.as_str()),
    ];
    if let Some(ctx) = graph_session_ctx {
        ids.push(ctx.to_string());
    }
    if let Some(ctx) = turn_ctx {
        ids.push(ctx.to_string());
    }
    ids
}
```

---

## Task 8: Add session_id and slugify helpers

**File**: `src/memory.rs` (or new `src/utils.rs`)

Uses `uuid` (already a workspace dependency in `Cargo.toml`) — no collisions possible:

```rust
/// Generate a short unique session ID (first 8 chars of UUID v4)
pub fn generate_session_id() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

/// Convert a string to a kebab-case slug, max length.
/// Fallback to "session-{id}" if input is too short/vague.
pub fn slugify(s: &str, max_len: usize) -> String {
    let slug: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.len() < 4 {
        return slug; // caller should fallback to "session-{id}"
    }

    if slug.len() > max_len {
        slug[..max_len].trim_end_matches('-').to_string()
    } else {
        slug
    }
}
```

Ensure `uuid` is in `Cargo.toml` dependencies (it's already a workspace dep):
```toml
uuid = { workspace = true }
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/runtime/mod.rs` | Add session/turn state fields, bootstrap in `new()`, session end in `shutdown()` |
| `src/runtime/agent_loop.rs` | Wrap agent loop with turn start/end, pass `TurnRecorder` |
| `src/runtime/tools.rs` | Record action/file events via `TurnRecorder` |
| `src/runtime/llm.rs` | Record LLM call events via `TurnRecorder` |
| `src/memory.rs` | Update `build_context_ids()` for session awareness, add `generate_session_id()`, `slugify()` |
| `src/turn_recorder.rs` | **NEW** — `TurnRecorder` struct |
| `src/snapshot.rs` | **NEW** — git state capture + file content snapshots (`.locus/snapshots/`) |
| `src/lib.rs` | Add `pub mod turn_recorder; pub mod snapshot;` |
| `Cargo.toml` | Add `sha2 = "0.10"`, `hex = "0.4"` deps |

## Files NOT Changed

- `src/config.rs` — no config changes needed
- `src/error.rs` — no new error types
- `src/context/` — context building stays the same (memories are still injected the same way)
- `src/tool_handler.rs` — meta-tool handling unchanged

---

## Verify

```bash
cargo check -p locus-runtime
cargo test -p locus-runtime
cargo clippy -p locus-runtime
```

---

## Execution Order

1. **First**: Complete `locus_graph/plan.md` (Tasks 1-7)
2. **Then**: This plan in order:
   - Task 8 (helpers — no deps)
   - Task 4 (snapshot — no deps)
   - Task 3 (TurnRecorder — depends on locus_graph hooks)
   - Task 1 (Runtime state — depends on helpers)
   - Task 2 (bootstrap — depends on state)
   - Task 7 (context retrieval — depends on state)
   - Task 5 (agent_loop integration — depends on everything above)
   - Task 6 (shutdown — depends on state)

---

## Future: Dynamic Tools (MCP/ACP)

This plan supports `dynamic_tools.md` without changes:

- **MCP server connect**: Call `store_tool_schema()` for each discovered tool (already exists in hooks)
- **MCP tool call**: `TurnRecorder.record_action()` works for any tool — just pass `"mcp_tool_name"` as the tool name
- **MCP disconnect**: Call `store_event()` with `contradicts` (already supported in client)
- **ACP**: Same pattern, different event_type prefix

No new hooks or types needed. The `store_turn_event()` hook with `event_type: "action"` handles MCP/ACP tool calls the same as ToolBus tools.

---

## Notes

- `TurnRecorder` is the central coordination point — all event recording goes through it. This keeps seq counters consistent and accumulates data for the summary.
- The `TurnRecorder` is `Arc<TurnRecorder>` so it can be shared between the agent loop, tool handler, and LLM handler without ownership issues.
- All `record_*` methods are `async` — they call `store_turn_event()` which is fire-and-forget (non-blocking).
- The snapshot module shells out to `git` — this is intentional. ToolBus's bash tool has the same pattern. No git library dependency needed.
- Session slug is set to `"new-session"` initially and updated after the first user message. The context_id uses the initial slug (it's just an identifier, not a live description).
