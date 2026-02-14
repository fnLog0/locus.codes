# Example: Constraints and Violations

---

## Storing Constraints

### Test Before Commit
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
    ..Default::default()
});
```

### No Force Push
```rust
payload: serde_json::json!({
    "kind": "constraint",
    "rule": "Never force-push to main or master",
    "scope": "global",
    "severity": "error"
})
```

### No Secrets in Code
```rust
payload: serde_json::json!({
    "kind": "constraint",
    "rule": "Never hardcode secrets or API keys in source code",
    "scope": "global",
    "severity": "error"
})
```

### Max File Size
```rust
payload: serde_json::json!({
    "kind": "constraint",
    "rule": "Do not create source files larger than 500 lines",
    "scope": "global",
    "severity": "warning"
})
```

---

## Violation Scenario

**Situation**: Agent attempts `git_commit` without running tests first.

### 1. ConstraintAgent Retrieves Constraints
```rust
let constraints = client.retrieve_memories(
    Some("coding-agent"),
    "constraints and rules",
    Some(20),
    Some(vec!["constraints".to_string()]),
    None,
)?;
// constraints.memories contains: "Always run tests before committing"
```

### 2. Violation Detected
ConstraintAgent checks proposed action against retrieved constraints → match found.

### 3. Action Blocked
```
⚠ Constraint violation: "Always run tests before committing"
  Action: git_commit — BLOCKED
  Run tests first? [y/n]
```

### 4. Violation Stored
```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "observation".to_string(),
    context_id: Some("errors".to_string()),
    source: Some("coding_agent".to_string()),
    payload: serde_json::json!({
        "kind": "constraint_violation",
        "constraint": "Always run tests before committing",
        "action": "git_commit without prior test run",
        "severity": "error",
        "blocked": true
    }),
    related_to: Some(vec!["constraints".to_string()]),
    ..Default::default()
});
```

### 5. Learning
Next retrieval for similar tasks will surface this violation — preventing the same mistake.
