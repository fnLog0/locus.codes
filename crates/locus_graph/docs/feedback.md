# Feedback — Corrections and Guidance

## What is Feedback?

Feedback is information that **corrects or guides** the agent — user corrections, validation results, system signals. Feedback has high confidence because it comes from authoritative sources.

```
Agent: "I'll use unwrap() here"
User: "Don't use unwrap, use ? operator"
  → Stored as Feedback (user correction)
  → Agent learns: prefer ? over unwrap
```

---

## When to Store Feedback

| Trigger | Source | Example |
|---------|--------|---------|
| **User corrects agent** | `user` | "Don't do X, do Y instead" |
| **Validation fails** | `validator` | Lint error, type check failure |
| **Test feedback** | `executor` | Tests failed after change |
| **System rejection** | `system` | Permission denied, resource limit |

---

## Feedback Structure

### User Correction

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "user_correction",
        "data": {
            "what_agent_did": "Used unwrap() on Option",
            "correction": "Use ? operator for error propagation",
            "reason": "unwrap panics, ? propagates errors gracefully",
            "applies_to": ["error_handling", "rust_patterns"],
        }
    }),
)
.context_id("feedback:rust_patterns")
.source("user")  // High confidence (0.7)
```

### Validation Feedback

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation_failed",
        "data": {
            "check": "cargo clippy",
            "error": "uninlined_format_args",
            "message": "format string can be inlined",
            "file": "src/main.rs",
            "line": 42,
        }
    }),
)
.context_id("feedback:linting")
.source("validator")  // Highest confidence (0.9)
```

### Test Feedback

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "test_feedback",
        "data": {
            "test": "test_auth_flow",
            "result": "failed",
            "error_message": "assertion failed: token.is_valid()",
            "after_change": "src/auth.rs: Added timeout check",
        }
    }),
)
.context_id("feedback:testing")
.source("executor")
```

---

## Source Priority for Feedback

| Source | Confidence | Use Case |
|--------|------------|----------|
| `validator` | 0.9 | Runtime-verified, cannot be disputed |
| `executor` | 0.8 | Test results, tool outputs |
| `user` | 0.7 | Human correction, preference |
| `agent` | 0.6 | Self-correction |
| `system` | 0.5 | System-level constraints |

---

## Feedback Linking

### Contradicts — Reverses a Previous Approach

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "user_correction",
        "data": {
            "correction": "Use async instead of threads",
        }
    }),
)
.context_id("feedback")
.contradicts(vec!["decision:use_thread_pool".to_string()])
.source("user")
```

### Reinforces — Confirms Existing Knowledge

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation_passed",
        "data": {
            "check": "cargo test",
            "result": "passed",
        }
    }),
)
.context_id("feedback")
.reinforces(vec!["skill:test_driven_development".to_string()])
.source("validator")
```

---

## Feedback → Skill Pipeline

Feedback often leads to skill formation:

```
1. Agent makes mistake
2. User corrects (Feedback)
3. Agent applies correction successfully
4. Pattern validated → Store as Skill
```

```rust
// User corrected approach
// After successfully applying the correction:
client.store_skill(
    "rust_error_handling",
    "Use ? operator instead of unwrap for error propagation",
    vec![
        "Use ? to propagate errors up the call stack",
        "Only use unwrap in tests or when invariant is guaranteed",
        "Consider expect() with message for debugging",
    ],
    true,  // validated by user correction + successful application
);
```

---

## Retrieval

Feedback is retrieved when:
- Agent is about to make a similar choice
- Validation fails → recall past corrections
- Starting a task → recall relevant guidance

```rust
let context_ids = vec![
    "feedback",      // corrections and guidance
    "decisions",     // past decisions to avoid contradicting
    "skills",        // learned patterns
];
```

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `EventKind::Feedback` |
| **Context IDs** | `feedback:{category}`, `feedback:linting`, `feedback:testing` |
| **Source** | `validator` (0.9), `user` (0.7), `executor` (0.8) |
| **Retrieve** | Before making choices, after validation failures |
| **Links** | `contradicts` to reverse, `reinforces` to confirm |
