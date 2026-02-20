# Actions — What the Agent Did

## What is an Action?

An action is an **event that the agent performed** — tool calls, file edits, commands run, LLM calls. Actions are the "doing" part of the agent loop.

```
User: "fix the bug"
  → Agent runs grep to find the bug    → Action
  → Agent edits the file               → Action
  → Agent runs tests                   → Action
  → Tests pass                         → Action (success)
```

---

## When to Store an Action

| Trigger | Context ID | Example |
|---------|------------|---------|
| **Tool executed** | `terminal:{tool_name}` | `store_tool_run("bash", args, result)` |
| **File edited** | `editor:{path}` | `store_file_edit("src/main.rs", "fix", None)` |
| **LLM called** | `llm:{model}` | `store_llm_call("claude-3", 100, 50, 2000, false)` |
| **Tests run** | `test:{file}` | `store_test_run("tests/api.rs", 10, 0, 5000)` |
| **Git operation** | `git:{repo_hash}` | `store_git_op(repo, "commit", "fix bug", false)` |

---

## Action Structure

```rust
CreateEventRequest::new(
    EventKind::Action,
    json!({
        "kind": "tool_run",
        "data": {
            "tool": "bash",
            "args": {"command": "cargo test"},
            "result_preview": {...},
            "duration_ms": 5432,
            "is_error": false,
        }
    }),
)
.context_id("terminal:bash")
.source("executor")
```

---

## Source Priority for Actions

| Source | Confidence | Use Case |
|--------|------------|----------|
| `executor` | 0.8 | Tool ran successfully, file written |
| `system` | 0.5 | System-triggered action |

---

## Retrieval

Actions are retrieved when:
- Agent needs to recall "what did I do last time?"
- Building context for a similar task
- Debugging "what went wrong?"

```rust
let context_ids = vec![
    "terminal",     // tool executions
    "editor",       // file changes
    "project:abc",  // project-specific actions
];
```

---

## Action vs Observation

| Type | Meaning | Example |
|------|---------|---------|
| **Action** | Agent did something successfully | Tool ran, file created |
| **Observation** | Agent noticed something (may be failure) | Error occurred, test failed |

When an action fails, store it as `EventKind::Observation` with `is_error: true`.

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `EventKind::Action` |
| **Context IDs** | `terminal`, `editor`, `llm`, `test`, `git` |
| **Source** | `executor` (0.8), `system` (0.5) |
| **Retrieve** | Included in memory recall for task context |
| **Links** | `related_to` for connected decisions, `reinforces` for repeated patterns |
