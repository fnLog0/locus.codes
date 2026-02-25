# Agent Prompts — locus_graph Session & Turn Hooks

Read `crates/locus_graph/plan.md` first for full context. Each prompt below is one task.

---

## Prompt 1 — CONTEXT_SESSIONS constant

Read `crates/locus_graph/plan.md` Task 1. Add `CONTEXT_SESSIONS` constant to `src/hooks.rs` alongside existing constants (keep alphabetical). Then add it to the re-export in `src/lib.rs` (keep alphabetical). Verify: `cargo check -p locus-graph`

---

## Prompt 2 — TurnSummary type

Read `crates/locus_graph/plan.md` Task 2. Add `TurnSummary` struct to `src/types.rs` after `EventLinks`. Then add `TurnSummary` to the re-export in `src/lib.rs`. Verify: `cargo check -p locus-graph`

---

## Prompt 3 — Fix safe_context_name

Read `crates/locus_graph/plan.md` Task 3. Fix `safe_context_name()` in `src/hooks.rs` to allow hyphens — add `|| c == '-'` to the char filter. This matches what `sanitize_context_id()` in `client.rs` already allows. Verify: `cargo check -p locus-graph`

---

## Prompt 4 — Session hooks

Read `crates/locus_graph/plan.md` Task 4. Add `store_session_start()` and `store_session_end()` to `impl LocusGraphClient` in `src/hooks.rs`. Follow the exact code in the plan. Context_ids: `session:{slug}_{id}` and `{repo_hash}:sessions`. Verify: `cargo check -p locus-graph`

---

## Prompt 5 — Turn hooks

Read `crates/locus_graph/plan.md` Task 5. Add `store_turn_start()`, `store_turn_end()`, and `store_turn_event()` to `impl LocusGraphClient` in `src/hooks.rs`. Follow the exact code in the plan. `store_turn_end` uses the `TurnSummary` type from Task 2. Verify: `cargo check -p locus-graph`

---

## Prompt 6 — Snapshot hook

Read `crates/locus_graph/plan.md` Task 6. Add `store_snapshot()` to `impl LocusGraphClient` in `src/hooks.rs`. Context_id: `snapshot:{session_id}_{turn_id}_{seq}`. Verify: `cargo check -p locus-graph`

---

## Prompt 7 — Bootstrap sessions master

Read `crates/locus_graph/plan.md` Task 7. Add `bootstrap_sessions_master()` to `impl LocusGraphClient` in `src/hooks.rs`. Context_id: `{repo_hash}:sessions`. Verify: `cargo check -p locus-graph`

---

## Prompt 8 — Final verification

Run all three checks. Fix any issues:

```bash
cargo check -p locus-graph
cargo clippy -p locus-graph
cargo test -p locus-graph
```
