# locus_graph — Tool Discovery Plan

Add tool discovery hooks to `locus_graph`. The runtime will use these to register tool schemas as memories and retrieve relevant tools before each LLM call.

**Scope**: Only changes to `crates/locus_graph/`. No runtime, toolbus, or UI changes.

---

## What Exists

- `src/hooks.rs` — convenience methods on `LocusGraphClient` (`store_tool_run`, `store_file_edit`, `store_error`, etc.)
- `src/types.rs` — `CreateEventRequest`, `EventKind`, `RetrieveOptions`, `ContextTypeFilter`
- `src/client.rs` — `retrieve_memories()`, `store_event()`, `generate_insights()`
- `src/lib.rs` — re-exports constants and types
- Constants: `CONTEXT_TERMINAL`, `CONTEXT_EDITOR`, `CONTEXT_USER_INTENT`, `CONTEXT_ERRORS`, `CONTEXT_DECISIONS`

## What to Add

Three things: a new context constant, two new hooks, and tests.

---

## Task 1: Add `CONTEXT_TOOLS` constant

**File**: `src/hooks.rs`

Add alongside existing constants:

```rust
pub const CONTEXT_TOOLS: &str = "tools";
```

**File**: `src/lib.rs`

Add to the re-export:

```rust
pub use hooks::{
    CONTEXT_DECISIONS, CONTEXT_EDITOR, CONTEXT_ERRORS, CONTEXT_TERMINAL, CONTEXT_TOOLS,
    CONTEXT_USER_INTENT,
};
```

---

## Task 2: Add `store_tool_schema()` hook

**File**: `src/hooks.rs`

Add a new method on `LocusGraphClient`. Follow the exact pattern of existing hooks (`store_tool_run`, `store_skill`, etc.).

```rust
/// Register a tool schema as a memory.
///
/// Called at startup for ToolBus tools, and on connect for MCP/ACP tools.
/// Stores the tool's description and schema so `retrieve_memories()` can
/// surface it when the user's intent matches.
///
/// Context ID: `tool:{tool_name}`
pub async fn store_tool_schema(
    &self,
    tool_name: &str,
    description: &str,
    parameters_schema: &serde_json::Value,
    source_type: &str,  // "toolbus", "mcp", "acp"
    tags: Vec<&str>,
) {
    let event = CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "tool_schema",
            "data": {
                "tool": tool_name,
                "description": description,
                "parameters": parameters_schema,
                "source_type": source_type,
                "tags": tags,
            }
        }),
    )
    .context_id(format!("tool:{}", tool_name))
    .related_to(vec![CONTEXT_TOOLS.to_string()])
    .source("system");

    self.store_event(event).await;
}
```

---

## Task 3: Add `store_tool_usage()` hook

**File**: `src/hooks.rs`

This is separate from `store_tool_run()` (which stores execution details). This stores the **intent→tool mapping** for discovery learning.

```rust
/// Store a tool usage pattern for discovery learning.
///
/// Called after a tool is successfully used. Links user intent to tool
/// so future `retrieve_memories()` calls surface this tool for similar intents.
///
/// Context ID: `tool:{tool_name}:usage`
pub async fn store_tool_usage(
    &self,
    tool_name: &str,
    user_intent: &str,
    success: bool,
    duration_ms: u64,
) {
    let event = CreateEventRequest::new(
        if success { EventKind::Action } else { EventKind::Observation },
        json!({
            "kind": "tool_usage",
            "data": {
                "tool": tool_name,
                "intent": user_intent,
                "success": success,
                "duration_ms": duration_ms,
            }
        }),
    )
    .context_id(format!("tool:{}:usage", tool_name))
    .related_to(vec![format!("tool:{}", tool_name)])
    .source("executor");

    self.store_event(event).await;
}
```

---

## Task 4: Add tests

**File**: `tests/hooks.rs` (append to existing test file)

Follow the existing test patterns in that file. Tests need a running LocusGraph gRPC server.

```rust
#[tokio::test]
async fn test_store_tool_schema() {
    let client = common::create_test_client().await;

    client.store_tool_schema(
        "bash",
        "Execute shell commands",
        &json!({"type": "object", "properties": {"command": {"type": "string"}}}),
        "toolbus",
        vec!["core", "exec"],
    ).await;

    // Verify it can be retrieved
    let result = client.retrieve_memories("execute a shell command", None).await.unwrap();
    assert!(result.items_found > 0);
}

#[tokio::test]
async fn test_store_tool_usage() {
    let client = common::create_test_client().await;

    client.store_tool_usage(
        "bash",
        "run cargo test",
        true,
        1500,
    ).await;

    // Verify usage pattern is stored
    let result = client.retrieve_memories("run tests", None).await.unwrap();
    assert!(result.items_found > 0);
}
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/hooks.rs` | Add `CONTEXT_TOOLS`, `store_tool_schema()`, `store_tool_usage()` |
| `src/lib.rs` | Add `CONTEXT_TOOLS` to re-exports |
| `tests/hooks.rs` | Add 2 integration tests |

## Files NOT Changed

- `src/client.rs` — no new client methods needed
- `src/types.rs` — no new types needed
- `src/config.rs` — no config changes
- `src/error.rs` — no new errors

---

## Verify

```bash
cargo check -p locus-graph
cargo test -p locus-graph --test hooks -- --test-threads=1 --nocapture
cargo clippy -p locus-graph
```

---

## Notes for Agent

- Follow the exact code style of existing hooks in `hooks.rs` (doc comments, json! macro, .context_id(), .source())
- `store_tool_schema` and `store_tool_usage` are both fire-and-forget (return nothing meaningful)
- Use `json!({})` from `serde_json` — it's already imported in `hooks.rs`
- Keep `CONTEXT_TOOLS` alphabetically sorted with the other constants
- The re-export in `lib.rs` should stay alphabetically sorted
