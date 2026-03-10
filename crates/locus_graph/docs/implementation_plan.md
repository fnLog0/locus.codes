# LocusGraph — Implementation Plan

Vertical slices. Each phase takes one hierarchy branch, implements it end-to-end through the whole runtime stack (hierarchy → storage → retrieval → context feeding → test), then moves to the next.

No horizontal layers. No "update all IDs first, then all caching, then all context." That's untestable. Instead: one branch, fully working, fully tested, then the next.

---

## The Hierarchy (implementation order)

```
agent:locus                                          ← Phase 7
  ├── skill_anchor:locus                             ← Phase 8
  ├── learning_anchor:locus                          ← Phase 9
  │     ├── proficiency_anchor:locus                 ← Phase 9
  │     ├── preference_anchor:locus                  ← Phase 9
  │     ├── pattern_anchor:locus                     ← Phase 9
  │     └── mistake_anchor:locus                     ← Phase 9
  └── project:locuscodes_abc123                      ← Phase 1
        ├── tool_anchor:locuscodes_abc123             ← Phase 2
        │     ├── tool:bash ... tool:read             ← Phase 2
        │     ├── meta:tool_search ...                ← Phase 2
        │     ├── mcp_anchor:locuscodes_abc123        ← Phase 10
        │     └── acp_anchor:locuscodes_abc123        ← Phase 10
        ├── session_anchor:locuscodes_abc123           ← Phase 3
        │     └── session:fix-jwt_a1b2c3d4            ← Phase 3
        │           └── turn:a1b2c3d4_001             ← Phase 4
        │                 ├── snapshot:...             ← Phase 4
        │                 ├── intent:...              ← Phase 4
        │                 ├── action:...              ← Phase 4
        │                 ├── decision:...            ← Phase 4
        │                 ├── error:...               ← Phase 4
        │                 ├── llm:...                 ← Phase 4
        │                 └── feedback:...            ← Phase 4
        ├── knowledge_anchor:locuscodes_abc123         ← Phase 5
        │     ├── fact:...                            ← Phase 5
        │     ├── rule:...                            ← Phase 6
        │     └── constraint:...                      ← Phase 6
        ├── skill_anchor:locuscodes_abc123             ← Phase 8
        └── learning_anchor:locuscodes_abc123          ← Phase 9
```

---

## Phase 1 — Project Root

**Hierarchy branch:**

```
project:locuscodes_abc123                            ← this
```

**What:** The root anchor. Everything extends from this. Without it, nothing has a parent.

**Store:** At cold start, check if `project:{project_name}_{repo_hash}` exists in LocusGraph. If not, create it. Payload: project name, repo root path, created_at.

**Retrieve:** At session start, fetch the project anchor. This confirms the project is known and gives the runtime the project_name and repo_hash to construct all other context_ids.

**Feed to LLM:** Not directly. The project anchor is structural — it's the root for extends chains. But its payload (project name, repo path) goes into the session context block that's already built in `context/messages.rs`.

**Runtime changes:**
- `memory.rs` — update `simple_hash()` if needed, add `ensure_project_anchor()` function that checks/creates the project root. Called at runtime initialization.
- `hooks.rs` — no more `CONTEXT_TOOLS = "fact:tools"`. Replace with a function that constructs the project anchor ID: `project_anchor_id(project_name, repo_hash) → "project:{project_name}_{repo_hash}"`.
- `context.md` — update prerequisites section.
- `tools.md` — update Step 1 from `knowledge:` to `project:`.

**Test:** Start runtime → verify `project:` event exists in LocusGraph. Restart → verify it's not duplicated (same context_id = overwrite). Run with new repo → verify different project anchor created.

**Done when:** `cargo test -p locus-runtime` passes. `project:{name}_{hash}` exists in LocusGraph after cold start.

---

## Phase 2 — Tool Anchor + Tools

**Hierarchy branch:**

```
project:locuscodes_abc123
  └── tool_anchor:locuscodes_abc123                   ← this
        ├── tool:bash                                 ← this
        ├── tool:create_file                          ← this
        ├── tool:edit_file                            ← this
        ├── tool:undo_edit                            ← this
        ├── tool:glob                                 ← this
        ├── tool:grep                                 ← this
        ├── tool:finder                               ← this
        ├── tool:read                                 ← this
        ├── meta:tool_search                          ← this
        ├── meta:tool_explain                         ← this
        └── meta:task                                 ← this
```

**Depends on:** Phase 1 (tool_anchor extends project root).

**What:** Register all ToolBus tools and meta-tools in LocusGraph so the agent can discover them.

**Store:** Update `bootstrap_tools()` in `memory.rs`:
- Create `tool_anchor:{project_name}_{repo_hash}` extending `project:{project_name}_{repo_hash}`
- For each ToolBus tool: create `tool:{tool_name}` extending `tool_anchor:`
- For each meta-tool: create `meta:{tool_name}` extending `tool_anchor:`

**Retrieve:** `handle_tool_search()` in `tool_handler.rs` already queries LocusGraph for tools. Update its `context_type` filter from `"fact"` to `"tool"` (since tool events now use `tool:` as the type). Verify it returns correct results.

**Feed to LLM:** Tool schemas are already fed via `ToolBus.list_tools()` → `format_tools()`. LocusGraph is the discovery layer (for `tool_search`), not the primary tool schema source. No change to context builder yet.

**Runtime changes:**
- `memory.rs` — rewrite `bootstrap_tools()`: change `tools:` → `tool:`, change `{hash}:tools` → `tool_anchor:`, update extends/related_to.
- `tool_handler.rs` — update `handle_tool_search()` context_type filter.
- `tools.md` — update Steps 2-4 and event graph.
- `dynamic_tools.md` — update extends references (but don't implement MCP/ACP yet, just fix the docs).

**Test:** Cold start → verify `tool_anchor:` and all `tool:` events exist in LocusGraph. Call `tool_search` meta-tool with "file operations" → verify it returns edit_file, create_file, etc. Restart → verify bootstrap is idempotent (same IDs = overwrite, no duplicates).

**Done when:** `cargo test -p locus-runtime` passes. `tool_search` returns correct tools. `tools.md` matches the code.

---

## Phase 3 — Session Anchor + Sessions

**Hierarchy branch:**

```
project:locuscodes_abc123
  └── session_anchor:locuscodes_abc123                ← this
        └── session:fix-jwt_a1b2c3d4                  ← this
```

**Depends on:** Phase 1 (session_anchor extends project root).

**What:** Session lifecycle — create, resume, close. The container for all turns.

**Store:**
- At cold start: ensure `session_anchor:{project_name}_{repo_hash}` exists, extending `project:`.
- At session start: create `session:{slug}_{session_id}` extending `session_anchor:`. Payload: title, slug, session_id, started_at, status, turn_count, totals.
- Update `session_anchor:` with `active_session` pointing to the new session.
- At session end: update session with status "closed", ended_at, final totals. Clear active_session in session_anchor.

**Retrieve:**
- At CLI launch: fetch `session_anchor:` → check `active_session`. If set → resume. If null → new session.
- `fetch_session_turns()` already exists in `memory.rs` — update it to use new session context_id format.

**Feed to LLM:** Session metadata (id, turn count, repo name) already goes into context via `build_session_context()` in `messages.rs`. No change needed — just make sure the IDs used internally are the new format.

**Runtime changes:**
- `memory.rs` — update `build_context_ids()`: add `project_name` parameter, change `{hash}:sessions` → `session_anchor:{project_name}_{hash}`. Add `ensure_session_anchor()`.
- `runtime/mod.rs` — update session creation logic to store session event with new extends chain.
- `agent_loop.rs` — update `process_message()` to use new session context_id.
- `context.md` — update Steps 1-2, Session Resume section.

**Test:** Start runtime with new session → verify `session_anchor:` and `session:` events in LocusGraph. Close session → verify status "closed", active_session null. Start again → verify resume detects no active session. Create session, quit without closing, restart → verify resume picks up active session.

**Done when:** Session create/resume/close lifecycle works end-to-end. `cargo test -p locus-runtime` passes.

---

## Phase 4 — Turns + Turn Events

**Hierarchy branch:**

```
session:fix-jwt_a1b2c3d4
  └── turn:a1b2c3d4_001                               ← this
        ├── snapshot:a1b2c3d4_001_001                  ← this
        ├── intent:a1b2c3d4_001_002                    ← this
        ├── action:a1b2c3d4_001_003                    ← this
        ├── decision:a1b2c3d4_001_005                  ← this
        ├── error:a1b2c3d4_003_004                     ← this
        ├── llm:a1b2c3d4_001_006                       ← this
        ├── file:a1b2c3d4_002_004                      ← this
        └── feedback:a1b2c3d4_002_007                  ← this
```

**Depends on:** Phase 3 (turns extend sessions).

**What:** The event timeline. Every action the agent takes becomes an event in LocusGraph. Turn anchors created at start, updated at end with summary.

**Store:**
- Turn start: create `turn:{session_id}_{turn_id}` extending `session:{slug}_{id}`. Minimal payload (started_at, status: active, user_message).
- During turn: for each tool call, LLM call, decision, error → create `{event_type}:{session_id}_{turn_id}_{seq}` extending the turn. Use the global seq counter.
- Turn end: overwrite turn anchor with full summary (same context_id = auto-override).

**This is where cache.db becomes important.** Turns generate 5-15 events each. Writing each one to LocusGraph synchronously would be slow. Instead:
- Add a simple write buffer: events go to cache.db first (or even just a `Vec<CreateEventRequest>` in memory for now).
- At turn end: batch flush to LocusGraph.
- If the full cache.db proxy (Phase 2 from old plan) is too big to build now, start with the simple Vec buffer. Upgrade to cache.db later.

**Retrieve:**
- Recent turn summaries: `retrieve_memories` with `context_type: "turn"`, scoped to current session. These are the continuity data — "what happened so far."
- This is the first time semantic search adds real value. User says "fix the auth bug" → search returns past turns about auth.

**Feed to LLM:** Update `build_messages()` to include recent turn summaries from LocusGraph (not just from Session.turns in memory). This enriches the context with cross-session history — if a past session also worked on auth, those turns show up.

**Runtime changes:**
- `memory.rs` — add `store_turn_start()`, `store_turn_event()`, `store_turn_end()` functions (the hook functions from context.md).
- `agent_loop.rs` — call `store_turn_start()` at turn begin, `store_turn_end()` at turn completion.
- `tool_handler.rs` — after each tool call, call `store_turn_event("action", ...)`.
- `runtime/llm.rs` — after each LLM call, call `store_turn_event("llm", ...)`.
- Add a write buffer (start with `Vec<CreateEventRequest>` on Runtime struct, flushed at turn end).
- `context/messages.rs` — fetch and inject recent turn summaries from LocusGraph.
- `context.md` — update Steps 3-5.

**Test:** Run a full turn (user message → LLM → tool calls → response) → verify turn anchor and all events appear in LocusGraph. Verify seq is monotonically increasing. Verify turn summary overwrites the anchor at end. Start a new session → verify past turn summaries appear in semantic search. Verify write buffer flushes correctly.

**Done when:** Every agent action is recorded. Turn summaries are retrievable. Past session context appears in new sessions. This is the "flywheel start" — the graph is now filling up.

---

## Phase 5 — Knowledge Anchor + Facts

**Hierarchy branch:**

```
project:locuscodes_abc123
  └── knowledge_anchor:locuscodes_abc123               ← this
        ├── fact:rust_error_conventions                ← this
        ├── fact:project_uses_tokio                    ← this
        └── fact:toolbus_is_safety_layer               ← this
```

**Depends on:** Phase 1 (knowledge_anchor extends project root). Phase 4 (semantic search works, context builder can inject facts).

**What:** Project knowledge — facts the agent learns about the codebase. These aren't session events. They're durable knowledge that persists across all sessions.

**Store:**
- At cold start: ensure `knowledge_anchor:{project_name}_{repo_hash}` exists, extending `project:`.
- Facts are created by the agent during sessions when it discovers something worth remembering. For now, the agent stores facts explicitly (e.g., after reading a file and learning something about the project structure).
- Later (Phase 7+), facts can be auto-generated from session analysis.

**Retrieve:** Semantic search already surfaces facts if they match the user's query. No special retrieval needed — they're in the graph, the embeddings handle relevance.

**Feed to LLM:** Facts appear in the semantic search results (Phase 4 already injects these). No context builder change needed. Facts just become part of the "relevant memories" block.

**Runtime changes:**
- `memory.rs` — add `ensure_knowledge_anchor()`. Add `store_fact()` helper.
- Agent behavior — when the agent reads files and discovers patterns (e.g., "this project uses anyhow for errors"), it should store a fact. This is a prompt/behavioral change — add guidance to the system prompt: "When you discover a project convention or important fact, store it to memory."
- `context/prompt.rs` — add a line to the system prompt about storing facts.

**Test:** Agent reads a file, discovers a convention → stores a fact. New session → user asks about that convention → fact appears in semantic results → LLM uses it. Verify facts persist across sessions. Verify they don't duplicate (same context_id = overwrite).

**Done when:** The agent accumulates project knowledge over sessions. Facts enrich future sessions automatically.

---

## Phase 6 — Rules + Constraints + Implicit Engine

**Hierarchy branch:**

```
project:locuscodes_abc123
  └── knowledge_anchor:locuscodes_abc123
        ├── rule:cargo_check_after_edit                ← this
        ├── constraint:no_secrets_in_code              ← this
        └── constraint:files_within_repo_root          ← this

(also built-in global rules, hard-coded in binary)
```

**Depends on:** Phase 5 (rules live under knowledge_anchor). Phase 4 (events exist to check against).

**What:** The safety layer. Rules (soft, should follow) and constraints (hard, must not violate). The implicit engine checks every tool call against them. This is three things built together:

1. Safety cache (in-memory) — loaded at session start
2. Trigger index — HashMap for fast rule lookup per tool
3. Implicit engine — the check that runs before ToolBus

**Store:**
- Built-in rules/constraints are hard-coded in the binary as default `CreateEventRequest` values. Stored to LocusGraph at cold start under a global `rule_anchor:locus` (which means we also need `agent:locus` root — but we can defer the full `agent:locus` hierarchy and just use `rule_anchor:locus` directly for now).
- Project-specific rules stored under `knowledge_anchor:{project}`.

**Retrieve:** Exhaustive load at session start. One call: `retrieve_memories("", context_types: {"constraint": [], "rule": []}, limit: 100)`. Returns all rules and constraints. Cached in-memory for the session.

**Build in-memory structures:**
- Split rules: has `trigger` field → trigger_index HashMap. No trigger → behavioral_rules list.
- Constraints → both trigger_index AND always-include list.
- Build `SafetyCache` struct on Runtime.

**Implicit engine:** New module `implicit_engine.rs`. Before every `ToolBus.call()`:
- `trigger_index.get(tool_name)` + `trigger_index.get("*")`
- Check each rule's condition
- Hard violation → block, return error
- Soft violation → queue warning, continue
- Pass → proceed

**Built-in condition checkers (start with these):**
- `path_within_repo_root` — file path is inside repo root
- `content_size_under_limit` — create_file content < 8k chars
- `no_destructive_command` — bash command doesn't match dangerous patterns (migrates from `requires_confirmation()`)
- `no_secret_patterns` — content doesn't match API_KEY, SECRET, password patterns

**Feed to LLM:**
- Hard constraints → always in system prompt ("## Boundaries")
- Behavioral rules → always in system prompt ("## Working Style")
- Soft violation warnings → prepended to next LLM call

**Runtime changes:**
- New file: `safety_cache.rs` — SafetyCache struct, population logic
- New file: `implicit_engine.rs` — check function, condition evaluators
- `tool_handler.rs` — integrate implicit engine before ToolBus.call()
- `context/prompt.rs` — inject constraints and behavioral rules from SafetyCache
- `memory.rs` — add exhaustive load function for safety data
- Remove `requires_confirmation()` from `tool_handler.rs` (replaced by implicit engine)
- `implicit_links.md` — already documents this, verify alignment

**Test:** Load rules → trigger_index has correct entries. Call edit_file without reading → soft violation. Call create_file with secret pattern → hard block. Call bash with "rm -rf" → hard block. Call bash with "ls" → pass. Verify warnings appear in next LLM context. Verify blocked actions return error without reaching ToolBus.

**Done when:** Safety layer works. Built-in rules enforced. Violations stored as events. This is the "the agent can't do anything stupid" milestone.

---

## Phase 7 — Agent Root + Global Scope

**Hierarchy branch:**

```
agent:locus                                           ← this
  └── project:locuscodes_abc123                       ← already exists (Phase 1)
```

**Depends on:** Phase 1-6 (project-level everything works).

**What:** The root above all projects. Enables cross-project state — global rules, global skills, global learning. Until now, everything was project-scoped. This phase adds the global layer.

**Store:** At cold start (first time ever), create `agent:locus`. Make `project:{project_name}_{repo_hash}` extend `agent:locus`. Move built-in rules from floating `rule_anchor:locus` to extend `agent:locus`.

**Retrieve:** Global data is loaded alongside project data. The exhaustive load (Phase 6) already gets all constraints/rules regardless of project — global ones just have different names (`rule:read_before_edit` vs `rule:locuscodes_abc123__cargo_check`).

**Feed to LLM:** No change. Global and project rules are already in the safety cache and context.

**Runtime changes:**
- `memory.rs` — add `ensure_agent_root()`. Update project anchor creation to extend `agent:locus`.
- Minor: update context_id construction helpers.

**Test:** Create two projects → both extend `agent:locus`. Global rules available in both. Project rules only in their project.

**Done when:** Multi-project foundation works. Short phase.

---

## Phase 8 — Skills

**Hierarchy branch:**

```
agent:locus
  └── skill_anchor:locus                              ← this (global)
        ├── skill:rust_debugging                      ← this
        └── skill:git_best_practices                  ← this

project:locuscodes_abc123
  └── skill_anchor:locuscodes_abc123                  ← this (project)
        ├── skill:anyhow_error_pattern                ← this
        └── skill:toolbus_api_stable                  ← this
```

**Depends on:** Phase 7 (global skill_anchor extends agent:locus). Phase 5 (semantic search returns skills).

**What:** Skills are learned capabilities. They start as facts or patterns and get promoted (later, in Phase 9). For now, skills are manually created by the agent or user.

**Store:**
- `skill_anchor:locus` for global skills, extends `agent:locus`
- `skill_anchor:{project}` for project skills, extends `project:`
- `skill:{name}` extends the appropriate anchor

**Retrieve:** Semantic search (Phase 4) already returns skills — they have type `skill:` which is searchable. No special retrieval needed.

**Feed to LLM:** Skills appear in semantic results. They're high-value context — "how to do X in this project." The context builder already injects semantic results.

**Runtime changes:**
- `memory.rs` — add `ensure_skill_anchors()`, `store_skill()` helper.
- System prompt — add guidance: "When you successfully complete a complex task, store the approach as a skill."

**Test:** Agent completes a task, stores a skill. New session → user asks about similar task → skill appears in context → agent follows the learned approach.

**Done when:** Skills persist and surface. The agent can teach itself.

---

## Phase 9 — Learning System

**Hierarchy branch:**

```
agent:locus
  └── learning_anchor:locus                           ← this (global)
        ├── proficiency_anchor:locus                  ← this
        │     └── proficiency:rust                    ← this
        ├── preference_anchor:locus                   ← this
        │     └── preference:response_style           ← this
        ├── pattern_anchor:locus                      ← this
        │     └── pattern:debug_compile_error         ← this
        └── mistake_anchor:locus                      ← this
              └── mistake:forgot_cargo_check          ← this

project:locuscodes_abc123
  └── learning_anchor:locuscodes_abc123               ← this (project)
        ├── proficiency_anchor:locuscodes_abc123      ← this
        ├── preference_anchor:locuscodes_abc123       ← this
        ├── pattern_anchor:locuscodes_abc123           ← this
        └── mistake_anchor:locuscodes_abc123           ← this
```

**Depends on:** Phase 6 (constraint violations create mistakes). Phase 8 (patterns promote to skills). Phase 4 (events to analyze).

**What:** The full learning system. Four new types: proficiency, preference, pattern, mistake. Session-end analysis. Graduation chain.

**This is the biggest phase. Break it into sub-phases:**

### 9a — Preferences

Simplest learning type. User corrects agent style → store preference.

- `preference_anchor:locus` for global, `preference_anchor:{project}` for project
- `preference:{topic}` — payload: topic, value, source (user/observed), confidence
- Loaded at session start with the exhaustive load (add `"preference": []` to type filter)
- Injected into LLM context in the always-include block
- Test: user says "be more concise" → preference stored → next session → agent is concise

### 9b — Mistakes

Created from repeated constraint violations (Phase 6 already stores violations).

- Session-end analysis: scan violations → group by rule → 2+ violations for same rule → create `mistake:`
- `mistake_anchor:{project}` for project mistakes, `mistake_anchor:locus` for global
- Loaded at session start, injected into context when relevant
- Test: agent violates "read before edit" 3 times → mistake created → next session → mistake appears in context → agent reads first

### 9c — Proficiency

Updated at session end based on success/failure rates.

- `proficiency:{domain}` — payload: domain, level (0.0-1.0), evidence, sessions_count
- Session-end: count successes vs errors per domain → adjust level
- Used as system knob — adjusts semantic search depth, not LLM text
- Test: 5 sessions working on toolbus with 90% success → proficiency rises to 0.7 → less context retrieved → faster

### 9d — Patterns

Recognized recurring workflows.

- Session-end analysis: compare tool call sequences across sessions
- If same sequence appears 3+ times → create `pattern:{name}`
- Pattern payload: trigger, steps, success_rate, times_used
- When pattern matches current task → surface in context
- Pattern success > 80% for 5+ uses → promote to `skill:` (graduation)
- Test: agent adds a new tool in 3 sessions using same steps → pattern recognized → 4th time → pattern in context → agent follows it automatically

### 9e — Graduation chain

```
constraint_violation → mistake → pattern → skill
```

- Mistakes from violations (9b)
- Patterns from repeated workflows (9d)
- Skills from successful patterns (Phase 8)
- Each promotion uses `extends` to the appropriate anchor

**Done when:** The agent improves measurably across sessions. Fewer mistakes, better recall, recognized patterns.

---

## Phase 10 — MCP/ACP + Stuck Detection

**Hierarchy branch:**

```
tool_anchor:locuscodes_abc123
  ├── mcp_anchor:locuscodes_abc123                    ← this
  │     └── mcp:filesystem-server                     ← this
  │           └── mcp_tool:filesystem-server__read    ← this
  └── acp_anchor:locuscodes_abc123                    ← this
        └── acp:code-review-agent                     ← this
```

**Plus:** Stuck detection and confirmation flow.

**Depends on:** Phase 6 (implicit engine for hard blocks as stuck signal). Phase 9 (proficiency for stuck threshold).

**MCP/ACP:**
- `mcp_anchor:` extends `tool_anchor:`
- `mcp:{server_id}` extends `mcp_anchor:`
- `mcp_tool:{server}_{name}` extends `mcp:{server_id}`
- On connect: store tools. On disconnect: `contradicts`. On reconnect: `reinforces` or `contradicts`.
- Already documented in `dynamic_tools.md` — implement what's designed.

**Stuck detection:**
- New `stuck_detector.rs` — checks signals per turn
- Signals: repeated error (2+), hard block, low proficiency (< 0.2), no semantic results, too many turns (5+)
- When stuck → agent asks user instead of guessing
- User response → `feedback:` event → feeds into learning (Phase 9)

**Confirmation flow:**
- Agent presents: what it tried, what failed, options, which it leans toward
- User decides → agent continues
- Over time: fewer confirmations as proficiency increases

**Done when:** MCP tools discoverable. Agent asks for help when stuck instead of looping.

---

## Phase 11 — cache.db Upgrade

**What:** Upgrade the write buffer (from Phase 4's simple Vec) to the full cache.db proxy with read-through / write-behind caching.

**Why last:** By now everything works with the simple buffer. This phase is optimization, not functionality. The system already stores, retrieves, and feeds context correctly. cache.db makes it faster and more durable.

**Build:**
- SQLite layer in LocusGraphClient
- `events_queue` table for write buffering
- `context_cache` table for read caching (exhaustive loads)
- `sync_meta` table for connection state
- Read path: check cache → gRPC if miss
- Write path: insert to queue → async batch sync
- Offline mode: serve from cache, queue writes

**Done when:** Session start is faster (cached reads). Writes never block. Agent survives LocusGraph downtime.

---

## Summary

| Phase | Branch | Builds | Milestone |
|---|---|---|---|
| 1 | `project:` root | Foundation | Project anchor exists |
| 2 | `tool_anchor:` + tools | Tool registration | Tools discoverable in graph |
| 3 | `session_anchor:` + sessions | Session lifecycle | Create/resume/close works |
| 4 | `turn:` + events | Event timeline | Every action recorded, semantic search works |
| 5 | `knowledge_anchor:` + facts | Project knowledge | Agent remembers facts across sessions |
| 6 | Rules + constraints + engine | Safety layer | Agent can't do stupid things |
| 7 | `agent:locus` root | Global scope | Multi-project foundation |
| 8 | `skill_anchor:` + skills | Learned capabilities | Agent teaches itself |
| 9 | `learning_anchor:` + full learning | Improvement over time | Proficiency, preferences, patterns, mistakes |
| 10 | MCP/ACP + stuck detection | Dynamic tools + intelligence | Full tool ecosystem + asks when stuck |
| 11 | cache.db proxy | Performance + durability | Fast, offline-capable |

**Each phase is independently testable. Each makes the system better. You can ship after any phase and have a working product.**

Phase 1-4: working agent with memory.
Phase 5-6: smart agent with safety.
Phase 7-9: learning agent.
Phase 10-11: production agent.
