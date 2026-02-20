# Skills — How the Agent Learns

## What is a Skill?

A skill is a **validated pattern** the agent discovers while doing real work. It's not a rule you write — it's knowledge the agent earns through experience.

```
User: "fix the auth bug"
  → Agent tries approach A, fails
  → Agent tries approach B, succeeds
  → Agent stores: skill:auth_error_recovery
  → Next time auth breaks → agent recalls the fix instantly
```

---

## Skill Lifecycle

```
 ┌─────────────┐
 │  1. OBSERVE  │  Agent notices a pattern while doing a task
 └──────┬───────┘
        │
        ▼
 ┌─────────────┐
 │  2. ATTEMPT  │  Agent tries the pattern
 └──────┬───────┘
        │
        ├── fails ──► stored as observation (not a skill yet)
        │
        ▼ succeeds
 ┌─────────────┐
 │  3. STORE    │  Stored as skill (validated: false)
 └──────┬───────┘
        │
        ▼ works again in another context
 ┌─────────────┐
 │  4. VALIDATE │  Marked as validated (validated: true)
 └──────┬───────┘
        │
        ▼ contradicted by new evidence
 ┌─────────────┐
 │  5. EVOLVE   │  Updated or replaced with better approach
 └─────────────┘
```

---

## When to Store a Skill

| Trigger | Example | Action |
|---------|---------|--------|
| **Solved a hard problem** | Fixed a race condition | Store the fix pattern |
| **Found a better way** | Discovered a faster query | Store, contradict the old way |
| **User corrected the agent** | "Don't use `unwrap`, use `?`" | Store as validated (user = authority) |
| **Pattern worked twice** | Same approach fixed two bugs | Reinforce existing skill |
| **Pattern failed** | Previously stored skill didn't work | Contradict or update |

### When NOT to Store

- Trivial operations (creating a file, running `ls`)
- One-off fixes with no reusable pattern
- Project-specific config (use `project:{hash}` instead)

---

## Skill Structure

```rust
store_skill(
    name: "error_recovery_pattern",      // skill:{name} — unique identifier
    description: "When a tool fails...", // what the skill is about
    steps: vec![                         // concrete steps to follow
        "Check the error message for known patterns",
        "Try with --verbose flag for more context",
        "If timeout, increase to 60s",
        "If permission denied, check file ownership",
    ],
    validated: true,                     // has this been confirmed to work?
)
```

### Stored as:

```json
{
  "event_kind": "fact",
  "context_id": "skills",
  "related_to": ["skill:error_recovery_pattern"],
  "source": "agent",
  "payload": {
    "kind": "skill",
    "data": {
      "name": "error_recovery_pattern",
      "description": "When a tool fails...",
      "steps": ["Check the error message...", "..."],
      "validated": true
    }
  }
}
```

---

## Skill Categories

### 1. Tool Skills — How to use tools effectively

```
skill:grep_large_codebase
  "Use ripgrep with --type flag to narrow search scope.
   For monorepos, search specific directories first."

skill:bash_long_running
  "For commands >30s, run in background with timeout.
   Always capture stderr separately."
```

### 2. Fix Skills — How to solve specific problems

```
skill:rust_lifetime_errors
  "When you see 'borrowed value does not live long enough':
   1. Check if you can clone instead of borrow
   2. If in a closure, try move semantics
   3. If in a struct, consider Arc<T>"

skill:docker_build_cache
  "COPY Cargo.toml and Cargo.lock first, then cargo build,
   then COPY src — this caches dependencies separately."
```

### 3. Convention Skills — How this project does things

```
skill:this_project_testing
  "Tests go in tests/ not src/. Integration tests use
   the common::test_client() helper. Always --test-threads=1."

skill:this_project_errors  
  "Use thiserror for library crates, anyhow for binaries.
   Never unwrap in library code."
```

### 4. Workflow Skills — Multi-step procedures

```
skill:deploy_to_dev
  "1. cargo build --release
   2. docker build -t app:dev-latest .
   3. docker push to ECR
   4. kubectl rollout restart deployment/app-dev"

skill:debug_failing_test
  "1. Run with RUST_LOG=debug
   2. Check if it's a timing issue (add sleep)
   3. Check test isolation (--test-threads=1)
   4. Look at recent file changes in the test's module"
```

---

## How Skills Flow Through the System

```
                    ┌──────────────────────────────────┐
                    │         AGENT LOOP                │
                    │                                   │
  User message ───► │  1. Recall memories               │
                    │     └─ context_ids: ["skills",    │
                    │        "decisions", "errors",     │
                    │        "project:abc", "session:x"] │
                    │                                   │
                    │  2. LocusGraph returns:            │
                    │     ├─ skill:rust_lifetime_errors  │
                    │     ├─ skill:this_project_testing  │
                    │     └─ decision:use_grpc           │
                    │                                   │
                    │  3. Injected into LLM prompt:      │
                    │     "## Relevant Memories          │
                    │      - When lifetime errors..."    │
                    │                                   │
                    │  4. LLM uses skills to respond     │
                    │                                   │
                    │  5. If agent learns something new: │
                    │     └─ store_skill(...)            │
                    └──────────────────────────────────┘
```

---

## Updating Skills

### Reinforce — Same skill worked again

```rust
// Agent used skill:grep_large_codebase and it worked
let event = CreateEventRequest::new(
    EventKind::Action,
    json!({
        "kind": "skill_applied",
        "data": {
            "skill": "grep_large_codebase",
            "context": "searched 50k files, found result in 2s",
            "success": true
        }
    }),
)
.context_id("skills")
.reinforces(vec!["skill:grep_large_codebase".to_string()])
.source("agent");
```

### Evolve — Found a better way

```rust
// Agent discovered a better approach
client.store_skill(
    "grep_large_codebase_v2",
    "Use ast-grep for structural search, ripgrep for text",
    vec![
        "For pattern matching: use ast-grep with language-specific rules",
        "For text search: use ripgrep with --type and path filters",
        "For symbol lookup: use LSP go-to-definition first",
    ],
    true,
);

// Mark old skill as superseded
let update = CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "skill_evolved",
        "data": {
            "old": "grep_large_codebase",
            "new": "grep_large_codebase_v2",
            "reason": "ast-grep is more precise for code patterns"
        }
    }),
)
.context_id("skills")
.contradicts(vec!["skill:grep_large_codebase".to_string()])
.related_to(vec!["skill:grep_large_codebase_v2".to_string()])
.source("agent");
```

### Invalidate — Skill was wrong

```rust
let event = CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "skill_invalidated",
        "data": {
            "skill": "always_use_clone",
            "reason": "User corrected: cloning is expensive, use references",
            "correction": "Prefer borrowing over cloning unless ownership needed"
        }
    }),
)
.context_id("skills")
.contradicts(vec!["skill:always_use_clone".to_string()])
.source("user");  // user correction = high confidence
```

---

## Retrieval Priority

When the agent recalls memories, skills are ranked by:

1. **Semantic relevance** — how closely the skill matches the current task
2. **Source confidence** — user (0.7) > agent (0.6) > system (0.5)
3. **Reinforcement count** — skills reinforced multiple times rank higher
4. **Recency** — recently validated skills over stale ones
5. **Contradiction status** — contradicted skills are deprioritized

---

## Summary

| Concept | Description |
|---------|-------------|
| **Store** | `context_id: "skills"`, `related_to: ["skill:{name}"]` |
| **Retrieve** | Always included in `build_context_ids()` |
| **Reinforce** | Use `reinforces: ["skill:{name}"]` when it works again |
| **Evolve** | Store new skill + `contradicts: ["skill:{old}"]` |
| **Invalidate** | Store feedback + `contradicts: ["skill:{wrong}"]` |
| **Source** | User corrections = `"user"`, agent discoveries = `"agent"` |
