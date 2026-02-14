# Context IDs

## What They Are

A `context_id` is a string label used to scope and group events in LocusGraph. It allows filtered retrieval — e.g. "only recall terminal commands" or "only recall editor changes."

## Format

Plain string labels. Use short, stable names:

```
"terminal"
"editor"
"user_intent"
"errors"
"project"
"constraints"
```

## Usage

### Storing

Pass `context_id` when storing an event:

```rust
CreateEventRequest {
    context_id: Some("terminal".to_string()),
    ...
}
```

### Retrieving

Filter retrieval by `context_ids` or `context_types`:

```rust
client.retrieve_memories(
    Some("coding-agent"),
    "recent terminal commands",
    Some(5),
    Some(vec!["terminal".to_string()]),  // only terminal events
    None,
)
```

### Listing

Discover what context types and IDs exist:

```rust
client.list_context_types(Some("coding-agent"), None, None)
client.list_contexts_by_type(Some("coding-agent"), "terminal", None, None)
client.search_contexts(Some("coding-agent"), "auth", None, None, None, None)
```

## Conventions for locus.codes

| context_id | Stored By | Contains |
|------------|-----------|----------|
| `terminal` | ToolBus (after run_cmd) | Commands, stdout, stderr, exit codes |
| `editor` | ToolBus (after file_write) | File paths, edit summaries, diff previews |
| `user_intent` | Orchestrator (on prompt) | User messages, intent summaries |
| `errors` | ToolBus / agents | Error messages, failure context |
| `project` | Session Manager | Project-level facts (language, framework, structure) |
| `constraints` | ConstraintAgent | Rules the agent must follow |
