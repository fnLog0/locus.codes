# LocusGraph — Dynamic Tool Discovery (MCP & ACP)

Unlike static ToolBus tools (seeded once at cold start), MCP and ACP tools are **runtime-discovered**. Their schemas appear and disappear as servers connect and disconnect.

---

## Lifecycle

```
Server Connect    → Discover Tools → Store Events
Server Disconnect → contradicts (marks tools unavailable)
Server Reconnect  → Re-discover   → reinforces (confirms tools still valid)
                                   → contradicts (if schema changed)
```

### Why `contradicts` and `reinforces` matter here

Unlike static tools (same `context_id` = auto-override), dynamic tools need explicit link semantics because:

- **Disconnect** must signal that tools are **unavailable** even though the schema is still correct — `contradicts` marks them stale so the agent stops trying to call them.
- **Reconnect with same schema** should `reinforces` the original event — this tells LocusGraph the tool is confirmed working again with fresh evidence.
- **Reconnect with changed schema** should `contradicts` the old event — the schema has actually changed.

---

## MCP Tools

### On Server Start

When `McpManager.start_server(id)` succeeds, discover and store tools:

```rust
// After successful manager.start_server(&server_id)
let tools = manager.list_tools(&server_id).await?;
let server_config = manager.get_config(&server_id).unwrap();

// 1. Store server anchor
locus_graph.store_event(Event {
    context_id: format!("mcp:{}", server_id),
    event_kind: "fact",
    source: "validator",
    payload: json!({
        "server_id":   server_id,
        "server_name": server_config.name,
        "transport":   server_config.transport_type,  // "stdio" | "sse"
        "tool_count":  tools.len(),
        "status":      "connected"
    }),
    extends:    vec![format!("{}:tools", repo_hash)],
    related_to: vec![format!("knowledge:{}_{}", project_name, repo_hash)],
});

// 2. Store each tool
for tool in tools {
    locus_graph.store_event(Event {
        context_id: format!("mcp_tool:{}_{}", server_id, tool.name),
        event_kind: "fact",
        source: "validator",
        payload: json!({
            "name":        tool.name,
            "description": tool.description,
            "parameters":  tool.input_schema  // MCP inputSchema
        }),
        extends:    vec![format!("mcp:{}", server_id)],
        related_to: vec![format!("{}:tools", repo_hash)],
    });
}
```

### On Server Stop / Disconnect

When `McpManager.stop_server(id)` is called or connection drops, use `contradicts` to mark every tool from that server as unavailable:

```rust
// 1. Contradict the server anchor
locus_graph.store_event(Event {
    context_id:  format!("mcp:{}", server_id),
    event_kind:  "fact",
    source:      "validator",
    payload:     json!({ "status": "disconnected" }),
    contradicts: vec![format!("mcp:{}", server_id)],
});

// 2. Contradict each tool individually
for tool_name in &previously_discovered_tool_names {
    locus_graph.store_event(Event {
        context_id:  format!("mcp_tool:{}_{}", server_id, tool_name),
        event_kind:  "fact",
        source:      "validator",
        payload:     json!({ "status": "unavailable" }),
        contradicts: vec![format!("mcp_tool:{}_{}", server_id, tool_name)],
    });
}
```

### On Server Reconnect

Re-discover tools and compare schemas against what was stored before:

```rust
let new_tools = manager.list_tools(&server_id).await?;

// Reconnect the server anchor (reinforces — it's the same server, back online)
locus_graph.store_event(Event {
    context_id: format!("mcp:{}", server_id),
    event_kind: "fact",
    source: "validator",
    payload: json!({
        "server_id":  server_id,
        "tool_count": new_tools.len(),
        "status":     "connected"
    }),
    reinforces: vec![format!("mcp:{}", server_id)],
    extends:    vec![format!("{}:tools", repo_hash)],
});

for tool in new_tools {
    let tool_ctx = format!("mcp_tool:{}_{}", server_id, tool.name);

    if schema_unchanged(&tool, &previous_schema) {
        // Same schema → reinforces (confirms tool is still valid)
        locus_graph.store_event(Event {
            context_id: tool_ctx.clone(),
            event_kind: "fact",
            source: "validator",
            payload: json!({
                "name":        tool.name,
                "description": tool.description,
                "parameters":  tool.input_schema
            }),
            reinforces: vec![tool_ctx],
            extends:    vec![format!("mcp:{}", server_id)],
        });
    } else {
        // Schema changed → contradicts (old schema is wrong)
        locus_graph.store_event(Event {
            context_id: tool_ctx.clone(),
            event_kind: "fact",
            source: "validator",
            payload: json!({
                "name":        tool.name,
                "description": tool.description,
                "parameters":  tool.input_schema
            }),
            contradicts: vec![tool_ctx],
            extends:     vec![format!("mcp:{}", server_id)],
        });
    }
}
```

---

## ACP Tools (Future)

ACP follows the identical pattern. Substitute `mcp` → `acp` and `server` → `agent`:

| MCP | ACP |
|---|---|
| `mcp:{server_id}` | `acp:{agent_id}` |
| `mcp_tool:{server_id}_{tool_name}` | `acp_tool:{agent_id}_{tool_name}` |
| `McpManager.list_tools()` | ACP agent capability negotiation |
| `inputSchema` | ACP tool schema (TBD) |

```rust
// Pseudocode — same shape, different source
for tool in acp_agent.capabilities() {
    locus_graph.store_event(Event {
        context_id: format!("acp_tool:{}_{}", agent_id, tool.name),
        event_kind: "fact",
        source: "validator",
        payload: json!({
            "name":        tool.name,
            "description": tool.description,
            "parameters":  tool.schema
        }),
        extends:    vec![format!("acp:{}", agent_id)],
        related_to: vec![format!("{}:tools", repo_hash)],
    });
}
```

---

## Event Graph (Combined)

```
knowledge:{project_name}_{repo_hash}
  └── {repo_hash}:tools
        ├── tools:bash                              ← static (cold start)
        ├── tools:edit_file                         ← static
        ├── ...
        ├── meta:tool_search                        ← meta (Runtime)
        ├── meta:tool_explain
        ├── meta:task
        ├── mcp:filesystem-server                   ← MCP server anchor
        │     ├── mcp_tool:filesystem-server_read
        │     └── mcp_tool:filesystem-server_write
        ├── mcp:github-server
        │     ├── mcp_tool:github-server_search
        │     └── mcp_tool:github-server_pr_create
        └── acp:code-review-agent                   ← ACP agent anchor (future)
              └── acp_tool:code-review-agent_review
```

---

## Key Differences from Static Tools

| | Static (ToolBus) | Dynamic (MCP/ACP) |
|---|---|---|
| **When** | Cold start, once | Runtime, on connect/disconnect |
| **Source** | `ToolBus.list_tools()` | `McpManager.list_tools()` / ACP negotiation |
| **context_id** | `tools:{name}` | `mcp_tool:{server}_{name}` / `acp_tool:{agent}_{name}` |
| **Update** | Same `context_id` auto-overrides | `contradicts` (disconnect/schema change), `reinforces` (reconnect same schema) |
| **Anchor** | `{repo_hash}:tools` | `mcp:{server_id}` / `acp:{agent_id}` |

---

## Auto-Start Bootstrap

On cold start, after static tool seeding (see `tools.md`):

```rust
// Cold start sequence
bootstrap_static_tools(&toolbus, &locus_graph);  // tools.md Steps 1-3

// Then start configured MCP servers
let mcp_manager = McpManager::load(config_path).await?;
mcp_manager.auto_start().await?;

// Discover and store tools for each running server
for server_id in mcp_manager.list_running().await {
    bootstrap_mcp_tools(&server_id, &mcp_manager, &locus_graph).await?;
}
```
