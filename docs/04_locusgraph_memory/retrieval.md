# Retrieval

How memories are found and returned from LocusGraph.

## API

```rust
client.retrieve_memories(
    graph_id: Option<&str>,    // e.g. Some("coding-agent")
    query: &str,               // natural language query
    limit: Option<u64>,        // max results (default: 5)
    context_ids: Option<Vec<String>>,   // filter by context_id
    context_types: Option<...>,         // filter by context type
)
```

## Response

```rust
ContextResult {
    memories: String,      // formatted string of relevant memories
    items_found: u64,      // number of items matched
}
```

The `memories` string is ready to inject into the LLM prompt — no parsing needed.

## When to Retrieve

| Scenario | Query Example | Filters |
|----------|--------------|---------|
| Before planning a task | `"recent changes and project context"` | None |
| Before running a command | `"previous terminal commands and errors"` | `context_ids: ["terminal"]` |
| Before editing a file | `"recent edits and conventions for this codebase"` | `context_ids: ["editor"]` |
| User says "like last time" | `"how we did X"` | None |
| Checking constraints | `"active constraints and rules"` | `context_ids: ["constraints"]` |

## Limits per Mode

| Mode | limit |
|------|-------|
| Rush | 5 |
| Smart | 10 |
| Deep | 20 |

## Context Filtering

```rust
// Only terminal events
context_ids: Some(vec!["terminal".to_string()])

// Only errors and terminal
context_ids: Some(vec!["errors".to_string(), "terminal".to_string()])
```
