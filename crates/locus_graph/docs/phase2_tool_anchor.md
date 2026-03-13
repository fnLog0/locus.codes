# Phase 2 — Tool Anchor + Tools

**Goal:** Migrate from `{repo_hash}:tools` / `tools:{name}` to `tool_anchor:{project_name}_{repo_hash}` / `tool:{name}` as the tool registry anchor. Update bootstrap, tool_search, hooks.rs constant, and docs.

This document is the concrete execution checklist for Phase 2. If it conflicts with `implementation_plan.md`, the main implementation plan wins.

**Hierarchy being built:**

```
project:{project_name}_{repo_hash}            ← Phase 1 (exists)
  └── tool_anchor:{project_name}_{repo_hash}  ← NEW
        ├── tool:bash                          ← renamed from tools:bash
        ├── tool:create_file
        ├── tool:edit_file
        ├── tool:undo_edit
        ├── tool:glob
        ├── tool:grep
        ├── tool:finder
        ├── tool:read
        ├── tool:task_list
        ├── tool:handoff
        ├── tool:web_automation
        ├── meta:tool_search
        ├── meta:tool_explain
        └── meta:task
```

**Depends on:** Phase 1 (project root anchor exists).

---

## What Exists Now

### Current bootstrap — `memory.rs` `bootstrap_tools()`

```rust
// Step 2: Tool registry master event
let tools_master_ctx = format!("{}:tools", safe_context_name(&repo_hash));
// ...extends: [project_anchor]

// Step 3: Individual tool events
let tool_ctx = format!("tools:{}", safe_context_name(&tool.name));
// ...extends: [tools_master_ctx], related_to: [project_anchor]

// Step 4: Meta-tool events
let meta_ctx = format!("meta:{}", safe_context_name(&tool.name));
// ...extends: [tools_master_ctx], related_to: [project_anchor]
```

**Problems:**
1. `{repo_hash}:tools` — the "type" is a hash, not a semantic type. Should be `tool_anchor:`.
2. `tools:bash` — plural `tools:` inconsistent with hierarchy (`tool:bash`). Should be `tool:`.
3. `hooks.rs` constant `CONTEXT_TOOLS = "fact:tools"` — stale, no longer matches any context_id.
4. `tool_search` in `tool_handler.rs` filters by `context_type("fact", name("tool"))` — this searches for type `fact` with name containing `tool`. Should search type `tool` directly.

### Where things are used

| File | What | Current | New |
|---|---|---|---|
| `memory.rs:146` | tools_master context_id | `{hash}:tools` | `tool_anchor:{project_name}_{repo_hash}` |
| `memory.rs:167` | individual tool context_id | `tools:{name}` | `tool:{name}` |
| `memory.rs:177-178` | tool extends/related_to | extends `{hash}:tools`, related_to project | extends `tool_anchor:`, related_to project |
| `memory.rs:186` | meta-tool context_id | `meta:{name}` | `meta:{name}` (unchanged) |
| `memory.rs:196-197` | meta extends/related_to | extends `{hash}:tools`, related_to project | extends `tool_anchor:`, related_to project |
| `hooks.rs:6` | CONTEXT_TOOLS constant | `"fact:tools"` | Remove or replace with helper |
| `tool_handler.rs:144` | tool_search filter | `context_type("fact", name("tool"))` | `context_type("tool", ContextTypeFilter::new())` |
| `memory.rs:7` | import CONTEXT_TOOLS | `use locus_graph::CONTEXT_TOOLS` | Remove import |
| `memory.rs:87` | build_context_ids uses CONTEXT_TOOLS | `CONTEXT_TOOLS.to_string()` | `tool_anchor_id(project_name, repo_hash)` |
| `tools.md` | Steps 2-4, event graph | `{hash}:tools`, `tools:bash` | `tool_anchor:`, `tool:bash` |

---

## Tasks

### Task 1: Add tool_anchor_id helper

**File:** `crates/locus_runtime/src/memory.rs`

Add a helper following the same pattern as `project_anchor_id()`:

```rust
/// Build the tool anchor context_id.
/// Format: "tool_anchor:{project_name}_{repo_hash}"
pub fn tool_anchor_id(project_name: &str, repo_hash: &str) -> String {
    format!(
        "tool_anchor:{}_{}",
        safe_context_name(project_name),
        safe_context_name(repo_hash)
    )
}
```

### Task 2: Rewrite bootstrap_tools to use tool_anchor

**File:** `crates/locus_runtime/src/memory.rs`

Update `bootstrap_tools()`:

**Step 2 — Tool registry master:**
- Old: `format!("{}:tools", safe_context_name(&repo_hash))`
- New: `tool_anchor_id(&project_name, &repo_hash)` — but `bootstrap_tools` receives `project_anchor: String`. It needs `project_name` and `repo_hash` separately now.

**Change signature:**
```rust
pub fn bootstrap_tools(
    locus_graph: std::sync::Arc<LocusGraphClient>,
    project_name: String,     // NEW (was repo_hash)
    repo_hash: String,
    project_anchor: String,
    tools: Vec<ToolInfo>,
    meta_tools: Vec<ToolInfo>,
    locus_version: String,
)
```

**Inside the function:**

```rust
// Step 2: Tool anchor (replaces {hash}:tools)
let tool_anchor = tool_anchor_id(&project_name, &repo_hash);
let master_event = CreateEventRequest::new(
    EventKind::Fact,
    serde_json::json!({
        "kind": "tool_anchor",
        "data": {
            "tool_count": tools.len() + meta_tools.len(),
            "tool_names": tool_names,
            "meta_names": meta_names,
            "locus_version": locus_version,
        }
    }),
)
.context_id(tool_anchor.clone())
.extends(vec![project_anchor.clone()])
.source("validator");

// Step 3: Individual tool events — tool:{name} instead of tools:{name}
let tool_ctx = format!("tool:{}", safe_context_name(&tool.name));
// ...extends: [tool_anchor], related_to: [project_anchor]

// Step 4: Meta-tool events — meta:{name} (unchanged)
let meta_ctx = format!("meta:{}", safe_context_name(&tool.name));
// ...extends: [tool_anchor], related_to: [project_anchor]
```

### Task 3: Update bootstrap_tools call site

**File:** `crates/locus_runtime/src/runtime/mod.rs`

Current call (line ~123):
```rust
memory::bootstrap_tools(
    Arc::clone(&locus_graph),
    repo_hash.clone(),
    memory::project_anchor_id(&project_name, &repo_hash),
    toolbus_tools.clone(),
    meta_tools.clone(),
    locus_constant::app::VERSION.to_string(),
);
```

New call:
```rust
memory::bootstrap_tools(
    Arc::clone(&locus_graph),
    project_name.clone(),
    repo_hash.clone(),
    memory::project_anchor_id(&project_name, &repo_hash),
    toolbus_tools.clone(),
    meta_tools.clone(),
    locus_constant::app::VERSION.to_string(),
);
```

### Task 4: Remove CONTEXT_TOOLS constant from hooks.rs

**File:** `crates/locus_graph/src/hooks.rs`

Current:
```rust
/// TODO(Phase 2): replace this with the `tool_anchor:{project_name}_{repo_hash}` helper.
pub const CONTEXT_TOOLS: &str = "fact:tools";
```

Delete the constant entirely. The `tool_anchor_id()` helper in `memory.rs` replaces it.

If other crates import `CONTEXT_TOOLS` from `locus_graph`, remove those imports too.

### Task 5: Update build_context_ids to use tool_anchor_id

**File:** `crates/locus_runtime/src/memory.rs`

Current (line 87):
```rust
let mut ids = vec![format!("{}:sessions", repo_hash), CONTEXT_TOOLS.to_string()];
```

New:
```rust
let mut ids = vec![
    format!("{}:sessions", repo_hash),  // Phase 3 will change this to session_anchor:
    tool_anchor_id(project_name, repo_hash),
];
```

Remove the `use locus_graph::CONTEXT_TOOLS` import (line 7).

### Task 6: Update handle_tool_search filter

**File:** `crates/locus_runtime/src/tool_handler.rs`

Current (line 144):
```rust
let options = RetrieveOptions::new()
    .limit(max_results)
    .context_type("fact", ContextTypeFilter::new().name("tool"));
```

New:
```rust
let options = RetrieveOptions::new()
    .limit(max_results)
    .context_type("tool", ContextTypeFilter::new())
    .context_type("meta", ContextTypeFilter::new());
```

This searches both `tool:` and `meta:` context types, which matches the new naming.

Also remove unused `ContextTypeFilter` import if the new filter syntax doesn't need it, or keep if still needed.

### Task 7: Update tools.md — Steps 2-4

**File:** `crates/locus_graph/docs/tools.md`

**Step 2 — Tool Registry Master Event:**
- Title: stays "Tool Registry Master Event" or rename to "Tool Anchor"
- `{repo_hash}:tools` → `tool_anchor:{project_name}_{repo_hash}`
- extends stays `project:{project_name}_{repo_hash}`

**Step 3 — Individual Tool Events:**
- `tools:{tool_name}` → `tool:{tool_name}`
- extends: `{repo_hash}:tools` → `tool_anchor:{project_name}_{repo_hash}`
- related_to stays `project:{project_name}_{repo_hash}`

**Step 4 — Meta-Tool Events:**
- `meta:{tool_name}` stays same
- extends: `{repo_hash}:tools` → `tool_anchor:{project_name}_{repo_hash}`
- related_to stays `project:{project_name}_{repo_hash}`

### Task 8: Update tools.md — Event graph

**File:** `crates/locus_graph/docs/tools.md`

Current event graph (line ~148):
```
project:{project_name}_{repo_hash}         ← project root anchor
  └── {repo_hash}:tools                    ← tool registry master
        ├── tools:bash                     ← static (ToolBus)
        ...
```

New:
```
project:{project_name}_{repo_hash}              ← project root anchor
  └── tool_anchor:{project_name}_{repo_hash}    ← tool anchor
        ├── tool:bash                            ← static (ToolBus)
        ├── tool:create_file
        ├── tool:edit_file
        ├── tool:undo_edit
        ├── tool:glob
        ├── tool:grep
        ├── tool:finder
        ├── tool:read
        ├── tool:task_list
        ├── tool:handoff
        ├── tool:web_automation
        ├── meta:tool_search                     ← meta (Runtime)
        ├── meta:tool_explain
        └── meta:task
```

### Task 9: Update tools.md — When to Run / Version-Aware Bootstrap

Update all remaining `{repo_hash}:tools` references to `tool_anchor:{project_name}_{repo_hash}` in:
- "When to Run" table (line ~170)
- "Version-Aware Bootstrap" section (line ~180)

### Task 10: Update tests

**File:** `crates/locus_runtime/src/memory.rs`

Update `test_build_context_ids` — it currently asserts `fact:tools`:

```rust
#[test]
fn test_build_context_ids() {
    let turn_contexts: Vec<String> = vec!["turn:test-session_turn-1".to_string()];
    let ids = build_context_ids("locuscodes", "abc123", "test-session", &turn_contexts);

    assert!(ids.contains(&"abc123:sessions".to_string())); // Phase 3 will change this
    assert!(ids.contains(&"tool_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"turn:test-session_turn-1".to_string()));
}
```

Add a test for `tool_anchor_id`:

```rust
#[test]
fn test_tool_anchor_id() {
    let id = tool_anchor_id("locuscodes", "abc123");
    assert_eq!(id, "tool_anchor:locuscodes_abc123");
}
```

### Task 11: Verify no stale references

After all changes, run:

```bash
grep -rn "fact:tools" crates/locus_runtime/src/ crates/locus_graph/src/ crates/locus_graph/docs/
grep -rn "CONTEXT_TOOLS" crates/locus_runtime/src/ crates/locus_graph/src/
grep -rn '"tools:' crates/locus_runtime/src/ crates/locus_graph/docs/
```

All should return zero hits (except this phase doc itself and `implementation_plan.md` historical references).

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
# Old patterns — should find ZERO hits in source code
grep -rn "CONTEXT_TOOLS" crates/locus_runtime/src/ crates/locus_graph/src/
grep -rn '"fact:tools"' crates/locus_runtime/src/ crates/locus_graph/src/

# Old tool naming — should find ZERO hits
grep -rn '"tools:' crates/locus_runtime/src/

# New patterns — should exist
grep -rn "tool_anchor:" crates/locus_runtime/src/memory.rs
grep -rn '"tool:' crates/locus_runtime/src/memory.rs
```

### Runtime test (if environment is set up)

Start the runtime. Check LocusGraph for:
- `tool_anchor:{project_name}_{repo_hash}` exists, extends `project:{project_name}_{repo_hash}`
- `tool:bash` exists, extends `tool_anchor:`
- `tool_search` meta-tool returns results when querying "file operations"
- Old `{hash}:tools` and `tools:bash` are stale (not recreated — they'll be overridden naturally or ignored)

---

## Files Changed (summary)

| File | Changes |
|---|---|
| `crates/locus_runtime/src/memory.rs` | Add `tool_anchor_id()`. Update `bootstrap_tools()` signature + body. Update `build_context_ids()` to use `tool_anchor_id`. Remove `CONTEXT_TOOLS` import. Update tests. |
| `crates/locus_runtime/src/runtime/mod.rs` | Update `bootstrap_tools()` call site to pass `project_name`. |
| `crates/locus_graph/src/hooks.rs` | Delete `CONTEXT_TOOLS` constant. |
| `crates/locus_runtime/src/tool_handler.rs` | Update `handle_tool_search` context_type filter from `"fact"` to `"tool"` + `"meta"`. |
| `crates/locus_graph/docs/tools.md` | Update Steps 2-4, event graph, When to Run, Version-Aware Bootstrap sections. |

**Do NOT change:**
- `phase1_project_root.md` — historical reference
- `hierarchy.md` — already uses `tool_anchor:` / `tool:` naming
- `implicit_links.md` — no tool registry references
- `runtime_flow.md` — no tool registry references
- `implementation_plan.md` — high-level plan, already correct

---

## What NOT to do in Phase 2

- Do NOT create `session_anchor:` — that's Phase 3
- Do NOT change `{hash}:sessions` in `build_context_ids` — that's Phase 3
- Do NOT implement MCP/ACP anchors — that's Phase 10
- Do NOT add safety cache or implicit engine — that's Phase 6
- Do NOT change how tools are fed to the LLM (via `ToolBus.list_tools()`) — LocusGraph is the discovery layer for `tool_search`, not the primary tool schema source
- Do NOT add new tools to ToolBus

**Phase 2 is ONLY about renaming the tool registry branch.** Same tools, same schemas, correct hierarchy naming. Verify. Move on.
