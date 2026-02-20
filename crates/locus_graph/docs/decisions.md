# Decisions — Why the Agent Chose X

## What is a Decision?

A decision captures **the reasoning behind a choice** the agent made. Not just what happened, but why.

```
User: "add auth to the API"
  → Agent considers: JWT vs session vs OAuth
  → Agent decides: "Use JWT for stateless auth"
  → Stores: decision with reasoning
  → Next time: recalls why JWT was chosen
```

---

## When to Store a Decision

| Trigger | Example | Why Store |
|---------|---------|-----------|
| **Chose between options** | "Picked PostgreSQL over MongoDB" | Avoid re-litigating |
| **Made a trade-off** | "Speed over memory efficiency" | Context for future changes |
| **Followed a pattern** | "Used repository pattern for DB access" | Consistency |
| **User directed** | "User said prefer functional style" | Respect preferences |
| **After tool execution** | "Executed 3 tools to complete task" | Audit trail |

---

## Decision Structure

```rust
CreateEventRequest::new(
    EventKind::Decision,
    json!({
        "kind": "decision",
        "data": {
            "summary": "Use JWT tokens for API authentication",
            "reasoning": "Stateless, scalable, works with microservices",
            "alternatives_considered": ["session-based", "OAuth"],
            "trade_offs": "Cannot revoke instantly without blacklist",
        }
    }),
)
.context_id("decisions")
.source("agent")
```

### Minimal Decision

```rust
// After executing tools
let summary = format!("Executed {} tool(s)", results.len());
store_decision(summary, None);  // reasoning optional
```

---

## Source Priority for Decisions

| Source | Confidence | Use Case |
|--------|------------|----------|
| `validator` | 0.9 | Runtime-verified, authoritative |
| `user` | 0.7 | User-specified preference |
| `agent` | 0.6 | Agent's own reasoning |

---

## Decision Lifecycle

```
┌─────────────┐
│  1. CHOOSE   │  Agent faces a choice
└──────┬───────┘
       │
       ▼
┌─────────────┐
│  2. REASON   │  Agent considers options, trade-offs
└──────┬───────┘
       │
       ▼
┌─────────────┐
│  3. STORE    │  Stored as decision with reasoning
└──────┬───────┘
       │
       ├── works out ──► reinforce
       │
       └── causes issues ──► contradict, store new decision
```

---

## Updating Decisions

### Reinforce — Decision worked out

```rust
CreateEventRequest::new(
    EventKind::Action,
    json!({
        "kind": "decision_validated",
        "data": {
            "decision": "use_jwt_auth",
            "context": "Scaled to 10k users without issues",
        }
    }),
)
.context_id("decisions")
.reinforces(vec!["decision:use_jwt_auth".to_string()])
.source("executor")
```

### Contradict — Decision was wrong

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "decision_reversed",
        "data": {
            "old_decision": "use_jwt_auth",
            "reason": "Need instant revocation for security",
            "new_approach": "Session-based with Redis",
        }
    }),
)
.context_id("decisions")
.contradicts(vec!["decision:use_jwt_auth".to_string()])
.source("user")
```

---

## Retrieval

Decisions are always included in memory recall:

```rust
let context_ids = vec![
    "decisions",    // ← always include
    "errors",
    "project:abc",
];
```

The agent recalls decisions to:
- Maintain consistency across sessions
- Avoid re-deciding solved problems
- Understand why code is structured a certain way

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `EventKind::Decision` |
| **Context ID** | `decisions` |
| **Source** | `agent` (0.6), `user` (0.7), `validator` (0.9) |
| **Retrieve** | Always included in `build_context_ids()` |
| **Links** | `reinforces` when validated, `contradicts` when reversed |
