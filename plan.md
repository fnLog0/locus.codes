# plan.md — Build locus.codes Using locus.codes (Bootstrapping Lifecycle)

**Strategy:** Build a thin kernel manually → use locus.codes to expand itself incrementally.

The agent needs stable tooling before it can build more tooling. Each phase produces a deliverable that unlocks the next phase.

---

## Lifecycle

```
Kernel → Single Agent → Diff Workflow → Subagents → LocusGraph → Constraints → Modes → Optimization
```

---

## Phase 0 — Define the Kernel (Manual, 3–5 days)

Write by hand. No AI dependency. This is the OS kernel — without it, nothing works.

### 0.1 — Cargo Workspace

```
locuscodes/
├── Cargo.toml              # workspace members
├── locus-cli/              # CLI entry point
├── locus-ui/               # ratatui + crossterm TUI
├── locus-runtime/          # orchestrator, scheduler, session manager, event bus, mode controller
├── locus-toolbus/          # execution gateway (all file/cmd/git ops)
├── locus-agents/           # subagent implementations
├── locus-llm/              # model router, prompt builder, response parser
├── locus-graph/            # LocusGraph SDK client, injection engine, event extractor
├── landing/                # landing page (existing)
├── locus_docs/             # architecture docs (existing)
└── plan.md                 # this file
```

### 0.2 — CLI (`locus-cli`)

- [x] `locus run` — boots the TUI app
- [x] Arg parsing: `--mode rush|smart|deep`, `--repo <path>`
- [x] Detects repo root (walk up to `.git`)

### 0.3 — UI Shell (`locus-ui`)

Built with **ratatui + crossterm**. The UI is mission control, not a text editor.

Layout (from `02_ui_layer/ui_overview.md`):
```
┌──────────────────────────────────────────┐
│  Nav Bar  [mode: Smart]  [view: Tasks]   │
├──────────────────────────────────────────┤
│           Main Content Area              │
│       (switches between 6 views)         │
├──────────────────────────────────────────┤
│  > prompt input bar (always visible)     │
└──────────────────────────────────────────┘
```

- [x] App frame: nav bar + main content + prompt bar
- [x] View Router: stack-based, `Esc` pops, direct shortcuts replace top
- [x] Task Board (home/default view): Active Task + Task Queue + History sections
- [x] Prompt Bar: always visible at bottom, multi-line (Shift+Enter), mode indicator `[Rush]`/`[Smart]`/`[Deep]`, submit with Enter
- [x] Command palette: `:mode`, `:view`, `:quit`, `:cancel`
- [x] Keybindings — global: `Ctrl+P` focus prompt, `Esc` back, `Ctrl+C` cancel, `Ctrl+Q` quit
- [x] Keybindings — view switching: `1` Task Board, `2` Plan, `3` Agents, `4` Diff Review, `5` Logs, `6` Memory Trace
- [x] Keybindings — navigation: `j`/`k` scroll, `g`/`G` top/bottom, `/` search, `Enter` select
- [x] Stub the other 5 views (Plan, Agents, Diff Review, Logs, Memory Trace) as empty placeholders

### 0.4 — Session Manager (`locus-runtime`)

- [x] State: `repo_root`, `branch`, `working_dir`, `git_state`, `mode`, `thread`, `config`
- [x] On startup: detect repo root, read branch + git state, load session config
- [x] Repo metadata: language/framework detection, test framework detection, project structure summary, `.gitignore` patterns
- [x] Thread tracking: prompt history, task results (stored locally, not in LocusGraph)

### 0.5 — Event Bus (`locus-runtime`)

Pub/sub for runtime ↔ UI real-time updates.

Runtime events to define (from `08_protocols/runtime_events.md`):

| Event | Payload |
|-------|---------|
| `TaskStarted` | task_id, prompt, mode |
| `TaskCompleted` | task_id, summary, duration |
| `TaskFailed` | task_id, error, step |
| `AgentSpawned` | agent_id, agent_type, task |
| `AgentCompleted` | agent_id, status, result |
| `ToolCalled` | tool, args, agent_id |
| `ToolResult` | tool, success, result, duration |
| `DiffGenerated` | files, hunks_count |
| `DiffApproved` | files |
| `DiffRejected` | files, reason |
| `TestResult` | passed, failed, total, output |
| `DebugIteration` | iteration, failure_summary |
| `CommitCreated` | hash, message, files |
| `MemoryRecalled` | locus_count, top_confidence |
| `MemoryStored` | event_kind, context_id |
| `ModeChanged` | old_mode, new_mode |

- [x] Event enum with all types above
- [x] Pub/sub channel (tokio broadcast or mpsc)
- [x] UI subscribes and re-renders on events
- [x] Automatic view switching: `DiffGenerated` → Diff Review, test failures → Logs View, debug loop → Agents View

### 0.6 — ToolBus (`locus-toolbus`)

Execution gateway. **All actions must go through ToolBus.** This is where safety and determinism lives.

Tools (from `08_protocols/toolbus_api.md`):

| Tool | Input | Output | Permission |
|------|-------|--------|------------|
| `file_read` | `{ path }` | `{ content, size }` | read |
| `file_write` | `{ path, content }` | `{ ok }` | write |
| `run_cmd` | `{ cmd, cwd?, timeout? }` | `{ stdout, stderr, exit_code }` | execute |
| `grep` | `{ pattern, path?, glob?, case_sensitive? }` | `{ matches[] }` | read |
| `glob` | `{ pattern }` | `{ files[] }` | read |
| `git_status` | `{}` | `{ status, branch, clean }` | read |
| `git_diff` | `{ path?, staged? }` | `{ diff }` | read |
| `git_add` | `{ paths[] }` | `{ ok }` | write |
| `git_commit` | `{ message }` | `{ hash }` | git_write |
| `git_push` | `{ force? }` | `{ ok }` | git_write |

Common response envelope:
```json
{ "tool": "...", "success": true|false, "result": {...}, "duration_ms": 42 }
```

- [x] Tool trait: `async fn call(args) → ToolResult`
- [x] All 10 tools implemented
- [x] Every call emits `ToolCalled` + `ToolResult` events on Event Bus
- [x] Permission enforcement (read=always, write=configurable, execute=configurable, git_write=ask)
- [x] Filesystem isolation: access limited to repo root only, symlinks outside blocked
- [x] Blocked commands: `rm -rf`, `sudo`, `curl`, `wget`
- [x] Allowed commands allowlist: `cargo test`, `cargo build`, `npm test`, etc.
- [x] Timeout per command (default 60s)
- [ ] Permission prompt in UI: `Allow file_write to src/foo.rs? [y/n/always]`

### 0.7 — Orchestrator (`locus-runtime`)

Basic single-thread task loop (no DAG yet, no parallel agents).

- [x] Receive prompt from UI via Event Bus
- [x] Analyze intent (simple: just forward to LLM)
- [x] Track task state: prompt → running → done/failed
- [x] Emit `TaskStarted`, `TaskCompleted`, `TaskFailed` events
- [x] Handle `:cancel` abort
- [x] Graceful shutdown

### 0.8 — Mode Controller (`locus-runtime`)

- [x] Mode enum: Rush / Smart / Deep
- [x] Default: Smart
- [x] `:mode rush|smart|deep` command
- [x] `F1`/`F2`/`F3` shortcuts
- [x] Mode indicator in prompt bar
- [x] Emit `ModeChanged` event

**Phase 0 Deliverable:** `locus run` opens TUI, shows Task Board, accepts prompt input, can call ToolBus tools (file_read, run_cmd, etc.), displays results in Logs View.

---

## Phase 1 — Single Agent MVP (Manual + AI assist)

**Phase 0 Completed:** Kernel is fully functional with TUI, 10 ToolBus tools (with permission enforcement and command blocking), view router with 6 views, command palette, session management with repo metadata, and full event bus support.

Before subagents, build one end-to-end pipeline:

```
Prompt → Scan repo → Generate patch → Show diff → Apply → Run tests
```

### 1.1 — LLM Engine (`locus-llm`)

- [ ] Model interface: `Input(system_prompt + memory_bundle + user_prompt + tool_definitions) → Output(tool_calls[] + reasoning + confidence)`
- [ ] Single model connection (no routing yet, pick one model)
- [ ] Self-hosted model integration (no external API calls)
- [ ] Prompt Builder: assemble system prompt + tool definitions + user prompt
- [ ] Response parser: validate JSON, extract `tool_calls[]`, `reasoning`, `confidence`
- [ ] Error handling: invalid JSON → retry, unknown tool → retry with tools reminder, max retries → fail
- [ ] System prompt templates per agent type (short + focused, from `06_llm_engine/prompt_templates.md`)
- [ ] Token budgets: Rush=6K, Smart=24K, Deep=48K
- [ ] Timeouts: Rush=30s, Smart=120s, Deep=300s
- [ ] Retry limits: Rush=1, Smart=3, Deep=5

### 1.2 — Secrets Safety (`locus-toolbus` / `locus-llm`)

- [ ] Secret detector: scan for API keys (`sk-`, `AKIA`), tokens, passwords, connection strings, private keys, base64 secrets
- [ ] Filter env vars before LLM context injection
- [ ] Sanitize ToolBus output before display/storage
- [ ] Redact with `[REDACTED]` on detection
- [ ] Block patches that contain secrets

### 1.3 — Basic Pipeline (wired through Orchestrator)

- [ ] Orchestrator: prompt → call LLM with repo context → get tool_calls → execute via ToolBus → return results
- [ ] Repo scan: walk directory, read relevant files, build context for LLM
- [ ] LLM generates file_write tool_calls → Orchestrator executes them
- [ ] Logs View: display command output and results
- [ ] Test: run `locus run`, type a prompt, see LLM response + tool execution

**Phase 1 Deliverable:** locus.codes can receive a prompt, call the LLM, execute tool calls, and display results. No diff review yet — changes apply directly (with write permission prompt).

---

## Phase 2 — Diff-First Workflow (Critical — Makes It Safe)

Without this, the agent will corrupt its own code. This is the safety layer.

### 2.1 — Diff Generation (`locus-runtime` / `locus-agents`)

From `07_execution_engine/diff_generation.md`:

- [ ] PatchAgent outputs new file content (not direct writes)
- [ ] Diff engine: compare original file ↔ new content → unified diff
- [ ] Unified diff format: file headers, hunk headers (`@@ -line,count +line,count @@`), context lines (3 before/after), `+`/`-` lines
- [ ] Multi-file diffs: each file gets its own hunks
- [ ] Syntax highlighting per language

### 2.2 — Patch Pipeline (`locus-runtime`)

From `07_execution_engine/patch_pipeline.md`:

```
Task → File identification → Context assembly → Patch generation → Diff creation
  → Validation → Diff Review → Apply atomically → Rollback on failure
```

- [ ] Patch validation: patch applies cleanly (no conflicts), basic syntax check (file parses after patch)
- [ ] Atomic application: all files or nothing
- [ ] Working directory state saved before apply
- [ ] Rollback: automatic on failure, manual via command
- [ ] Staging area: track pending changes separately from working dir

### 2.3 — Diff Review View (`locus-ui`)

From `02_ui_layer/views/diff_review.md`:

```
┌──────────────────────────────────────────┐
│ Diff Review: Fix auth bug in login.rs    │
│ Files changed: 2                         │
├──────────────────────────────────────────┤
│ ▸ src/auth/login.rs (+12 -3)             │
│ ▸ src/auth/tests.rs (+28 -0)             │
├──────────────────────────────────────────┤
│ (syntax-highlighted diff with context)   │
├──────────────────────────────────────────┤
│ [a]pprove  [r]eject  [e]dit  [n]ext     │
└──────────────────────────────────────────┘
```

- [ ] File list at top, navigable (`n`/`p` next/prev file)
- [ ] Syntax-highlighted diffs with context lines
- [ ] Hunk-level approve/reject
- [ ] `a` approve all, `r` reject all, `e` edit (prompt modification)
- [ ] `j`/`k` scroll within diff
- [ ] Auto-opens when `DiffGenerated` event fires
- [ ] On approve → patches applied, pipeline continues
- [ ] On reject → Orchestrator notified, can retry or abort

### 2.4 — Updated Pipeline

- [ ] Orchestrator: prompt → LLM → PatchAgent generates diff (not direct writes) → Diff Review → user approves → ToolBus file_write → done
- [ ] Input states: Ready (waiting), Running (task in progress), Confirmation (approve/reject in Diff Review)

**Phase 2 Deliverable:** locus.codes generates unified diffs, shows them in Diff Review, user approves/rejects, patches applied atomically with rollback.

---

## ⚡ Self-Hosting Begins Here

After Phase 2, locus.codes can safely modify its own code:

```
locus task "Implement X" → review diff → approve → apply → test
```

Every feature below is built BY locus.codes ON locus.codes.

---

## Phase 3 — Parallel Subagents

### 3.1 — Agent Trait + Reports (`locus-agents`)

From `03_runtime_core/subagents.md` and `08_protocols/agent_reports.md`:

- [ ] `Agent` trait: `async fn run(context) → AgentReport`
- [ ] AgentReport: `{ agent_id, agent_type, task_id, status(success|failure|partial), result, artifacts, duration_ms, tokens_used }`
- [ ] Agents do NOT communicate directly — all coordination through Orchestrator
- [ ] Each agent has its own context window (no shared state)

### 3.2 — Scheduler (`locus-runtime`)

From `03_runtime_core/scheduler.md`:

- [ ] Receive DAG from Orchestrator
- [ ] Identify ready tasks (no unmet dependencies) → spawn agents
- [ ] On completion: mark done, check dependents, spawn newly-ready tasks
- [ ] On failure: report to Orchestrator, pause dependents
- [ ] Async execution: tokio tasks, results via channels
- [ ] Max concurrent agents: Rush=2, Smart=4, Deep=6
- [ ] Timeout per agent (configurable)
- [ ] Priority: MemoryRecallAgent=High, RepoAgent/SearchAgent/PatchAgent=Normal, ConstraintAgent=Low
- [ ] Emit `AgentSpawned`, `AgentCompleted` events

### 3.3 — Orchestrator DAG Builder (`locus-runtime`)

From `03_runtime_core/orchestrator.md`:

```
"Fix auth bug in login.rs"
├── [parallel]
│   ├── RepoAgent: scan repo, find relevant files
│   ├── MemoryRecallAgent: recall relevant memories (Phase 4)
│   └── SearchAgent: grep/search for patterns
├── [sequential]
│   ├── PatchAgent: generate fix (depends on parallel results)
│   ├── DiffReview: user approval
│   ├── TestAgent: run tests
│   ├── DebugAgent: fix failures (conditional)
│   └── Commit: git commit (conditional)
└── EventExtractor: write memories (Phase 4, always)
```

- [ ] DAG construction: decompose prompt into subtasks with dependencies
- [ ] Parallel branch identification
- [ ] Track state of every DAG node
- [ ] Cancel/abort on user request

### 3.4 — Subagents (first 3)

**RepoAgent:**
- [ ] Input: task description, repo metadata
- [ ] Output: file tree, relevant file paths, file contents
- [ ] Tools: `file_read`, `grep`, `glob`

**PatchAgent:**
- [ ] Input: task, relevant files, memory bundle, search results
- [ ] Output: unified diffs
- [ ] Tools: `file_read`, `file_write` (via ToolBus)
- [ ] Uses LLM

**TestAgent:**
- [ ] Auto-detect test framework from Session Manager metadata: Cargo.toml→`cargo test`, package.json→`npm test`, pytest.ini→`pytest`, go.mod→`go test ./...`, Makefile→`make test`
- [ ] Run via ToolBus `run_cmd` (sandboxed)
- [ ] Parse output: `TestResult { total, passed, failed, skipped, failures[], duration }`
- [ ] On failure → report to Orchestrator
- [ ] Emit `TestResult` event

### 3.5 — DebugAgent + Debug Loop

From `07_execution_engine/debug_loop.md`:

```
Test failure → DebugAgent: analyze → generate fix → apply → test again → loop or exit
```

- [ ] Input: test failure output, changed files, original task
- [ ] Output: fix patch (unified diff), root_cause analysis
- [ ] Recalls relevant memories (Phase 4)
- [ ] Max retries: Rush=0 (fail immediately), Smart=3, Deep=5
- [ ] Debug iteration: analyze → fix → test → evaluate
- [ ] Exit: tests pass | max retries | user cancels
- [ ] Emit `DebugIteration` events

### 3.6 — SearchAgent

- [ ] Input: search queries (from task analysis)
- [ ] Output: matching files, lines, context
- [ ] Tools: `grep`, `glob`, `file_read`

### 3.7 — Commit Pipeline

From `07_execution_engine/commit_pipeline.md`:

```
Tests pass → user confirms → LLM generates commit message → git add → git commit → optional git push
```

- [ ] LLM generates commit message from: original task, files changed, test results
- [ ] Conventional commits format (configurable)
- [ ] `git add` → `git commit` → optional `git push` (all via ToolBus with permissions)
- [ ] `git push --force` blocked by default
- [ ] Rollback: `git reset --soft HEAD~1` (via ToolBus, requires permission)
- [ ] Emit `CommitCreated` event

### 3.8 — Remaining UI Views

- [ ] Plan View: execution DAG visualization
- [ ] Agents View: active subagent cards (spawned agents, status, duration)
- [ ] Logs View: command output display (ToolBus results)

**Phase 3 Deliverable:** Tasks run with parallel subagents. RepoAgent scans, PatchAgent generates, TestAgent tests, DebugAgent fixes, SearchAgent searches. DAG execution with proper dependency ordering.

---

## Phase 4 — LocusGraph Integration (The Real Product Starts)

Integrate deterministic memory. The LLM stays unaware — injection is implicit.

### 4.1 — LocusGraph SDK Client (`locus-graph`)

From `04_locusgraph_memory/`:

- [ ] `store_event(CreateEventRequest)` — store one memory event
- [ ] `retrieve_memories(graph_id, query, limit, context_ids, context_types)` → `ContextResult { memories: String, items_found: u64 }`
- [ ] `generate_insights(graph_id, task, ...)` → `{ insight, recommendation, confidence }`
- [ ] `list_context_types(graph_id, ...)`
- [ ] `list_contexts_by_type(graph_id, type, ...)`
- [ ] `search_contexts(graph_id, q, ...)`

### 4.2 — Event Schema

`CreateEventRequest` (from `04_locusgraph_memory/event_model.md`):

```rust
CreateEventRequest {
    graph_id: String,
    event_kind: String,       // fact | action | decision | observation | feedback
    context_id: Option<String>, // "terminal" | "editor" | "user_intent" | "errors" | "project" | "constraints"
    source: Option<String>,
    payload: serde_json::Value,
    related_to: Option<Vec<String>>,
    extends: Option<Vec<String>>,
    reinforces: Option<Vec<String>>,
    contradicts: Option<Vec<String>>,
    timestamp: Option<String>,
}
```

Context ID conventions:

| context_id | Stored By | Contains |
|------------|-----------|----------|
| `terminal` | ToolBus (after run_cmd) | Commands, stdout, stderr, exit codes |
| `editor` | ToolBus (after file_write) | File paths, edit summaries, diff previews |
| `user_intent` | Orchestrator (on prompt) | User messages, intent summaries |
| `errors` | ToolBus / agents | Error messages, failure context |
| `project` | Session Manager | Project-level facts (language, framework, structure) |
| `constraints` | ConstraintAgent | Rules the agent must follow |

- [ ] All event types implemented as Rust types
- [ ] Payload conventions: `kind` + data pattern
- [ ] Relation fields: `related_to`, `extends`, `reinforces`, `contradicts`

### 4.3 — Event Extractor (Layer H — Learning Engine)

From `01_system_architecture/architecture.md`:

After every action (diff/test/commit):
1. Logs + diffs → deterministic event extractor
2. Events written to LocusGraph
3. Relations + reinforcement updated

- [ ] ToolBus hook: after `run_cmd` → store action event (terminal context)
- [ ] ToolBus hook: after `file_write` → store fact event (editor context)
- [ ] Orchestrator hook: on prompt → store fact event (user_intent context)
- [ ] TestAgent hook: on pass → store observation with `reinforces`
- [ ] TestAgent hook: on fail → store fact in errors context
- [ ] Commit hook: store action event with commit hash, files, test results
- [ ] Emit `MemoryStored` events

### 4.4 — Memory Recall + Injection Engine (Layer G — Secret Weapon)

From `04_locusgraph_memory/injection_engine.md`:

```
Orchestrator → MemoryRecallAgent → retrieve_memories() → Injection Engine → inject into LLM prompt
```

The LLM never "queries memory." Memories just appear in context.

Injection position (from `06_llm_engine/prompt_templates.md`):
```
┌─────────────────────────────┐
│ System Prompt               │
├─────────────────────────────┤
│ {memories from retrieval}   │  ← inject here
├─────────────────────────────┤
│ Tool Definitions            │
├─────────────────────────────┤
│ User Prompt                 │
└─────────────────────────────┘
```

- [ ] MemoryRecallAgent: makes multiple scoped retrievals and concatenates
  - General: `retrieve_memories(gid, "project context", 5, None, None)`
  - Constraints: `retrieve_memories(gid, "rules", 10, ["constraints"], None)`
  - Task-specific: `retrieve_memories(gid, &task_text, 5, None, None)`
  - Bundle: `format!("{}\n\n{}\n\n{}", general, constraints, relevant)`
- [ ] Retrieval limits per mode: Rush=5, Smart=10, Deep=20
- [ ] Token budget per mode: Rush≈500, Smart≈2000, Deep≈5000
- [ ] Insights injection for complex tasks: `generate_insights()` → insight + recommendation
- [ ] Prompt Builder updated: insert memory bundle between system prompt and tool definitions
- [ ] Memory Trace view: UI shows what memories were recalled/injected, locus count, confidence

### 4.5 — Reinforcement (Skill Formation)

From `04_locusgraph_memory/reinforcement.md`:

- [ ] Test pass after fix → store observation with `reinforces` linking to the approach
- [ ] User approves patch → store feedback with `reinforces`
- [ ] User rejects patch → store feedback with `contradicts`
- [ ] Approach fails → store with `contradicts` (lowers future retrieval rank)
- [ ] LocusGraph server-side ranking boosts reinforced events, lowers contradicted events

**Phase 4 Deliverable:** locus.codes stores memories after every task, recalls relevant context before every task, and injects it into the LLM transparently. Skills emerge from reinforcement, not static files.

---

## Phase 5 — Constraint Engine (Trust + Correctness)

Constraints are enforced **before** patch approval. The agent stops repeating bad behavior.

### 5.1 — Constraint Storage

From `04_locusgraph_memory/constraints.md`:

- [ ] Constraints stored as `event_kind: "fact"`, `context_id: "constraints"`
- [ ] Payload: `{ kind: "constraint", rule: "...", scope: "global"|"project", severity: "error"|"warning" }`
- [ ] Default constraints: always run tests before committing, never force-push to main, never hardcode secrets, no files > 500 lines

### 5.2 — ConstraintAgent (`locus-agents`)

From `03_runtime_core/subagents.md`:

- [ ] Input: proposed changes, active constraints from LocusGraph
- [ ] Retrieves constraints: `retrieve_memories(gid, "active constraints", 20, ["constraints"], None)`
- [ ] Checks proposed actions against constraint rules
- [ ] Output: pass/fail per constraint, violations list

### 5.3 — Violation Detection + Blocking

From `04_locusgraph_memory/violations.md`:

- [ ] Violation event: `event_kind: "observation"`, `context_id: "errors"`, payload with `kind: "constraint_violation"`, `related_to: ["constraints"]`
- [ ] Severity `error` → block the action, user cannot proceed until fixed
- [ ] Severity `warning` → allow with notice
- [ ] Violations surfaced in prompt bar or Diff Review
- [ ] Violations recalled by `retrieve_memories` in future similar tasks → agent learns

### 5.4 — Patch Pipeline Integration

- [ ] ConstraintAgent runs after diff generation, before Diff Review
- [ ] Violations shown alongside diff
- [ ] "Must fix before commit" enforcement

**Phase 5 Deliverable:** Agent stops repeating mistakes. Constraints enforced before approval. Violations stored and recalled.

---

## Phase 6 — Modes (Rush / Smart / Deep)

Product polish. Modes control the entire agent behavior stack.

From `01_system_architecture/modes.md` and `06_llm_engine/safety_and_limits.md`:

### Mode Effects

| Dimension | Rush | Smart | Deep |
|-----------|------|-------|------|
| Model | Cheap/fast | Balanced SOTA | Strongest |
| Max concurrent agents | 2 | 4 | 6 |
| Memory retrieval limit | 5 (~500 tokens) | 10 (~2K tokens) | 20 (~5K tokens) |
| Input token budget | 4K | 16K | 24K |
| Output token budget | 2K | 8K | 16K |
| Request timeout | 30s | 120s | 300s |
| Retry limit | 1 | 3 | 5 |
| Debug loop iterations | 0 (fail fast) | 3 | 5 |
| Test strictness | Skip if unnecessary | Run tests | Full suite + benchmarks |
| Subagents | RepoAgent + PatchAgent only | All 7 | All 7 + extended context |

### Rush specifics
- Reduced context window (fewer memories injected)
- Skip SearchAgent if unnecessary
- No debug loop (fail fast, report to user)
- Best for: rename variable, fix typo, add import, one-file edits

### Smart specifics
- Full context with memory injection
- All relevant subagents spawned
- Standard debug loop (3 retries)
- Best for: feature implementation, bug fixes, refactoring

### Deep specifics
- Maximum context with deep memory recall
- Extended thinking / chain-of-thought enabled
- Extended debug loop (5+ retries)
- Best for: architecture decisions, cross-file refactors, complex debugging

### Fallback
- Rush → fall back to any available model
- Smart → fall back to Rush with warning
- Deep → fail with error (no compromise on quality)

### Checklist

- [ ] Model Router (`locus-llm`): select model based on active mode
- [ ] Scheduler: respect mode limits (max agents, timeouts)
- [ ] MemoryRecallAgent: retrieval depth varies by mode
- [ ] Prompt Builder: token budget enforcement per mode
- [ ] TestAgent: strictness varies by mode
- [ ] DebugAgent: retry limit varies by mode
- [ ] Fallback logic per mode

**Phase 6 Deliverable:** locus.codes has three distinct operating modes that control model selection, agent spawning, memory depth, and test strictness.

---

## Non-Negotiable Invariants

Lock these early. If they break, the system self-corrupts.

| Invariant | Why |
|-----------|-----|
| ToolBus API never changes randomly | Every agent and the Orchestrator depend on it |
| Event schema is versioned (v1, v2, ...) | Memory corruption if schema drifts |
| RuntimeEvent protocol is stable | UI ↔ Runtime contract |
| Diff review required for all writes | Self-corruption prevention |
| Secrets never in prompts/events/logs | Security baseline |
| Filesystem access limited to repo root | Sandbox safety |

---

## Sandbox Enforcement (from `09_security/sandbox.md`)

Apply from Phase 0 onward:

| Restriction | Detail |
|-------------|--------|
| Filesystem | Repo root only, symlinks outside blocked, `/tmp` allowed |
| Network | No outbound by default, relaxable per-session |
| Resources | CPU 120s, Memory 512MB, File size 50MB, Open files 256, Processes 32 |
| Environment | Sensitive vars stripped, `HOME` set to sandbox dir, `PATH` minimal |

---

## Daily Development Loop (Post Phase 2)

```
1. Create task         → locus task "implement X"
2. Agent plans         → Orchestrator builds DAG
3. Agents scan         → RepoAgent + SearchAgent find context
4. Memory recalled     → MemoryRecallAgent injects relevant memories (Phase 4+)
5. Patch generated     → PatchAgent generates unified diff
6. Constraints checked → ConstraintAgent validates (Phase 5+)
7. You review          → approve/reject hunks in Diff Review
8. Apply               → patch applied atomically
9. Run tests           → TestAgent: cargo test
10. Debug loop         → DebugAgent fixes failures (if any)
11. Commit             → LLM generates message, git commit
12. Learn              → Event Extractor writes to LocusGraph (Phase 4+)
```

This is the closed feedback loop. The 10-step execution pipeline from `01_system_architecture/execution_pipeline.md`.

---

## Current Status

- [x] Architecture docs complete (`docs/` — 30+ files covering all 8 layers)
- [x] Landing page exists (`apps/landing/`)
- [x] Plan defined (this file)
- [x] **Phase 0 — Kernel** ← COMPLETE
- [ ] **Phase 1 — Single Agent MVP** ← NEXT
