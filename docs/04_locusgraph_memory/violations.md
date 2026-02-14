# Constraint Violations

When an action violates a constraint, a violation event is stored.

## Storing a Violation

```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "observation".to_string(),
    context_id: Some("errors".to_string()),
    source: Some("coding_agent".to_string()),
    payload: serde_json::json!({
        "kind": "constraint_violation",
        "constraint": "Always run tests before committing",
        "action": "git_commit without running tests",
        "severity": "error",
        "blocked": true
    }),
    related_to: Some(vec!["constraints".to_string()]),
    extends: None,
    reinforces: None,
    contradicts: None,
    timestamp: None,
});
```

## Effects

| Effect | Description |
|--------|-------------|
| **Stored** | Violation recorded for future recall |
| **Linked** | `related_to` → `"constraints"` context |
| **Surfaced** | Shown to user in prompt bar or Diff Review |
| **Blocking** | `error` severity stops the action; `warning` allows with notice |

## Learning

Violations are recalled by `retrieve_memories` in future similar tasks — preventing the same mistake. The agent learns from its errors naturally.
