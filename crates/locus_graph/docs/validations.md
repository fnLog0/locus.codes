# Validations — Verification Results

## What is a Validation?

A validation is the **result of checking something** — linting, type checking, tests, security scans. Validations confirm correctness or reveal issues.

```
Agent: "I fixed the bug"
Validation: cargo test → FAILED
  → Agent sees validation failed
  → Agent fixes the issue
  → Validation: cargo test → PASSED
```

---

## When to Store a Validation

| Trigger | Context ID | Example |
|---------|------------|---------|
| **Tests run** | `validation:tests` | `cargo test` result |
| **Lint check** | `validation:lint` | `cargo clippy` result |
| **Type check** | `validation:types` | `cargo check` result |
| **Security scan** | `validation:security` | Dependency audit |
| **Build** | `validation:build` | `cargo build` result |

---

## Validation Structure

### Test Validation

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation",
        "data": {
            "type": "tests",
            "command": "cargo test",
            "result": "failed",
            "passed": 45,
            "failed": 2,
            "failures": [
                {
                    "test": "test_auth_flow",
                    "error": "assertion failed at src/auth.rs:42",
                },
                {
                    "test": "test_user_creation",
                    "error": "timeout after 30s",
                }
            ],
            "duration_ms": 12345,
        }
    }),
)
.context_id("validation:tests")
.source("validator")  // Highest confidence (0.9)
```

### Lint Validation

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation",
        "data": {
            "type": "lint",
            "command": "cargo clippy",
            "result": "warnings",
            "errors": 0,
            "warnings": 3,
            "items": [
                {
                    "level": "warning",
                    "code": "uninlined_format_args",
                    "message": "format string can be inlined",
                    "file": "src/main.rs",
                    "line": 42,
                }
            ],
        }
    }),
)
.context_id("validation:lint")
.source("validator")
```

### Type Check Validation

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation",
        "data": {
            "type": "type_check",
            "command": "cargo check",
            "result": "passed",
            "errors": 0,
            "warnings": 0,
        }
    }),
)
.context_id("validation:types")
.source("validator")
```

### Build Validation

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation",
        "data": {
            "type": "build",
            "command": "cargo build --release",
            "result": "passed",
            "duration_ms": 45000,
            "output_size_bytes": 2048576,
        }
    }),
)
.context_id("validation:build")
.source("validator")
```

---

## Validation Results

| Result | Meaning | Next Action |
|--------|---------|-------------|
| `passed` | All checks successful | Continue |
| `failed` | Critical errors | Must fix |
| `warnings` | Non-critical issues | Should fix |
| `skipped` | Check not run | Investigate |

---

## Source Priority for Validations

| Source | Confidence | Use Case |
|--------|------------|----------|
| `validator` | 0.9 | Runtime-verified, authoritative |
| `executor` | 0.8 | Tool output, reliable |

Validations always use `validator` or `executor` source — they are objective results.

---

## Validation → Feedback Pipeline

Failed validations become feedback for the agent:

```
1. Agent makes change
2. Validation runs (cargo test)
3. Validation fails
4. Stored as validation + feedback
5. Agent recalls, fixes issues
6. Re-validate
```

---

## Validation Linking

### After Change

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation",
        "data": {
            "type": "tests",
            "result": "failed",
            "after_change": "src/auth.rs: Added timeout logic",
        }
    }),
)
.context_id("validation:tests")
.related_to(vec!["action:edit_src_auth_rs".to_string()])
.source("validator")
```

### Reinforces Pattern

```rust
CreateEventRequest::new(
    EventKind::Feedback,
    json!({
        "kind": "validation",
        "data": {
            "type": "tests",
            "result": "passed",
            "pattern_used": "test_driven_development",
        }
    }),
)
.context_id("validation:tests")
.reinforces(vec!["skill:test_driven_development".to_string()])
.source("validator")
```

---

## Validation Categories

### Test Validations
```
validation:tests
  - Unit test results
  - Integration test results
  - Coverage reports
```

### Lint Validations
```
validation:lint
  - Clippy warnings/errors
  - ESLint results
  - Custom lint rules
```

### Type Validations
```
validation:types
  - cargo check
  - tsc (TypeScript)
  - mypy (Python)
```

### Security Validations
```
validation:security
  - cargo audit
  - npm audit
  - SAST scan results
```

---

## Retrieval

Validations are retrieved:
- After making changes → what failed before?
- Before committing → are validations passing?
- Debugging → what was the last validation state?

```rust
let context_ids = vec![
    "validation:tests",  // test results
    "validation:lint",   // lint results
    "validation:types",  // type check results
];
```

---

## Summary

| Concept | Description |
|---------|-------------|
| **Event Kind** | `EventKind::Feedback` (validations are feedback) |
| **Context IDs** | `validation:{type}` |
| **Source** | `validator` (0.9), `executor` (0.8) |
| **Retrieve** | After changes, before commits, during debugging |
| **Links** | `related_to` changes, `reinforces` patterns |
