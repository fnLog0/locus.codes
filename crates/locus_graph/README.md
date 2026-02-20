# locus_graph

LocusGraph SDK for Rust — implicit memory layer for coding agents.

## Overview

One `graph_id`, one brain — all sessions read/write to the same graph. The agent learns conventions from actions, no manual AGENT.md files needed.

### Key Features

- **Prevent hallucination** — retrieve relevant memories before every LLM call
- **Persistence** — every tool call, file edit, user intent, and error becomes a memory
- **Learning** — the AI improves across sessions by recalling past context
- **Cross-session** — start a new session, still remember project patterns
- **Semantic recall** — "how do we handle auth?" → relevant memories injected

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
locus_graph = { path = "../locus_graph" }
```

## Quick Start

```rust
use locus_graph::{LocusGraphClient, LocusGraphConfig, CreateEventRequest, EventKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client from environment (LOCUSGRAPH_AGENT_SECRET required)
    let config = LocusGraphConfig::from_env()?;
    let client = LocusGraphClient::new(config).await?;

    // Or with explicit config:
    let config = LocusGraphConfig::new(
        "http://localhost:50051",
        "your-agent-secret",
        "your-graph-id"
    )
    .cache_reads(true)
    .queue_stores(true);

    // Store a memory (fire-and-forget)
    let event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "technical_fact",
            "data": {
                "topic": "auth",
                "value": "we use JWT tokens"
            }
        })
    )
    .context_id("fact:auth")
    .source("agent");
    client.store_event(event).await;

    // Retrieve memories before LLM call
    let result = client.retrieve_memories("how do we handle auth?", None).await?;
    println!("Found {} memories", result.items_found);

    Ok(())
}
```

## Configuration

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `LOCUSGRAPH_AGENT_SECRET` | Yes | - | Agent secret token |
| `LOCUSGRAPH_SERVER_URL` | No | `http://127.0.0.1:50051` | gRPC server endpoint |
| `LOCUSGRAPH_GRAPH_ID` | No | `locus-agent` | Graph ID for memory storage |

### Programmatic Configuration

```rust
let config = LocusGraphConfig::new(
    "http://localhost:50051",  // gRPC endpoint
    "your-agent-secret",        // agent secret
    "your-graph-id"             // graph ID
)
.db_path(PathBuf::from("/path/to/cache.db"))  // SQLite cache location
.cache_reads(true)   // Enable read caching
.queue_stores(true); // Enable background write queueing
```

## API Reference

### Client

```rust
// Create client
let client = LocusGraphClient::new(config).await?;

// Store event (fire-and-forget)
client.store_event(event).await;

// Store event with result
let event_id = client.store_event_result(event).await?;

// Retrieve memories
let result = client.retrieve_memories("query", None).await?;

// Retrieve with options
let options = RetrieveOptions::new()
    .limit(10)
    .context_id("fact:auth");
let result = client.retrieve_memories("query", Some(options)).await?;

// Generate insights
let insight = client.generate_insights("task description", None).await?;

// List context types
let types = client.list_context_types(None, None).await?;

// List contexts by type
let contexts = client.list_contexts_by_type("fact", None, None).await?;

// Search contexts
let contexts = client.search_contexts("auth", None, None, None).await?;
```

### Event Kinds

| Kind | Description |
|------|-------------|
| `Fact` | A factual piece of information |
| `Action` | An action that was taken |
| `Decision` | A decision that was made |
| `Observation` | An observation from the system |
| `Feedback` | Feedback from user or system |

### Creating Events

```rust
let event = CreateEventRequest::new(EventKind::Fact, json!({
    "kind": "technical_fact",
    "data": { "topic": "auth", "value": "we use JWT tokens" }
}))
.context_id("fact:auth")           // Optional: group related events
.source("agent")                   // Required: validator, executor, user, agent, or system
.related_to(vec!["fact:api".into()]) // Optional: link to other contexts
.extends(vec!["fact:base".into()])   // Optional: this extends another context
.reinforces(vec!["fact:x".into()])   // Optional: supports another context
.contradicts(vec!["fact:y".into()]); // Optional: contradicts another context

client.store_event(event).await;
```

## Convenience Hooks

Pre-built methods for common use cases:

### Store Tool Run

```rust
// After executing a tool
client.store_tool_run(
    "bash",
    &json!({"command": "cargo build"}),
    &json!({"exit_code": 0, "output": "Build succeeded"}),
    1500,  // duration_ms
    false  // is_error
).await;
```

### Store File Edit

```rust
// After editing a file
client.store_file_edit(
    "src/main.rs",
    "Added new function for processing",
    Some("@@ -1,3 +1,5 @@\n+fn new_function() {}")
).await;
```

### Store User Intent

```rust
// When user sends a message
client.store_user_intent(
    "Please add error handling to the login function",
    "Add error handling to login"
).await;
```

### Store Error

```rust
// On any error
client.store_error(
    "tool_execution",
    "Command timed out after 30 seconds",
    Some("cargo test -- --nocapture")
).await;
```

### Store Decision

```rust
// After LLM responds with a decision
client.store_decision(
    "Use SQLite for local caching",
    Some("SQLite provides good performance for local operations")
).await;
```

### Store Project Convention

```rust
// When agent discovers project conventions
client.store_project_convention(
    "locuscodes",
    "Use snake_case for function names",
    vec!["fn process_data() {}", "fn calculate_total() {}"]
).await;
```

### Store Skill

```rust
// When a pattern is validated
client.store_skill(
    "error_recovery",
    "Standard pattern for recovering from errors",
    vec![
        "Log the error with context",
        "Check if error is recoverable",
        "Retry with exponential backoff",
        "Fall back to alternative approach if retry fails"
    ],
    true  // validated
).await;
```

## Testing

Integration tests require a running LocusGraph gRPC server:

```bash
# Run integration tests
cargo test -p locus-graph --test integration -- --test-threads=1 --nocapture

# Run hook tests
cargo test -p locus-graph --test hooks -- --test-threads=1 --nocapture
```

## Architecture

```
locus_graph/
├── src/
│   ├── lib.rs      # Crate root, re-exports
│   ├── client.rs   # LocusGraphClient implementation
│   ├── config.rs   # Configuration types
│   ├── types.rs    # Event, result, and option types
│   ├── hooks.rs    # Convenience methods for common events
│   └── error.rs    # Error types
└── tests/
    ├── common/mod.rs    # Test utilities
    ├── integration.rs   # Full API integration tests
    └── hooks.rs         # Hook convenience tests
```

## Dependencies

- `locus-proxy` — gRPC client for LocusGraph server
- `tokio` — Async runtime
- `serde` / `serde_json` — JSON serialization
- `tracing` — Logging
- `anyhow` / `thiserror` — Error handling
