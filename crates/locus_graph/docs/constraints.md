# Constraints — Rules the Agent Must Follow

## What is a Constraint?

A constraint is a **hard rule or limitation** the agent must respect — security rules, resource limits, user preferences, project requirements. Unlike facts (descriptive), constraints are prescriptive.

```
Constraint: "Never commit to main branch"
  → Agent creates feature branch instead
  → Constraint recalled before every commit
```

---

## When to Store a Constraint

| Trigger | Context ID | Example |
|---------|------------|---------|
| **Security rule** | `constraint:security` | "Never log API keys" |
| **Resource limit** | `constraint:resources` | "Max 1GB memory for builds" |
| **User preference** | `constraint:user` | "Don't use AI-generated comments" |
| **Project rule** | `constraint:project` | "All PRs need 2 approvals" |
| **Compliance** | `constraint:compliance` | "PII must be encrypted" |

---

## Constraint Structure

### Security Constraint

```rust
CreateEventRequest::new(
    EventKind::Fact,  // Constraints are facts about rules
    json!({
        "kind": "constraint",
        "data": {
            "category": "security",
            "rule": "Never commit secrets to version control",
            "enforcement": "hard",  // hard = must follow, soft = should follow
            "applies_to": ["git", "files"],
            "rationale": "Prevents credential exposure",
        }
    }),
)
.context_id("constraint:security")
.source("user")  // User-specified = high priority
```

### Resource Constraint

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "constraint",
        "data": {
            "category": "resources",
            "rule": "Build commands must complete within 5 minutes",
            "enforcement": "hard",
            "applies_to": ["cargo build", "npm run build"],
            "rationale": "CI timeout limit",
        }
    }),
)
.context_id("constraint:resources")
.source("system")
```

### Project Constraint

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "constraint",
        "data": {
            "category": "project",
            "rule": "No external dependencies without review",
            "enforcement": "soft",
            "applies_to": ["Cargo.toml", "package.json"],
            "rationale": "Security and license compliance",
        }
    }),
)
.context_id("constraint:project")
.related_to(vec!["project:abc123".to_string()])
.source("user")
```

---

## Constraint Enforcement Levels

| Level | Meaning | Behavior |
|-------|---------|----------|
| `hard` | Must follow | Agent stops if violated |
| `soft` | Should follow | Agent warned, can proceed |
| `preference` | Nice to have | Agent informed, flexible |

---

## Source Priority for Constraints

| Source | Confidence | Use Case |
|--------|------------|----------|
| `validator` | 0.9 | Runtime-enforced (e.g., CI rules) |
| `user` | 0.7 | User-specified preferences |
| `agent` | 0.6 | Agent-inferred from patterns |
| `system` | 0.5 | System limitations |

---

## Constraint Categories

### Security Constraints
```
constraint:security
  - "Never log sensitive data"
  - "Validate all user input"
  - "Use parameterized queries"
  - "Don't expose stack traces in production"
```

### Resource Constraints
```
constraint:resources
  - "Max 1GB heap"
  - "Timeout: 60s for API calls"
  - "Max 10 concurrent operations"
```

### Project Constraints
```
constraint:project
  - "Main branch is protected"
  - "All code must pass linting"
  - "No force pushes"
```

### Style Constraints
```
constraint:style
  - "Use tabs, not spaces"
  - "Max 80 character lines"
  - "No AI-generated comments"
```

---

## Constraint Checking

Before actions, the agent should check constraints:

```rust
// Pseudo-code for constraint checking
async fn check_constraints(action: &Action) -> Vec<ConstraintViolation> {
    let constraints = recall_memories("constraints", vec!["constraint"]).await;

    constraints.iter()
        .filter_map(|c| {
            if c.applies_to(&action) && c.is_violated_by(&action) {
                Some(ConstraintViolation {
                    constraint: c,
                    action: action.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}
```

---

## Updating Constraints

### Add Constraint

```rust
CreateEventRequest::new(
    EventKind::Fact,
    json!({
        "kind": "constraint",
        "data": {
            "category": "security",
            "rule": "New: All file writes must be under project root",
            "enforcement": "hard",
        }
    }),
)
.context_id("constraint:security")
.source("user")
```

### Relax Constraint

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "constraint_updated",
        "data": {
            "constraint": "build_timeout",
            "old_value": "5 minutes",
            "new_value": "10 minutes",
            "reason": "Larger projects need more time",
        }
    }),
)
.context_id("constraint:resources")
.contradicts(vec!["constraint:build_timeout_5min".to_string()])
.source("user")
```

---

## Retrieval

Constraints are retrieved:
- Before file operations → security constraints
- Before git operations → project constraints
- Before long operations → resource constraints

```rust
let context_ids = vec![
    "constraint:security",  // security rules
    "constraint:project",   // project rules
    "constraint:user",      // user preferences
];
```

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `EventKind::Fact` (constraints are facts about rules) |
| **Context IDs** | `constraint:{category}` |
| **Source** | `user` (0.7), `validator` (0.9), `system` (0.5) |
| **Retrieve** | Before actions that might violate |
| **Links** | `contradicts` to update, `extends` to add detail |
