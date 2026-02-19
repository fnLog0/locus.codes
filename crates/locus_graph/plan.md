# locus_graph — Plan

LocusGraph Rust SDK. The implicit memory layer for locus.codes.
One `graph_id`, one brain — all sessions read/write to the same graph.

**Philosophy**: Amp-style simplicity with LocusGraph as the persistent brain.
No manual AGENT.md files — the agent learns conventions from actions.

---

## Purpose

- **Prevent hallucination** — retrieve relevant memories before every LLM call
- **Persistence** — every tool call, file edit, user intent, and error becomes a memory
- **Learning** — the AI improves across sessions by recalling past context
- **Cross-session** — start a new session, still remember project patterns
- **Semantic recall** — "how do we handle auth?" → relevant memories injected

---

## Amp → LocusGraph Translation

| Amp Concept | locus.codes Equivalent | Advantage |
|-------------|------------------------|-----------|
| `AGENT.md` file | `LocusGraph.retrieve_memories()` | Dynamic, learns automatically |
| Thread history | `context_id: "session:{id}"` | Cross-session persistence |
| `.cursorrules` | `context_id: "project:rules"` | Learns from actions, not static |
| MCP tools | `ToolBus` + MCP adapter | Already implemented |
| Command piping | `ToolBus::call("bash", ...)` | Centralized, sandboxed |

---

## Modules

### 1. `config`

Global configuration. One `graph_id` for the entire system, set once at startup.

```rust
pub struct LocusGraphConfig {
    pub server_url: Option<String>,     // default: https://api.locusgraph.com
    pub agent_secret: Option<String>,   // from LOCUSGRAPH_AGENT_SECRET env var
    pub graph_id: String,               // single brain, never per-session
}

impl LocusGraphConfig {
    pub fn from_env() -> Result<Self, LocusError> {
        Ok(Self {
            server_url: std::env::var("LOCUSGRAPH_SERVER_URL").ok(),
            agent_secret: Some(
                std::env::var("LOCUSGRAPH_AGENT_SECRET")
                    .map_err(|_| LocusError::Config("LOCUSGRAPH_AGENT_SECRET not set".into()))?
            ),
            graph_id: std::env::var("LOCUSGRAPH_GRAPH_ID")
                .unwrap_or_else(|_| "locus-agent".to_string()),
        })
    }
}
```

### 2. `client`

HTTP client wrapping the LocusGraph API. Shared via `Arc<LocusGraphClient>` across the runtime.

```rust
pub struct LocusGraphClient {
    config: LocusGraphConfig,
    http: reqwest::Client,
}

impl LocusGraphClient {
    pub fn new(config: LocusGraphConfig) -> Self;

    /// Store a memory event (fire-and-forget — failures don't block agent)
    pub async fn store_event(&self, event: CreateEventRequest) -> Result<()>;

    /// Semantic search — returns memories relevant to a query
    /// Called BEFORE every LLM call to inject context
    pub async fn retrieve_memories(
        &self,
        query: &str,
        limit: Option<u64>,
        context_ids: Option<Vec<String>>,
        context_types: Option<HashMap<String, Vec<String>>>,
    ) -> Result<ContextResult>;

    /// Reason over stored memories for a task
    pub async fn generate_insights(
        &self,
        task: &str,
        locus_query: Option<&str>,
        limit: Option<u64>,
        context_ids: Option<Vec<String>>,
        context_types: Option<HashMap<String, Vec<String>>>,
    ) -> Result<InsightResult>;

    /// List available context types in the graph
    pub async fn list_context_types(&self) -> Result<Vec<ContextType>>;

    /// Search contexts by name
    pub async fn search_contexts(&self, query: &str, context_type: Option<&str>) -> Result<Vec<Context>>;
}

/// Request to store a memory event
pub struct CreateEventRequest {
    pub event_kind: EventKind,
    pub context_id: Option<String>,
    pub source: Option<String>,
    pub payload: serde_json::Value,
    pub related_to: Option<Vec<String>>,
    pub extends: Option<Vec<String>>,
    pub reinforces: Option<Vec<String>>,
    pub contradicts: Option<Vec<String>>,
    pub timestamp: Option<String>,
}

/// Returned from retrieve_memories
pub struct ContextResult {
    pub memories: String,      // Markdown-formatted string to inject into prompt
    pub items_found: u64,
}

/// Returned from generate_insights
pub struct InsightResult {
    pub insight: String,
    pub recommendation: String,
    pub confidence: f64,
}
```

### 3. `hooks`

Pre-built helpers the runtime calls at specific points. Each builds a `CreateEventRequest` and fires it async (non-blocking).

```rust
impl LocusGraphClient {
    /// After executing any tool (bash, grep, edit_file, etc.)
    /// context_id: "terminal"
    pub async fn store_tool_run(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        result: &serde_json::Value,
        duration_ms: u64,
        is_error: bool,
    );

    /// After writing/editing a file
    /// context_id: "editor"
    pub async fn store_file_edit(
        &self,
        path: &str,
        summary: &str,
        diff_preview: Option<&str>,
    );

    /// When user sends a message
    /// context_id: "user_intent"
    pub async fn store_user_intent(
        &self,
        message: &str,
        intent_summary: &str,
    );

    /// On any error (tool failure, LLM error, etc.)
    /// context_id: "errors"
    pub async fn store_error(
        &self,
        context: &str,
        error_message: &str,
        command_or_file: Option<&str>,
    );

    /// After LLM responds — store the decision/reasoning
    /// context_id: "decisions"
    pub async fn store_decision(
        &self,
        summary: &str,
        reasoning: Option<&str>,
    );

    /// When agent discovers project conventions
    /// context_id: "project:{repo_hash}"
    pub async fn store_project_convention(
        &self,
        repo: &str,
        convention: &str,
        examples: Vec<&str>,
    );

    /// When a pattern is validated (becomes a learned skill)
    /// context_id: "skill:{name}"
    pub async fn store_skill(
        &self,
        name: &str,
        description: &str,
        steps: Vec<&str>,
        validated: bool,
    );
}
```

---

## Integration: The Agent Loop (Amp-style)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              RUNTIME LOOP                                   │
│                                                                             │
│   1. User message arrives                                                   │
│      → hooks::store_user_intent()                                           │
│                                                                             │
│   2. BEFORE LLM call                                                        │
│      → client::retrieve_memories(user_query)                                │
│      → inject memories into prompt                                          │
│      → emit SessionEvent::MemoryRecall { query, items_found }               │
│                                                                             │
│   3. LLM call (streaming)                                                   │
│      → for each text chunk: emit TextDelta                                  │
│      → for each thinking chunk: emit ThinkingDelta                          │
│      → for each tool_use:                                                   │
│          → emit ToolStart                                                   │
│          → ToolBus::call(tool_name, args)                                   │
│          → hooks::store_tool_run()      // remember what happened           │
│          → emit ToolDone                                                    │
│                                                                             │
│   4. After turn completes                                                   │
│      → hooks::store_decision()          // remember AI reasoning            │
│                                                                             │
│   5. On any error                                                           │
│      → hooks::store_error()             // remember failures                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Prompt Structure (Memory-Enhanced)

```
┌─────────────────────────────────────────────────────────────┐
│ SYSTEM PROMPT                                               │
│ - Role: coding agent                                        │
│ - Tools available (from ToolBus)                            │
│ - Safety rules                                              │
├─────────────────────────────────────────────────────────────┤
│ MEMORY (from LocusGraph.retrieve_memories)                  │
│ - Relevant past decisions                                   │
│ - Project conventions learned                               │
│ - Previous errors and fixes                                 │
│ - User preferences                                          │
│ - Learned skills                                            │
├─────────────────────────────────────────────────────────────┤
│ SESSION CONTEXT                                             │
│ - Current working directory                                 │
│ - Recent file edits                                         │
│ - Active task                                               │
├─────────────────────────────────────────────────────────────┤
│ CONVERSATION                                                │
│ - User message                                              │
│ - Previous turns (with tool results)                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Context Scopes (Conventions)

| context_id | What gets stored | When |
|------------|------------------|------|
| `terminal` | Command runs, stdout/stderr, exit codes | After every ToolBus bash call |
| `editor` | File edits, diffs | After edit_file, create_file |
| `user_intent` | User goals, constraints | Every user message |
| `errors` | Failures, stack traces | On any error |
| `decisions` | AI reasoning, architecture choices | After LLM responds |
| `project:{repo}` | Conventions, patterns discovered | When agent learns codebase |
| `skill:{name}` | Learned procedures, best practices | When pattern validated |

---

## TUI Integration (locus_ui)

When memories are retrieved, runtime emits:
```rust
SessionEvent::MemoryRecall {
    query: "how do we handle auth?",
    items_found: 5,
}
```

TUI shows subtle indicator: `📚 5 memories recalled`

---

## Dependencies

```toml
[dependencies]
locus-core = { path = "../locus_core" }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
anyhow = "1"
thiserror = "2"
tracing = "0.1"
```

---

## Error Handling

- `store_event` failures are **non-blocking** — log and continue. Memory is best-effort.
- `retrieve_memories` failures return empty context — agent works without memory, just less informed.
- `generate_insights` failures are surfaced to user if they asked for a summary.

---

## Environment Variables

| Variable | Required | Default |
|----------|----------|---------|
| `LOCUSGRAPH_AGENT_SECRET` | Yes | — |
| `LOCUSGRAPH_SERVER_URL` | No | `https://api.locusgraph.com` |
| `LOCUSGRAPH_GRAPH_ID` | No | `locus-agent` |

---

## Build Order

1. `config` — `LocusGraphConfig::from_env()`
2. `error` — LocusGraph-specific error types
3. `types` — `CreateEventRequest`, `ContextResult`, `InsightResult`, `EventKind`
4. `client` — HTTP client with `store_event`, `retrieve_memories`, `generate_insights`
5. `hooks` — Pre-built helpers on top of client

---

## Why This Is Better Than Amp

| Aspect | Amp | locus.codes |
|--------|-----|-------------|
| Memory | Thread history (session-only) | LocusGraph (cross-session, semantic search) |
| Context file | `AGENT.md` (manual updates) | Auto-learned from actions |
| Skills | Static markdown files | Learned patterns in graph |
| Insights | None | `generate_insights()` for reasoning |
| Learning | Per-session only | Accumulates across all sessions |

---

## Key Principles

1. **Fire-and-forget storage** — Memory writes never block the agent loop
2. **Recall before respond** — Always query memories before LLM call
3. **Semantic over literal** — Query by meaning, not exact match
4. **Cross-session persistence** — New session, same brain
5. **Learn from actions** — No manual AGENT.md maintenance
