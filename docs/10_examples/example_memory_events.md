# Example: Memory Events

Sample `store_event` calls showing different event kinds.

---

## 1. Terminal Command (action)

```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "action".to_string(),
    context_id: Some("terminal".to_string()),
    source: Some("coding_agent".to_string()),
    payload: serde_json::json!({
        "kind": "terminal_command",
        "cwd": "/project",
        "command": "cargo test",
        "exit_code": 0,
        "stdout_preview": "running 12 tests\ntest result: ok. 12 passed"
    }),
    related_to: None,
    extends: None,
    reinforces: None,
    contradicts: None,
    timestamp: None,
});
```

## 2. File Edit (fact)

```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "fact".to_string(),
    context_id: Some("editor".to_string()),
    source: Some("coding_agent".to_string()),
    payload: serde_json::json!({
        "kind": "file_edit",
        "path": "src/auth/login.rs",
        "summary": "Added token expiry validation to validate_token()",
        "diff_preview": "+ if self.is_expired(token) {"
    }),
    related_to: None,
    extends: None,
    reinforces: None,
    contradicts: None,
    timestamp: None,
});
```

## 3. User Intent (fact)

```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "fact".to_string(),
    context_id: Some("user_intent".to_string()),
    source: Some("user".to_string()),
    payload: serde_json::json!({
        "kind": "user_intent",
        "message_preview": "Fix the authentication bug in login.rs",
        "intent_summary": "Fix auth bug related to token validation"
    }),
    related_to: None,
    extends: None,
    reinforces: None,
    contradicts: None,
    timestamp: None,
});
```

## 4. Error (fact)

```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "fact".to_string(),
    context_id: Some("errors".to_string()),
    source: Some("coding_agent".to_string()),
    payload: serde_json::json!({
        "kind": "error",
        "context": "test_runner",
        "error_message": "test auth::test_expired_token FAILED: assertion failed",
        "command_or_file": "cargo test"
    }),
    related_to: Some(vec!["terminal".to_string()]),
    extends: None,
    reinforces: None,
    contradicts: None,
    timestamp: None,
});
```

## 5. Test Pass Reinforcing a Fix (observation)

```rust
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "observation".to_string(),
    context_id: Some("terminal".to_string()),
    source: Some("coding_agent".to_string()),
    payload: serde_json::json!({
        "kind": "test_result",
        "outcome": "pass",
        "summary": "All 12 tests pass after adding token expiry check"
    }),
    related_to: None,
    extends: None,
    reinforces: Some(vec!["editor".to_string()]),
    contradicts: None,
    timestamp: None,
});
```
