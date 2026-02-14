# Constraints

Constraints are events stored with `context_id: "constraints"` that define rules the agent must follow.

## Storing a Constraint

```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "fact".to_string(),
    context_id: Some("constraints".to_string()),
    source: Some("user".to_string()),
    payload: serde_json::json!({
        "kind": "constraint",
        "rule": "Always run tests before committing",
        "scope": "global",
        "severity": "error"
    }),
    related_to: None,
    extends: None,
    reinforces: None,
    contradicts: None,
    timestamp: None,
});
```

## Retrieving Active Constraints

```rust
let constraints = client.retrieve_memories(
    Some("coding-agent"),
    "active constraints and rules",
    Some(20),
    Some(vec!["constraints".to_string()]),
    None,
)?;
// constraints.memories → string with all active rules
```

## Examples

| Rule | Scope | Severity |
|------|-------|----------|
| Always run tests before committing | global | error |
| Never force-push to main/master | global | error |
| Do not create files larger than 500 lines | global | warning |
| Never hardcode secrets in source code | global | error |

## Enforcement

1. ConstraintAgent calls `retrieve_memories` with `context_ids: ["constraints"]`
2. Constraints returned as context string
3. Proposed actions checked against constraint rules
4. Violations → stored as observation events, action blocked if severity is `error`
