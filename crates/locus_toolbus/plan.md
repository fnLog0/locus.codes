# locus_toolbus — Plan

No new tools to add. ToolBus is stable and complete for the current phase.

**Scope**: Status doc only. No code changes.

---

## Current Tools (11 registered)

| # | Tool | Directory | Category |
|---|------|-----------|----------|
| 1 | `bash` | `tools/bash/` | exec |
| 2 | `create_file` | `tools/create_file/` | file write |
| 3 | `edit_file` | `tools/edit_file/` | file write |
| 4 | `undo_edit` | `tools/undo_edit/` | file write |
| 5 | `glob` | `tools/glob/` | search |
| 6 | `grep` | `tools/grep/` | search |
| 7 | `finder` | `tools/finder/` | search |
| 8 | `read` | `tools/read/` | file read |
| 9 | `task_list` | `tools/task_list/` | planning |
| 10 | `handoff` | `tools/handoff/` | agent |
| 11 | `web_automation` | `tools/web_automation/` | web |

## Meta-Tools (NOT in ToolBus — handled by runtime)

These tools are visible to the LLM but intercepted in `locus_runtime/tool_handler.rs` before reaching ToolBus:

| Tool | Handled in | Why not ToolBus |
|------|-----------|-----------------|
| `tool_search` | `locus_runtime` | Queries LocusGraph, not filesystem |
| `tool_explain` | `locus_runtime` | Returns cached schema, no execution |
| `task` | `locus_runtime` | Spawns sub-runtime, not a tool call |

## Stub Directories

| Directory | Status | Purpose |
|-----------|--------|---------|
| `src/mcp/` | Empty | Future: MCP client for external tool servers |
| `src/acp/` | Empty | Future: ACP client for agent-to-agent tools |

MCP and ACP tools will NOT be registered in `register_defaults()`. They will be dynamically registered at runtime when servers connect, and their schemas stored in LocusGraph for discovery (see `crates/locus_graph/docs/tool-discovery.md`).

---

## What ToolBus Does NOT Do

ToolBus is intentionally simple — a registry + dispatcher. It does NOT:

- **Select tools** — that's LocusGraph's job (tool discovery)
- **Learn from usage** — that's LocusGraph's job (store_tool_run)
- **Manage sub-agents** — that's the runtime's job (task tool)
- **Route to MCP/ACP** — future runtime responsibility
- **Enforce permissions** — future sandbox layer

ToolBus stays dumb. Graph stays smart.

---

## Future: MCP/ACP Integration

When MCP/ACP are implemented, ToolBus will gain a `register_external()` method:

```rust
// Future — not implemented yet
impl ToolBus {
    /// Register a tool backed by an external MCP/ACP server.
    pub fn register_external(&mut self, tool: ExternalTool) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }
}
```

External tools implement the same `Tool` trait — ToolBus doesn't care where execution happens. The `execute()` method internally routes to MCP/ACP transport.

---

## No Changes Needed

For the tool discovery and task tool plans:
- `crates/locus_graph/plan.md` — no ToolBus changes
- `crates/locus_runtime/plan.md` — no ToolBus changes
- `crates/locus_runtime/task-tool-plan.md` — no ToolBus changes

ToolBus `list_tools()` and `call()` APIs are stable. Everything builds on top.
