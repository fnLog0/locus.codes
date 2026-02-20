# Tool Discovery — Semantic Tool Selection via LocusGraph

## The Problem

Every tool loaded into the LLM prompt costs tokens. With few tools, this is fine. At scale, it breaks:

```
7 tools   × ~200 tokens =   1,400 tokens  ✓ current state
50 tools  × ~200 tokens =  10,000 tokens  ✗ wasteful
300 tools × ~200 tokens =  60,000 tokens  ✗ exceeds useful context
```

The agent should see only **5–15 relevant tools per call**, regardless of how many exist.

---

## Solution: Three Tiers

Store all tool schemas in LocusGraph. Before each LLM call, retrieve only what's relevant. The agent learns which tools work for which intents over time.

```
┌──────────────────────────────────────────────────────┐
│  TIER 0 — Always Available (~1,200 tokens)           │
│  Core tools loaded in EVERY prompt                   │
│                                                      │
│  tool_search, tool_explain      ← meta-tools         │
│  bash, edit_file, create_file   ← file ops           │
│  glob, grep, finder             ← search             │
│  undo_edit                      ← safety             │
├──────────────────────────────────────────────────────┤
│  TIER 1 — Hot Tools (~1,800 tokens, brief schemas)   │
│  0–10 tools selected per call via LocusGraph         │
│                                                      │
│  Promoted by: past usage, repo context, user intent  │
├──────────────────────────────────────────────────────┤
│  TIER 2 — Long Tail (0 tokens until requested)       │
│  280+ tools never loaded directly                    │
│                                                      │
│  Accessed ONLY via: tool_search → tool_explain       │
│  Full schema expanded only when about to execute     │
├──────────────────────────────────────────────────────┤
│  TOKEN BUDGET:                                       │
│  Tier 0: 1,200  +  Tier 1: 1,800  +  Tier 2: 0–800  │
│  Total: ~3,800 tokens (vs 60,000 without tiers)      │
└──────────────────────────────────────────────────────┘
```

---

## How It Works

### 1. Register Tool Schemas as Memories

At startup (ToolBus) or on connect (MCP/ACP), store each tool as a fact:

```rust
async fn register_tool_in_graph(
    client: &LocusGraphClient,
    tool_name: &str,
    description: &str,
    parameters_schema: &serde_json::Value,
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
                "tags": tags,
            }
        }),
    )
    .context_id(format!("tool:{}", tool_name))
    .source("system");

    client.store_event(event).await;
}
```

```rust
// ToolBus tools
for tool in toolbus.list_tools() {
    register_tool_in_graph(&graph, tool.name, tool.description, &tool.parameters, vec!["toolbus"]).await;
}

// MCP server tools
for tool in mcp_server.list_tools().await? {
    register_tool_in_graph(&graph, &tool.name, &tool.description, &tool.input_schema, vec!["mcp", &mcp_server.name]).await;
}
```

### 2. Select Relevant Tools Before Each LLM Call

This replaces `toolbus.list_tools()` in the runtime:

```rust
async fn select_hot_tools(
    graph: &LocusGraphClient,
    catalog: &ToolCatalog,
    user_message: &str,
    repo_context: &RepoContext,
    token_budget: u32,  // ~1,800 for Tier 1
) -> Vec<ToolDescriptor> {
    // Ask LocusGraph for tools matching this intent
    let memories = graph.retrieve_memories(
        user_message,
        Some(RetrieveOptions::new()
            .limit(15)
            .context_type("fact", ContextTypeFilter::new().name("tool"))
        ),
    ).await.unwrap_or_default();

    let mut candidates = parse_tool_ids_from_memories(&memories);

    // Cold start fallback: repo-specific defaults
    if candidates.is_empty() {
        candidates = repo_context.default_tools();
    }

    // Fill up to token budget using brief schemas
    let mut selected = Vec::new();
    let mut tokens_used = 0;
    for tool_id in candidates {
        if let Some(tool) = catalog.get(&tool_id) {
            if tokens_used + tool.tokens_brief <= token_budget {
                tokens_used += tool.tokens_brief;
                selected.push(tool.clone());
            }
        }
    }

    selected
}
```

### 3. Two Meta-Tools for Long-Tail Discovery

The model uses `tool_search` and `tool_explain` to find tools not in Tier 0/1:

```
User: "create a PR for this branch"

LLM sees: 10 core tools (Tier 0) + 3 hot tools (Tier 1)
LLM thinks: "I don't see a PR tool, let me search"

1. tool_search(query="create GitHub pull request")
   → [{ tool_id: "mcp.github.create_pr", summary: "Create a pull request" }, ...]

2. tool_explain(tool_id="mcp.github.create_pr", detail="full")
   → { args: { owner, repo, title, head, base }, returns: "PR object" }

3. Agent calls mcp.github.create_pr with correct args

4. store_tool_run() → graph learns: "create PR" → mcp.github.create_pr
   → Next time: tool appears in Tier 1 automatically
```

#### `tool_search`

```rust
struct ToolSearchArgs {
    query: String,
    domain: Option<String>,     // "repo", "infra", "code"
    namespace: Option<String>,  // "github", "k8s"
    max_results: Option<u32>,   // default: 5
}
```

Under the hood: lexical match on name/summary/tags + LocusGraph `retrieve_memories()` boost for tools that worked for similar intents before.

#### `tool_explain`

```rust
struct ToolExplainArgs {
    tool_id: String,
    detail: Option<String>,  // "brief" (default, ~50 tokens) or "full" (~250 tokens)
}
```

### 4. Learn from Tool Usage

Every tool execution feeds back into the graph:

```rust
// After execution — already exists in hooks.rs
graph.store_tool_run("mcp.github.create_pr", &args, &result, duration_ms, false).await;
```

The graph builds three kinds of knowledge over time:

- **Intent → Tool**: "create PR" → `mcp.github.create_pr`
- **Tool → Outcome**: "create_pr usually succeeds, takes ~1200ms"
- **Tool chains**: "bug fix" → `grep → read → edit_file → bash`

```
Session 1:  No history → tool_search needed → 1-2 extra round-trips
Session 5:  Some history → Tier 1 partially filled → fewer searches
Session 20: Rich history → right tools surfaced instantly
```

#### Store tool chain patterns

```rust
graph.store_event(CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "tool_chain",
        "data": {
            "chain": ["grep", "read", "edit_file", "bash"],
            "intent": "fix a bug",
            "success": true,
        }
    }),
)
.context_id("skill:tool_chain:bug_fix")
.source("agent")
).await;
```

### 5. Generate Insights for Complex Tasks

For multi-step work, ask the graph for a plan:

```rust
let insight = graph.generate_insights(
    "Set up CI/CD pipeline for the Rust workspace",
    Some(InsightsOptions::new()
        .locus_query("ci cd pipeline rust cargo")
        .limit(10)),
).await?;

// insight.recommendation:
// "Based on past sessions:
//  1. Use bash to create .github/workflows/ci.yml
//  2. Use edit_file for Cargo.toml workspace config
//  3. Use mcp.github.create_pr to submit
//  Confidence: 0.85"
```

---

## Unified Tool Catalog

Every tool — regardless of source — normalizes into one format:

```rust
struct ToolDescriptor {
    tool_id: String,           // "toolbus.fs.edit_file", "mcp.github.create_pr"
    source: ToolSource,        // ToolBus | MCP(server_id) | ACP(agent_id)
    namespace: String,         // "fs", "github", "k8s"
    domain: String,            // "code", "repo", "infra"
    summary: String,           // 1-2 lines
    capabilities: Vec<String>, // ["read", "write", "exec"]
    schema_brief: String,      // ~30-80 tokens
    schema_full: JsonValue,    // ~200+ tokens
    tokens_brief: u32,
    tokens_full: u32,
}

enum ToolSource {
    ToolBus,
    Mcp { server_id: String, server_name: String },
    Acp { agent_id: String, agent_name: String },
}
```

### Namespacing

```
TOOLBUS (native)                MCP (servers)                    ACP (agents)
─────────────────               ──────────────────               ─────────────────
toolbus.fs.edit_file            mcp.github.create_pr             acp.reviewer.review_pr
toolbus.fs.create_file          mcp.github.search_issues         acp.deployer.deploy
toolbus.fs.glob                 mcp.slack.send_message           acp.tester.run_suite
toolbus.search.grep             mcp.postgres.query               acp.planner.create_plan
toolbus.exec.bash               mcp.s3.upload                    acp.monitor.check_health
```

### Domain Taxonomy

| Domain | Namespaces | Description |
|--------|-----------|-------------|
| `code` | `fs`, `search`, `exec`, `git` | Core coding operations |
| `repo` | `github`, `gitlab`, `bitbucket` | Repository platforms |
| `infra` | `k8s`, `docker`, `terraform` | Infrastructure |
| `data` | `postgres`, `redis`, `s3` | Data stores |
| `comms` | `slack`, `jira`, `email` | Communication |
| `agents` | `reviewer`, `deployer`, `tester` | ACP agent capabilities |

---

## Cold Start

When LocusGraph has no history, use keyword + repo signal heuristics:

```rust
fn cold_start_domains(user_message: &str, repo: &RepoContext) -> Vec<String> {
    let mut domains = Vec::new();
    let msg = user_message.to_lowercase();

    if msg.contains("pr") || msg.contains("github") { domains.push("github".into()); }
    if msg.contains("deploy") || msg.contains("k8s")  { domains.push("k8s".into()); }
    if msg.contains("test")                            { domains.push("testing".into()); }

    if repo.has_file("Dockerfile") { domains.push("docker".into()); }
    if repo.has_file("k8s/")       { domains.push("k8s".into()); }

    domains.push("code".into()); // always include core
    domains
}
```

---

## Safety Gates

```rust
enum ToolRisk {
    ReadOnly,       // grep, glob, list — always safe
    WriteLocal,     // edit_file, create_file — safe within sandbox
    WriteRemote,    // github.create_pr — needs full schema first
    Destructive,    // k8s.delete, db.drop — requires user confirmation
}
```

Rule: **`tool_explain(detail="full")` is required before executing any `WriteRemote` or `Destructive` tool.** Prevents hallucinated args on dangerous operations.

---

## Runtime Integration

### What Changes

The current runtime (`locus_runtime`) loads ALL tools into every LLM call:

```rust
// BEFORE — runtime.rs line 213
let tools = self.toolbus.list_tools();  // ALL tools, always
let system_prompt = context::build_system_prompt(&tools);
```

After tool discovery, this becomes:

```rust
// AFTER
let tier0 = self.tool_catalog.tier0();                          // core + meta-tools (fixed)
let tier1 = select_hot_tools(&self.locus_graph, &user_msg).await; // LocusGraph picks relevant
let tools = merge_dedup(tier0, tier1);                           // 12-15 tools total
let system_prompt = context::build_system_prompt(&tools);
```

### Files Changed

| File | Change |
|------|--------|
| `runtime.rs` | `list_tools()` → `select_hot_tools()` |
| `context.rs` | `build_system_prompt` accepts selected tools + always includes `tool_search`/`tool_explain` |
| `memory.rs` | `build_context_ids()` adds `"tool"` context type |
| `tool_handler.rs` | Routes to ToolBus / MCP / ACP based on `tool_id` prefix |
| `config.rs` | Add `tool_token_budget: u32` (default 3800) |

### Token Savings

| Scale | Without Discovery | With Discovery | Savings |
|---|---|---|---|
| 7 tools (current) | 1,400 | 1,400 | — |
| 50 tools | 10,000 | ~3,800 | 62% |
| 100 tools | 20,000 | ~3,800 | 81% |
| 300 tools | 60,000 | ~3,800 | 93.7% |

Budget stays **fixed at ~3,800 tokens** regardless of total tool count.

---

## Performance Budget

| Operation | Target | How |
|---|---|---|
| Tier 0 loading | 0ms | Static, compiled in |
| Tier 1 selection | <50ms | `retrieve_memories()` + local cache |
| `tool_search` | <80ms | Local BM25 + LocusGraph boost |
| `tool_explain(brief)` | <5ms | SQLite lookup |
| `tool_explain(full)` | <10ms | SQLite lookup |
| Total overhead | <100ms | Well under 200ms budget |

---

## Context IDs

| Pattern | Purpose |
|---------|---------|
| `tool:{name}` | Tool schema storage |
| `tool:{name}:usage` | Usage statistics and patterns |
| `terminal:{tool_name}` | Individual tool run results (existing) |
| `skill:tool_chain:{workflow}` | Learned multi-tool workflows |

---

## Implementation Checklist

### `locus_runtime`

- [ ] Before each LLM call: `select_hot_tools()` via LocusGraph
- [ ] Build prompt with Tier 0 + Tier 1 only
- [ ] Implement `tool_search` and `tool_explain` as meta-tools
- [ ] Route tool calls by source (ToolBus / MCP / ACP)
- [ ] Add `tool_token_budget` to `RuntimeConfig`

### `locus_graph`

- [ ] Add `register_tool_schema()` convenience hook
- [ ] Add `discover_tools()` method with tool context type filter
- [ ] Cache tool schemas locally (they don't change often)

### `locus_toolbus`

- [ ] No changes needed — ToolBus stays dumb, graph stays smart

### `locus_agents` (future)

- [ ] Subagents inherit parent's `graph_id` → share tool knowledge
- [ ] Subagents can specialize per domain

---

## Summary

| Concept | Description |
|---------|-------------|
| **Problem** | Too many tools → context window bloat |
| **Solution** | Three tiers: always-available + LocusGraph-selected + on-demand discovery |
| **Register** | `context_id: "tool:{name}"`, `source: "system"` |
| **Discover** | `retrieve_memories(user_message)` with tool context filter |
| **Meta-tools** | `tool_search` + `tool_explain` for Tier 2 access |
| **Learn** | `store_tool_run()` creates intent→tool→outcome links |
| **Plan** | `generate_insights()` for multi-step tool chains |
| **Cold start** | Keyword + repo signal heuristics |
| **Token budget** | Fixed ~3,800 regardless of total tool count |
