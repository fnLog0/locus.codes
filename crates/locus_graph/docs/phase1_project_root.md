# Phase 1 — Project Root Anchor

**Goal:** Migrate from `knowledge:{hash}_{hash}` to `project:{project_name}_{repo_hash}` as the root anchor. Update everything that touches this ID — docs, constants, runtime bootstrap, context_id construction.

This document is the concrete execution checklist for Phase 1. If it conflicts with `implementation_plan.md`, the main implementation plan wins.

**Hierarchy being built:**

```
project:{project_name}_{repo_hash}
```

One node. The root. Everything else will extend from this in later phases.

---

## Reference: How Ordigo Does It

In `/Users/nasimakhtar/Projects/locusgraph/ordigo/ordigo/src/locusgraph/restaurant/`:

**Pattern — 3 files per hierarchy node:**

```
restaurant/
  anchor.ts    — constants, context_id builder function, payload schema
  mutations.ts — store/update functions (calls client.storeEvent)
  queries.ts   — get/list functions (calls client.getContext, listContextsByType)
  index.ts     — re-exports
```

**anchor.ts** defines:
- `context_type = "restaurant"` — the type part of `{type}:{name}`
- `restaurantAnchorContextId(slug)` → `"restaurant:{slug}"` — context_id builder
- Payload schema (zod validation)

**mutations.ts** calls:
- `client.storeEvent({ graph_id, context_id, event_kind: "fact", source: "validator", payload, extends: [...] })`
- After storing the root, cascades to create child anchors (`storeOutletAncherEvent`)

**queries.ts** calls:
- `client.getContext({ graphId, context_id })` — get single context
- `client.listContextsByType(type, graphId, { page, page_size })` — list all of a type
- `client.batchGetContext(ids)` — batch fetch

**Key differences from locus.codes:**
- Ordigo uses REST (`@locusgraph/client`), locus.codes uses gRPC proxy (`locus_proxy`)
- Ordigo only uses `extends` link, locus.codes has all four (`extends`, `related_to`, `reinforces`, `contradicts`)
- Ordigo validates payloads with zod, locus.codes uses Rust types + serde
- Ordigo cascades child anchor creation from parent mutation, locus.codes can do the same

---

## What Exists Now

### Current root anchor

In `runtime/mod.rs` line 105:
```rust
format!("knowledge:{}_{}", repo_hash, repo_hash)
```

This is wrong — it uses `repo_hash` for both project_name AND repo_hash. Should be `project:{project_name}_{repo_hash}`.

### Where it's used

| File | What | Current | New |
|---|---|---|---|
| `runtime/mod.rs:105` | Bootstrap project_anchor arg | `knowledge:{hash}_{hash}` | `project:{project_name}_{hash}` |
| `memory.rs:148` | tools_master extends | Uses `project_anchor` param (passed in) | Same param, just fix caller |
| `memory.rs:180,200` | Tool/meta related_to | Uses `project_anchor` param | Same |
| `hooks.rs:6` | `CONTEXT_TOOLS` constant | `"fact:tools"` | Keep the Phase 2 TODO, but do not treat it as the project-root source of truth |
| `tools.md:17` | Step 1 | `knowledge:{project_name}_{repo_hash}` | `project:{project_name}_{repo_hash}` |
| `tools.md:47,100,143,167,187` | Extends / related_to / event graph / rerun conditions | `knowledge:{project_name}_{repo_hash}` | `project:{project_name}_{repo_hash}` |

**Note:** `implementation_plan.md` mentions `context.md` and later phases mention `dynamic_tools.md`, but neither file exists in the current repo. For this repo, the Phase 1 doc migration is `tools.md` only.

### Available gRPC methods (via locus_proxy)

The `LocusGraphClient` in `crates/locus_graph/src/client.rs` wraps `locus_proxy::LocusProxyClient` and exposes:

| Method | What it does | Used for |
|---|---|---|
| `store_event(CreateEventRequest)` | Store/overwrite an event | Creating project anchor |
| `retrieve_memories(query, options)` | Semantic search | Not needed for Phase 1 |
| `list_context_types(page, page_size)` | List all types in graph | Checking if `project` type exists |
| `list_contexts_by_type(type, page, page_size)` | List contexts of a type | Checking if project anchor exists |
| `search_contexts(query, type, page, page_size)` | Search by name | Finding specific project anchor |
| `generate_insights(task, options)` | Reasoning | Not needed |

**Note:** Unlike Ordigo's REST client which has `getContext(context_id)` for direct lookup, the gRPC proxy doesn't expose a direct get-by-id. Use `search_contexts` with the full context_id, or `list_contexts_by_type("project")` to check existence.

---

## Tasks

### Task 1: Add project_name to Runtime

**File:** `crates/locus_runtime/src/runtime/mod.rs`

The Runtime struct needs `project_name` alongside `repo_hash`. Currently only `repo_hash` exists (line 57).

**Add field:**
```rust
pub struct Runtime {
    // ... existing fields ...
    repo_hash: String,
    project_name: String,  // NEW
}
```

**Derive project_name:** Extract from repo root path's last component. The repo root is `config.repo_root` (a `PathBuf`).

```rust
let project_name = config.repo_root
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("unknown")
    .to_lowercase()
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
    .collect::<String>();
```

**Update all three constructors** (`new`, `new_with_shared`, `new_continuing`) to set `project_name`.

### Task 2: Create project anchor helper

**File:** `crates/locus_runtime/src/memory.rs`

Add a helper function following the Ordigo pattern (anchor.ts → context_id builder):

```rust
/// Build the project root anchor context_id.
/// Format: "project:{project_name}_{repo_hash}"
pub fn project_anchor_id(project_name: &str, repo_hash: &str) -> String {
    format!("project:{}_{}", safe_context_name(project_name), safe_context_name(repo_hash))
}
```

`safe_context_name` already exists in `memory.rs` (line 208). It lowercases and replaces non-alphanumeric chars with `_`.

### Task 3: Create ensure_project_anchor function

**File:** `crates/locus_runtime/src/memory.rs`

Following Ordigo's `storeRestaurantAncherEvent` pattern — check if exists, create if not:

```rust
/// Ensure the project root anchor exists in LocusGraph.
/// Idempotent — same context_id = overwrite.
/// Called at Runtime::new() before anything else.
pub async fn ensure_project_anchor(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
    repo_root: &std::path::Path,
) {
    use locus_graph::{CreateEventRequest, EventKind};

    let anchor_id = project_anchor_id(project_name, repo_hash);

    let event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "project_anchor",
            "data": {
                "project_name": project_name,
                "repo_hash": repo_hash,
                "repo_root": repo_root.to_string_lossy(),
                "created_at": chrono::Utc::now().to_rfc3339(),
            }
        }),
    )
    .context_id(anchor_id)
    .source("validator");

    locus_graph.store_event(event).await;
}
```

**No extends** — this is the root. Nothing above it. (In Phase 7, we'll add `agent:locus` above and make project extend it, but not now.)

**source: "validator"** — matches Ordigo pattern. Machine-verified, system-created.

**event_kind: Fact** — this is a factual statement about the project existing.

### Task 4: Call ensure_project_anchor at startup

**File:** `crates/locus_runtime/src/runtime/mod.rs`

In `Runtime::new()`, after creating `locus_graph` but before `bootstrap_tools`:

```rust
// Ensure project root anchor exists (idempotent)
memory::ensure_project_anchor(
    &locus_graph,
    &project_name,
    &repo_hash,
    &config.repo_root,
).await;
```

### Task 5: Fix bootstrap_tools project_anchor argument

**File:** `crates/locus_runtime/src/runtime/mod.rs`

Current (line 105):
```rust
format!("knowledge:{}_{}", repo_hash, repo_hash)
```

New:
```rust
memory::project_anchor_id(&project_name, &repo_hash)
```

This fixes the double-hash bug AND migrates to the new `project:` format.

### Task 6: Update build_context_ids

**File:** `crates/locus_runtime/src/memory.rs`

Current signature (line 81):
```rust
pub fn build_context_ids(
    repo_hash: &str,
    _session_slug: &str,
    turn_contexts: &[String],
) -> Vec<String>
```

**Add `project_name` parameter:**
```rust
pub fn build_context_ids(
    project_name: &str,
    repo_hash: &str,
    _session_slug: &str,
    turn_contexts: &[String],
) -> Vec<String>
```

Current body builds `format!("{}:sessions", repo_hash)` — this will change to `session_anchor:` in Phase 3. For now, keep it but fix the call sites to pass `project_name`.

**Update all call sites** in `runtime/mod.rs` (lines 112, 152, 186) to pass `project_name`.

### Task 7: Keep hooks.rs aligned with the Phase 1 boundary

**File:** `crates/locus_graph/src/hooks.rs`

Current:
```rust
pub const CONTEXT_TOOLS: &str = "fact:tools";
```

The main implementation plan calls out `hooks.rs` because the old constant contributes to the naming confusion. In the current repo, Phase 1 does **not** migrate tool registry IDs yet, so the constant can stay for now, but it must not be used as the project-root source of truth.

Acceptable Phase 1 outcome:
- keep `project_anchor_id()` as the reusable helper for project-root naming
- leave `CONTEXT_TOOLS` with a clear `Phase 2` TODO for the `tool_anchor:` migration

Do **not** do the full tool registry migration here. That belongs to Phase 2.

### Task 8: Update docs — tools.md Step 1

**File:** `crates/locus_graph/docs/tools.md`

Lines 15-28 — update Step 1:
- Title: "Project Knowledge Anchor" → "Project Root Anchor"
- `knowledge:{project_name}_{repo_hash}` → `project:{project_name}_{repo_hash}`
- Description stays the same

### Task 9: Update docs — tools.md all other references

**File:** `crates/locus_graph/docs/tools.md`

Every `knowledge:{project_name}_{repo_hash}` in extends/related_to throughout the file → `project:{project_name}_{repo_hash}`.

Locations:
- Line 47: Step 2 extends
- Line 100: Step 3/4 related_to
- Line 143: Event graph root
- Lines 167, 187: rerun and version-aware bootstrap sections

### Task 10: Update test

**File:** `crates/locus_runtime/src/memory.rs`

Update `test_build_context_ids` (line 225) to pass `project_name` and verify the new format.

```rust
#[test]
fn test_build_context_ids() {
    let turn_contexts: Vec<String> = vec!["turn:test-session_turn-1".to_string()];
    let ids = build_context_ids("locuscodes", "abc123", "test-session", &turn_contexts);

    assert!(ids.contains(&"abc123:sessions".to_string())); // Phase 3 will change this
    assert!(ids.contains(&"turn:test-session_turn-1".to_string()));
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
# Search for old patterns — should find ZERO hits
grep -r "knowledge:" crates/locus_runtime/src/ crates/locus_graph/src/ crates/locus_graph/docs/
```

Exception: `knowledge_anchor:` is a future pattern (Phase 5) that may appear in `hierarchy.md` — that's fine. But `knowledge:{project_name}` should be gone.

### Runtime test (if environment is set up)

Start the runtime. Check LocusGraph for:
- `project:{project_name}_{repo_hash}` exists
- event_kind is `fact`
- source is `validator`
- payload contains project_name, repo_hash, repo_root

---

## Files Changed (summary)

| File | Changes |
|---|---|
| `crates/locus_runtime/src/runtime/mod.rs` | Add `project_name` field. Update `new()`, `new_with_shared()`, `new_continuing()`. Call `ensure_project_anchor()`. Fix `bootstrap_tools()` arg. Update `build_context_ids()` calls. |
| `crates/locus_runtime/src/memory.rs` | Add `project_anchor_id()`. Add `ensure_project_anchor()`. Update `build_context_ids()` signature. Update test. |
| `crates/locus_graph/src/hooks.rs` | Keep `CONTEXT_TOOLS` clearly marked as a Phase 2 tool-anchor migration, not a Phase 1 project-root identifier. |
| `crates/locus_graph/docs/tools.md` | Update Step 1 + all `knowledge:` references. |

**Do NOT change:**
- `hierarchy.md` — already uses new pattern
- `implicit_links.md` — already uses new pattern
- `implementation_plan.md` — already uses new pattern
- `runtime_flow.md` — already uses new pattern
- any non-existent doc paths referenced by older notes — treat them as stale, not as new work to create in Phase 1

---

## What NOT to do in Phase 1

- Do NOT create `tool_anchor:` — that's Phase 2
- Do NOT create `session_anchor:` — that's Phase 3
- Do NOT change `tools:{name}` to `tool:` — that's Phase 2
- Do NOT change `{hash}:sessions` — that's Phase 3
- Do NOT migrate `CONTEXT_TOOLS` / `{hash}:tools` to `tool_anchor:` — that's Phase 2
- Do NOT add new modules (safety_cache, implicit_engine) — those are Phase 6
- Do NOT cascade child anchor creation from project anchor — each phase adds its own anchor

**Phase 1 is ONLY about the project root.** One node. Get it right. Verify. Move on.
