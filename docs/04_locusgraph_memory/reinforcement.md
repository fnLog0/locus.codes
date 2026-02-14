# Reinforcement

Reinforcement is how skills form naturally — not from static files, but from accumulated positive outcomes.

## How It Works

1. Agent takes an action (e.g. fixes a bug using a certain pattern)
2. Tests pass → success observation stored with `reinforces` linking to the approach
3. LocusGraph server-side ranking boosts reinforced events
4. Next time a similar task appears, the approach surfaces higher in retrieval

## Storing Reinforcement

```rust
// After tests pass following a fix
client.store_event(CreateEventRequest {
    graph_id: "coding-agent".to_string(),
    event_kind: "observation".to_string(),
    context_id: Some("editor".to_string()),
    source: Some("coding_agent".to_string()),
    payload: serde_json::json!({
        "kind": "test_result",
        "outcome": "pass",
        "summary": "All 12 tests pass after adding token expiry check"
    }),
    related_to: None,
    extends: None,
    reinforces: Some(vec!["editor".to_string()]),  // reinforces the edit
    contradicts: None,
    timestamp: None,
});
```

## What Reinforcement Replaces

| Traditional Agents | locus.codes |
|-------------------|-------------|
| Skills written as static files | Skills emerge from repeated reinforcement |
| AGENTS.md with hardcoded patterns | Patterns surface naturally via retrieval ranking |
| Human writes instructions | Agent develops them through experience |

## Contradiction (Negative Signal)

When an approach fails, store with `contradicts` — the server-side ranking lowers those events in future retrieval:

```rust
contradicts: Some(vec!["editor".to_string()])
```
