# locus_runtime — Tool Discovery Integration Plan

Wire LocusGraph tool discovery into the runtime. Instead of loading ALL tools into every LLM call, use two tiers: always-available core tools + on-demand discovery via `tool_search`/`tool_explain` meta-tools.

**Scope**: Only changes to `crates/locus_runtime/`. Depends on `locus_graph` having `CONTEXT_TOOLS`, `store_tool_schema()`, and `store_tool_usage()` (see `crates/locus_graph/plan.md`).

---

## What Exists

The bottleneck is in two places — both do `self.toolbus.list_tools()` which loads ALL tools:

- `runtime.rs:298` in `process_message()` — `let tools = self.toolbus.list_tools();`
- `runtime.rs:213` in `process_tool_results()` — `let tools = self.toolbus.list_tools();`

These feed into:
- `context.rs:20` — `build_system_prompt(&tools)` — puts all tool descriptions in system prompt text
- `context.rs:123` — `build_generate_request(..., &tools, ...)` — converts all tools to LLM function definitions

Currently 7 ToolBus tools (~1,400 tokens). Works fine now. Breaks at 50+ tools.

### Current flow

```
process_message() / process_tool_results()
  → toolbus.list_tools()                    // ALL tools
  → build_system_prompt(&all_tools)         // ALL in system prompt
  → build_generate_request(..., &all_tools) // ALL as function defs
  → stream_llm_response()
```

### Target flow

```
process_message() / process_tool_results()
  → get_active_tools(&user_message)         // core tools + tool_search + tool_explain
  → build_system_prompt(&active_tools)      // only 10-12 tools
  → build_generate_request(..., &active_tools)
  → stream_llm_response()
  → if LLM calls tool_search → return matching tool summaries
  → if LLM calls tool_explain → return full schema
  → store_tool_usage() after successful execution
```

---

## Task 1: Add `tool_token_budget` to `RuntimeConfig`

**File**: `src/config.rs`

Add field to `RuntimeConfig`:

```rust
/// Maximum tokens to spend on tool schemas per LLM call
pub tool_token_budget: u32,
```

Default: `3800`. Add builder method:

```rust
pub fn with_tool_token_budget(mut self, budget: u32) -> Self {
    self.tool_token_budget = budget;
    self
}
```

Add env var support in `from_env()`:

```rust
if let Ok(budget) = std::env::var("LOCUS_TOOL_BUDGET") {
    if let Ok(val) = budget.parse::<u32>() {
        config.tool_token_budget = val;
    }
}
```

---

## Task 2: Add `CORE_TOOLS` constant and `get_active_tools()` to `memory.rs`

**File**: `src/memory.rs`

These are the tools always included in every LLM call (Tier 0):

```rust
/// Tools always available in every LLM call.
/// These are cheap, universally useful, and don't need discovery.
pub const CORE_TOOLS: &[&str] = &[
    "bash",
    "edit_file",
    "create_file",
    "undo_edit",
    "glob",
    "grep",
    "finder",
    "tool_search",
    "tool_explain",
];
```

Add function to filter tools:

```rust
use locus_toolbus::ToolInfo;

/// Get the active tool list for an LLM call.
///
/// Returns core tools (always available) filtered from the full tool list.
/// In the future, this will also include LocusGraph-promoted hot tools.
pub fn get_active_tools(all_tools: &[ToolInfo]) -> Vec<ToolInfo> {
    all_tools
        .iter()
        .filter(|t| CORE_TOOLS.contains(&t.name.as_str()))
        .cloned()
        .collect()
}
```

Also add `CONTEXT_TOOLS` to `build_context_ids()`:

```rust
pub fn build_context_ids(repo_hash: &str, session_id: &SessionId) -> Vec<String> {
    vec![
        format!("project:{}", repo_hash),
        CONTEXT_DECISIONS.to_string(),
        CONTEXT_ERRORS.to_string(),
        CONTEXT_USER_INTENT.to_string(),
        locus_graph::CONTEXT_TOOLS.to_string(),  // NEW
        format!("session:{}", session_id.as_str()),
    ]
}
```

---

## Task 3: Implement `tool_search` and `tool_explain` as ToolBus tools

**File**: `src/tool_handler.rs`

These are NOT real ToolBus tools (they don't touch the filesystem). They are handled directly in the runtime before reaching ToolBus. Add handling at the top of `handle_tool_call()`:

```rust
pub async fn handle_tool_call(
    tool: ToolUse,
    toolbus: &Arc<ToolBus>,
    locus_graph: Arc<LocusGraphClient>,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<ToolResultData, RuntimeError> {
    // Handle meta-tools directly (don't go through ToolBus)
    match tool.name.as_str() {
        "tool_search" => return handle_tool_search(&tool, &locus_graph, event_tx).await,
        "tool_explain" => return handle_tool_explain(&tool, toolbus, event_tx).await,
        _ => {} // fall through to ToolBus
    }

    // ... existing ToolBus handling below ...
}
```

#### `handle_tool_search`

```rust
async fn handle_tool_search(
    tool: &ToolUse,
    locus_graph: &LocusGraphClient,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<ToolResultData, RuntimeError> {
    let start = Instant::now();

    let query = tool.args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let max_results = tool.args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5);

    let _ = event_tx.send(SessionEvent::tool_start(tool.clone())).await;

    let options = locus_graph::RetrieveOptions::new()
        .limit(max_results)
        .context_type("fact", locus_graph::ContextTypeFilter::new().name("tool"));

    let result = locus_graph.retrieve_memories(query, Some(options))
        .await
        .unwrap_or_default();

    let duration_ms = start.elapsed().as_millis() as u64;
    let output = serde_json::json!({
        "results": result.memories,
        "items_found": result.items_found,
    });

    let tool_result = ToolResultData::success(output, duration_ms);
    let _ = event_tx.send(SessionEvent::tool_done(tool.id.clone(), tool_result.clone())).await;

    Ok(tool_result)
}
```

#### `handle_tool_explain`

```rust
async fn handle_tool_explain(
    tool: &ToolUse,
    toolbus: &Arc<ToolBus>,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<ToolResultData, RuntimeError> {
    let start = Instant::now();

    let tool_id = tool.args.get("tool_id").and_then(|v| v.as_str()).unwrap_or("");

    let _ = event_tx.send(SessionEvent::tool_start(tool.clone())).await;

    // Look up tool in ToolBus
    let all_tools = toolbus.list_tools();
    let found = all_tools.iter().find(|t| t.name == tool_id);

    let duration_ms = start.elapsed().as_millis() as u64;
    let output = match found {
        Some(t) => serde_json::json!({
            "tool_id": t.name,
            "description": t.description,
            "parameters": t.parameters,
        }),
        None => serde_json::json!({
            "error": format!("Tool '{}' not found", tool_id),
        }),
    };

    let tool_result = ToolResultData::success(output, duration_ms);
    let _ = event_tx.send(SessionEvent::tool_done(tool.id.clone(), tool_result.clone())).await;

    Ok(tool_result)
}
```

---

## Task 4: Add `tool_search` and `tool_explain` to tool definitions

**File**: `src/context.rs`

The LLM needs to know these meta-tools exist. Add them to the tool list in `build_generate_request` or create a helper that appends them:

```rust
/// Meta-tool definitions for tool discovery.
pub fn meta_tool_definitions() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            name: "tool_search".to_string(),
            description: "Search for available tools by describing what you want to do. Returns tool names and summaries.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Describe what you want to do, e.g. 'create a GitHub pull request'"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum results to return (default: 5)"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolInfo {
            name: "tool_explain".to_string(),
            description: "Get the full schema for a specific tool before calling it. Use after tool_search.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "tool_id": {
                        "type": "string",
                        "description": "The tool name from tool_search results"
                    }
                },
                "required": ["tool_id"]
            }),
        },
    ]
}
```

---

## Task 5: Replace `toolbus.list_tools()` with `get_active_tools()`

**File**: `src/runtime.rs`

Two locations. Same change in both.

### In `process_message()` (line ~298):

```rust
// BEFORE
let tools = self.toolbus.list_tools();

// AFTER
let all_tools = self.toolbus.list_tools();
let mut tools = memory::get_active_tools(&all_tools);
tools.extend(context::meta_tool_definitions());
```

### In `process_tool_results()` (line ~213):

Same change:

```rust
// BEFORE
let tools = self.toolbus.list_tools();

// AFTER
let all_tools = self.toolbus.list_tools();
let mut tools = memory::get_active_tools(&all_tools);
tools.extend(context::meta_tool_definitions());
```

---

## Task 6: Store tool usage after successful execution

**File**: `src/tool_handler.rs`

After a successful tool call, store the intent→tool link. Add this after `store_tool_run` in `handle_tool_call()`:

```rust
// Store tool usage for discovery learning (fire-and-forget)
if !tool_result.is_error {
    let graph = Arc::clone(&locus_graph);
    let tool_name = tool.name.clone();
    // Use empty intent for now — runtime will pass user message in future
    tokio::spawn(async move {
        graph.store_tool_usage(&tool_name, "", true, duration_ms).await;
    });
}
```

---

## Task 7: Register tool schemas at startup

**File**: `src/runtime.rs`

In `Runtime::new()`, after ToolBus is created, register all tools in LocusGraph:

```rust
// Register tool schemas in LocusGraph for discovery
let graph_clone = Arc::clone(&Arc::new(locus_graph));
let tools_to_register = toolbus.list_tools();
tokio::spawn(async move {
    for tool in tools_to_register {
        graph_clone.store_tool_schema(
            &tool.name,
            &tool.description,
            &tool.parameters,
            "toolbus",
            vec!["core"],
        ).await;
    }
});
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/config.rs` | Add `tool_token_budget` field, builder, env var |
| `src/memory.rs` | Add `CORE_TOOLS`, `get_active_tools()`, add `CONTEXT_TOOLS` to `build_context_ids` |
| `src/context.rs` | Add `meta_tool_definitions()` |
| `src/tool_handler.rs` | Intercept `tool_search`/`tool_explain` before ToolBus, add `store_tool_usage` call |
| `src/runtime.rs` | Replace `list_tools()` → `get_active_tools()` in 2 places, register tools at startup |

## Files NOT Changed

- `src/lib.rs` — no new public exports needed
- `src/error.rs` — no new error types

---

## Verify

```bash
cargo check -p locus-runtime
cargo test -p locus-runtime
cargo clippy -p locus-runtime
```

---

## Dependencies

This plan depends on `crates/locus_graph/plan.md` being completed first:
- `CONTEXT_TOOLS` constant
- `store_tool_schema()` method
- `store_tool_usage()` method

---

## Notes for Agent

- `tool_search` and `tool_explain` are NOT ToolBus tools — they are intercepted in `tool_handler.rs` before reaching ToolBus
- `get_active_tools()` filters from the full ToolBus list using `CORE_TOOLS` — it returns only tools whose names are in that list
- `meta_tool_definitions()` returns `Vec<ToolInfo>` — same type as `toolbus.list_tools()`
- Both `process_message()` and `process_tool_results()` need the same change — don't miss the second one
- Fire-and-forget pattern: use `tokio::spawn` for non-blocking writes (follow existing patterns in `memory.rs`)
- Keep existing tests passing — the change is additive, not breaking
