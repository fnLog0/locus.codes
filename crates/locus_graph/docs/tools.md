# LocusGraph — Tool Knowledge Bootstrap (Cold Start)

When the server cold-starts, LocusGraph must seed itself with baseline knowledge about the project and its available tools. This is a **one-time initialization** for static, code-defined tools.

---

## Prerequisites

- `repo_hash` — deterministic hash of the repository (e.g. git remote + root path).
- `project_name` — human-readable project name (ask the user if unknown).
- `ToolBus.list_tools()` — returns all registered tools with name, description, and parameter schema.

---

## Step 1 — Project Knowledge Anchor

Check if `knowledge:{project_name}_{repo_hash}` exists. If **not**, create it:

```json
{
  "context_id": "knowledge:{project_name}_{repo_hash}",
  "event_kind": "fact",
  "source": "validator",
  "payload": "<project description — ask the user if unavailable>"
}
```

> This is the root anchor. All other project knowledge extends from it.

---

## Step 2 — Tool Registry Master Event

Create a single master event that represents the tool registry as a whole:

```json
{
  "context_id": "{repo_hash}:tools",
  "event_kind": "fact",
  "source": "validator",
  "payload": {
    "tool_count": 11,
    "tool_names": ["bash", "create_file", "edit_file", "undo_edit",
                   "glob", "grep", "finder", "read",
                   "task_list", "handoff", "web_automation"]
  },
  "extends": ["knowledge:{project_name}_{repo_hash}"]
}
```

---

## Step 3 — Individual Tool Events

Iterate over `ToolBus.list_tools()` and create one fact event per tool. The `payload` is the tool's schema directly — no wrapper.

```rust
// Pseudocode — runs during cold-start bootstrap
let toolbus = ToolBus::new(repo_root);

for tool_info in toolbus.list_tools() {
    // tool_info: ToolInfo { name, description, parameters }

    locus_graph.store_event(Event {
        context_id: format!("tools:{}", tool_info.name),
        event_kind: "fact",
        source: "validator",
        payload: json!({
            "name":        tool_info.name,
            "description": tool_info.description,
            "parameters":  tool_info.parameters   // exact schema.json
        }),
        extends:    vec![format!("{}:tools", repo_hash)],
        related_to: vec![format!("knowledge:{}_{}", project_name, repo_hash)],
    });
}
```

**Example output** (for `bash`):

```json
{
  "context_id": "tools:bash",
  "event_kind": "fact",
  "source": "validator",
  "payload": {
    "name": "bash",
    "description": "Executes the given shell command using bash (or sh on systems without bash)",
    "parameters": {
      "type": "object",
      "properties": {
        "command": { "type": "string", "description": "The shell command to execute" },
        "timeout": { "type": "integer", "description": "Timeout in seconds (default: 60)", "default": 60 },
        "working_dir": { "type": "string", "description": "Working directory for the command (optional)" }
      },
      "required": ["command"]
    }
  },
  "extends": ["{repo_hash}:tools"],
  "related_to": ["knowledge:{project_name}_{repo_hash}"]
}
```

---

## Step 4 — Meta-Tool Events

Meta-tools live in `locus_runtime::context` (not ToolBus). They are the agent's self-discovery and delegation layer. Seed them from `meta_tool_definitions()`:

```rust
// From locus_runtime::context::meta_tool_definitions()
let meta_tools = meta_tool_definitions();  // tool_search, tool_explain, task

for tool_info in meta_tools {
    locus_graph.store_event(Event {
        context_id: format!("meta:{}", tool_info.name),
        event_kind: "fact",
        source: "validator",
        payload: json!({
            "name":        tool_info.name,
            "description": tool_info.description,
            "parameters":  tool_info.parameters
        }),
        extends:    vec![format!("{}:tools", repo_hash)],
        related_to: vec![format!("knowledge:{}_{}", project_name, repo_hash)],
    });
}
```

These three tools:

| Tool | Purpose |
|---|---|
| `tool_search` | Search available tools by natural language query |
| `tool_explain` | Get full schema for a tool before calling it |
| `task` | Run a sub-task in a separate agent (parallel execution) |

---

## Event Graph

```
knowledge:{project_name}_{repo_hash}       ← project root anchor
  └── {repo_hash}:tools                    ← tool registry master
        ├── tools:bash                     ← static (ToolBus)
        ├── tools:create_file
        ├── tools:edit_file
        ├── tools:undo_edit
        ├── tools:glob
        ├── tools:grep
        ├── tools:finder
        ├── tools:read
        ├── tools:task_list
        ├── tools:handoff
        ├── tools:web_automation
        ├── meta:tool_search               ← meta (Runtime)
        ├── meta:tool_explain
        └── meta:task
```

---

## When to Run

| Condition | Action |
|---|---|
| `knowledge:{project_name}_{repo_hash}` missing | Run Steps 1 → 2 → 3 → 4 |
| Anchor exists, `{repo_hash}:tools` missing | Run Steps 2 → 3 → 4 |
| New tool added to ToolBus | Run Step 3 for the new tool only |
| Tool schema changed | Re-run Step 3 for that tool — same `context_id` overrides the old payload |
| **locus.codes binary version changed** | Re-run Steps 2 → 3 → 4. Since every event uses the same `context_id`, LocusGraph automatically overrides the previous payload — no `contradicts` needed. |

### Version-Aware Bootstrap

The `{repo_hash}:tools` master event should include the locus.codes version:

```json
{
  "context_id": "{repo_hash}:tools",
  "event_kind": "fact",
  "source": "validator",
  "payload": {
    "tool_count": 14,
    "tool_names": ["bash", "create_file", "..."],
    "locus_version": "0.1.0"
  },
  "extends": ["knowledge:{project_name}_{repo_hash}"]
}
```

On cold start, compare `locus_version` in the stored event against the running binary version. If they differ, simply re-run Steps 2 → 3 → 4. Each `store_event` call uses the same `context_id` (e.g. `tools:bash`, `meta:task`), so LocusGraph replaces the old payload with the new one automatically. No explicit invalidation required.

---

## Notes

- These events cover **static tools** (compiled into the ToolBus crate). Dynamic/MCP tools will have a separate bootstrap path.
- `event_kind` is `fact` (not `knowledge`) — matching the LocusGraph enum: `fact | action | decision | observation | feedback`.
- `source: "validator"` signals these were machine-verified, not user-provided.
