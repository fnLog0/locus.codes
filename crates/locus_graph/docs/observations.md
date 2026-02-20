# Observations — What the Agent Noticed

## What is an Observation?

An observation is something the agent **noticed but didn't cause** — errors, system state, test failures, environment details. Unlike actions, observations are "read-only" perceptions.

```
User: "run the tests"
  → Agent runs tests
  → Tests fail
  → Agent stores: Observation (test failure)
  → Agent stores: Action (test run with results)
```

---

## When to Store an Observation

| Trigger | Context ID | Example |
|---------|------------|---------|
| **Error occurred** | `errors` | Tool failed, LLM error |
| **Test failed** | `test:{file}` | `store_test_run` with failed > 0 |
| **Discovered state** | `project:{hash}` | "Project uses TypeScript 5.0" |
| **User intent** | `user_intent` | User's goal for the session |
| **Environment info** | `system` | OS, available tools, permissions |

---

## Observation Structure

### Error Observation

```rust
CreateEventRequest::new(
    EventKind::Observation,
    json!({
        "kind": "error",
        "data": {
            "context": "tool_bash",
            "error_message": "Permission denied",
            "command_or_file": "/etc/passwd",
        }
    }),
)
.context_id("errors")
.source("system")
```

### User Intent Observation

```rust
CreateEventRequest::new(
    EventKind::Observation,
    json!({
        "kind": "user_intent",
        "data": {
            "message_preview": "fix the login bug",
            "intent_summary": "User wants authentication fixed",
        }
    }),
)
.context_id("user_intent")
.source("user")
```

### Project State Observation

```rust
CreateEventRequest::new(
    EventKind::Observation,
    json!({
        "kind": "project_state",
        "data": {
            "aspect": "language_version",
            "value": "Rust 1.75",
            "how_discovered": "cargo --version",
        }
    }),
)
.context_id("project:abc123")
.source("agent")
```

---

## Source Priority for Observations

| Source | Confidence | Use Case |
|--------|------------|----------|
| `validator` | 0.9 | Runtime-verified fact |
| `executor` | 0.8 | Tool output, test result |
| `user` | 0.7 | User-stated information |
| `agent` | 0.6 | Agent inference |
| `system` | 0.5 | System-detected state |

---

## Observation vs Fact

| Type | Meaning | Permanence |
|------|---------|------------|
| **Observation** | Noticed now, may change | Ephemeral, situational |
| **Fact** | Established truth | Persistent, reusable |

Observations can become facts when validated:

```
Observation: "Tests seem to pass with --test-threads=1"
    ↓ (works consistently)
Fact: "This project requires --test-threads=1 for tests"
```

---

## Error Observations

Errors are a critical observation type. They're stored for:
- Pattern recognition ("this error keeps happening")
- Recovery strategies ("last time we fixed it by...")
- Avoiding repeated mistakes

```rust
// In tool_handler.rs
memory::store_error(
    locus_graph,
    format!("tool_{}", tool.name),
    e.to_string(),
    file_path.map(|s| s.to_string()),
);
```

---

## Retrieval

Observations are retrieved when:
- Agent encounters an error → recall similar errors
- Agent starts a task → recall user intent
- Agent needs context → recall project state

```rust
let context_ids = vec![
    "errors",       // past errors
    "user_intent",  // what user wanted
    "project:abc",  // project-specific observations
];
```

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `EventKind::Observation` |
| **Context IDs** | `errors`, `user_intent`, `project:{hash}`, `system` |
| **Source** | `system` (0.5), `agent` (0.6), `user` (0.7), `executor` (0.8) |
| **Retrieve** | Included for error recovery and context |
| **Links** | `related_to` for connected actions, `extends` for additional context |
