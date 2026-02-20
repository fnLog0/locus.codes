# Technical Facts — Established Knowledge

## What is a Technical Fact?

A technical fact is **verified, reusable knowledge** about the codebase, architecture, or domain. Unlike observations (ephemeral), facts are persistent and authoritative.

```
Agent discovers: "API uses JWT tokens"
  → Validates by reading code
  → Stores as Fact
  → Future sessions: "How does auth work?" → Recalls fact
```

---

## When to Store a Fact

| Trigger | Context ID | Example |
|---------|------------|---------|
| **Discovered convention** | `project:{hash}` | "Tests go in tests/ directory" |
| **Architecture decision** | `fact:architecture` | "Microservices communicate via gRPC" |
| **API knowledge** | `fact:api` | "Rate limit: 100 req/min per user" |
| **Discovered pattern** | `skill:{name}` | "Error handling uses anyhow in binaries" |
| **Validated observation** | `fact:{topic}` | Observation confirmed → Fact |

---

## Fact Structure

### Project Convention

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "project_convention",
        "data": {
            "topic": "testing",
            "value": "All tests use --test-threads=1 due to shared state",
            "discovered_in": "session:abc123",
            "validated": true,
        }
    }),
)
.context_id("project:xyz789")
.source("agent")
```

### Technical Fact

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "technical_fact",
        "data": {
            "topic": "database",
            "value": "PostgreSQL with connection pooling (max 10)",
            "evidence": ["config/database.yml", "lib/db/pool.rb"],
        }
    }),
)
.context_id("fact:database")
.source("validator")  // High confidence - verified in code
```

### API Specification

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "api_spec",
        "data": {
            "endpoint": "/api/v1/users",
            "method": "POST",
            "auth": "required",
            "rate_limit": "100/minute",
            "fields": ["email", "name", "role"],
        }
    }),
)
.context_id("fact:api:users")
.source("executor")
```

---

## Source Priority for Facts

| Source | Confidence | Use Case |
|--------|------------|----------|
| `validator` | 0.9 | Code-verified, runtime-confirmed |
| `executor` | 0.8 | Tool-verified (tests pass, lints clean) |
| `user` | 0.7 | User-stated, trusted |
| `agent` | 0.6 | Agent-discovered, needs validation |
| `system` | 0.5 | System-detected |

---

## Observation → Fact Pipeline

Facts often start as observations:

```
1. Agent observes pattern
2. Agent validates through code/tests
3. Pattern confirmed → Store as Fact
```

```rust
// Observation
CreateEventRequest::new(
    EventKind::Observation,
    json!({
        "kind": "pattern_observed",
        "data": {
            "pattern": "All handlers return Result<T, ApiError>",
            "files_checked": ["src/handlers/*.rs"],
        }
    }),
)
.context_id("project:abc")
.source("agent")

// After validation across codebase → becomes Fact
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "project_convention",
        "data": {
            "topic": "error_handling",
            "value": "All handlers return Result<T, ApiError>",
            "validated": true,
        }
    }),
)
.context_id("project:abc")
.source("validator")  // Upgraded confidence
```

---

## Fact Linking

### Extends — Adds Detail to Existing Fact

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "technical_fact",
        "data": {
            "topic": "auth",
            "value": "JWT tokens expire after 24 hours",
        }
    }),
)
.context_id("fact:auth")
.extends(vec!["fact:api_auth".to_string()])
.source("validator")
```

### Contradicts — Replaces Outdated Fact

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "technical_fact",
        "data": {
            "topic": "auth",
            "value": "JWT tokens now expire after 1 hour (security update)",
            "reason": "Changed in commit abc123",
        }
    }),
)
.context_id("fact:auth")
.contradicts(vec!["fact:jwt_expiry_24h".to_string()])
.source("validator")
```

---

## Retrieval

Facts are retrieved for:
- "How does X work?" queries
- Building context for tasks
- Avoiding repeated discovery

```rust
let context_ids = vec![
    "fact:auth",      // auth facts
    "fact:api",       // API facts
    "project:abc",    // project conventions
];
```

---

## Fact Categories

### Architecture Facts
```
fact:architecture
  - "System uses event sourcing"
  - "Services communicate via message queue"
  - "Database is PostgreSQL with read replicas"
```

### API Facts
```
fact:api
  - Endpoint specifications
  - Rate limits
  - Authentication requirements
```

### Project Facts
```
project:{hash}
  - Testing conventions
  - Code style rules
  - Build requirements
```

### Domain Facts
```
fact:domain
  - Business rules
  - Data constraints
  - Workflow sequences
```

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `EventKind::Fact` |
| **Context IDs** | `fact:{topic}`, `project:{hash}`, `skill:{name}` |
| **Source** | `validator` (0.9), `executor` (0.8), `user` (0.7) |
| **Retrieve** | For "how does X work?" queries |
| **Links** | `extends` to detail, `contradicts` to update |
