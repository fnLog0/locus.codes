# Outcomes — Task Results

## What is an Outcome?

An outcome is the **final result of a task or workflow** — whether it succeeded, failed, or was abandoned. Outcomes capture the "so what" of agent work.

```
User: "fix the login bug"
  → Agent attempts fix
  → Tests pass
  → Outcome: SUCCESS (bug fixed, tests green)
```

---

## When to Store an Outcome

| Trigger | Context ID | Example |
|---------|------------|---------|
| **Task completed** | `outcome:task` | Bug fixed, feature added |
| **Task failed** | `outcome:task` | Could not resolve issue |
| **Task abandoned** | `outcome:task` | User cancelled, blocked |
| **Milestone reached** | `outcome:milestone` | Phase 1 complete |
| **Session ended** | `outcome:session` | Summary of session work |

---

## Outcome Structure

### Task Success

```rust
CreateEventRequest::new(
    EventKind::Action,
    json!({
        "kind": "outcome",
        "data": {
            "task": "fix_login_bug",
            "result": "success",
            "summary": "Fixed null pointer exception in auth flow",
            "actions_taken": [
                "Identified bug in src/auth.rs:42",
                "Added null check before token validation",
                "Added test case for null token",
            ],
            "files_changed": ["src/auth.rs", "tests/auth_test.rs"],
            "tests_passed": 47,
            "tests_failed": 0,
            "duration_ms": 120000,
        }
    }),
)
.context_id("outcome:task")
.related_to(vec!["session:abc123".to_string()])
.source("executor")
```

### Task Failure

```rust
CreateEventRequest::new(
    EventKind::Observation,
    json!({
        "kind": "outcome",
        "data": {
            "task": "optimize_database",
            "result": "failed",
            "summary": "Could not reduce query time below threshold",
            "attempts": [
                "Added index on user_id - 15% improvement",
                "Denormalized frequently joined tables - 10% improvement",
                "Query still takes 2.5s (target: 500ms)",
            ],
            "blockers": [
                "Database schema constraints prevent further optimization",
                "Would require architectural changes",
            ],
            "suggestions": [
                "Consider read replicas",
                "Implement caching layer",
            ],
        }
    }),
)
.context_id("outcome:task")
.related_to(vec!["session:abc123".to_string()])
.source("agent")
```

### Task Abandoned

```rust
CreateEventRequest::new(
    EventKind::Observation,
    json!({
        "kind": "outcome",
        "data": {
            "task": "migrate_to_typescript",
            "result": "abandoned",
            "summary": "Migration paused due to time constraints",
            "progress": "45%",
            "completed": [
                "Converted 12/27 files",
                "Set up tsconfig",
            ],
            "remaining": [
                "15 files remaining",
                "Update build pipeline",
            ],
            "reason": "User requested to pause for higher priority task",
            "resumable": true,
        }
    }),
)
.context_id("outcome:task")
.related_to(vec!["session:abc123".to_string()])
.source("user")
```

---

## Outcome Results

| Result | Meaning | Event Kind |
|--------|---------|------------|
| `success` | Task completed successfully | `Action` |
| `failed` | Task could not be completed | `Observation` |
| `abandoned` | Task stopped before completion | `Observation` |
| `partial` | Task partially completed | `Action` or `Observation` |

---

## Source Priority for Outcomes

| Source | Confidence | Use Case |
|--------|------------|----------|
| `validator` | 0.9 | Runtime-verified success/failure |
| `executor` | 0.8 | Tool-confirmed completion |
| `user` | 0.7 | User-confirmed outcome |
| `agent` | 0.6 | Agent-reported outcome |

---

## Session Outcomes

At session end, store a summary:

```rust
CreateEventRequest::new(
    EventKind::Action,
    json!({
        "kind": "session_outcome",
        "data": {
            "session_id": "abc123",
            "duration_ms": 1800000,  // 30 minutes
            "tasks_completed": 3,
            "tasks_failed": 1,
            "files_changed": 7,
            "tests_added": 5,
            "summary": "Fixed auth bug, added rate limiting, started DB optimization",
            "outcomes": [
                {"task": "fix_login_bug", "result": "success"},
                {"task": "add_rate_limiting", "result": "success"},
                {"task": "optimize_database", "result": "failed"},
            ],
        }
    }),
)
.context_id("outcome:session")
.related_to(vec!["session:abc123".to_string()])
.source("executor")
```

---

## Outcome Linking

### Related to Decisions

```rust
CreateEventRequest::new(
    EventKind::Action,
    json!({
        "kind": "outcome",
        "data": {
            "task": "implement_caching",
            "result": "success",
        }
    }),
)
.context_id("outcome:task")
.related_to(vec![
    "decision:use_redis_cache".to_string(),
    "session:abc123".to_string(),
])
.source("executor")
```

### Reinforces Skill

```rust
CreateEventRequest::new(
    EventKind::Action,
    json!({
        "kind": "outcome",
        "data": {
            "task": "debug_auth_issue",
            "result": "success",
            "skill_used": "debug_failing_test",
        }
    }),
)
.context_id("outcome:task")
.reinforces(vec!["skill:debug_failing_test".to_string()])
.source("executor")
```

---

## Retrieval

Outcomes are retrieved:
- Starting similar task → what happened last time?
- Planning work → what's been tried?
- Retrospectives → what worked/failed?

```rust
let context_ids = vec![
    "outcome:task",     // task outcomes
    "outcome:session",  // session summaries
];
```

---

## Outcome vs Action

| Type | Meaning | Scope |
|------|---------|-------|
| **Action** | Something agent did | Individual operation |
| **Outcome** | Result of a task | Aggregate of actions |

An outcome summarizes multiple actions into a meaningful result.

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `Action` (success), `Observation` (failed/abandoned) |
| **Context IDs** | `outcome:task`, `outcome:session`, `outcome:milestone` |
| **Source** | `executor` (0.8), `validator` (0.9), `user` (0.7) |
| **Retrieve** | Before similar tasks, for retrospectives |
| **Links** | `related_to` sessions/decisions, `reinforces` skills |
