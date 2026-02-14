# Event Model

How events are stored in LocusGraph via the Rust SDK.

## CreateEventRequest

```rust
CreateEventRequest {
    graph_id: String,
    event_kind: String,       // "fact" | "action" | "decision" | "observation" | "feedback"
    context_id: Option<String>, // scoping label, e.g. "terminal", "editor", "errors"
    source: Option<String>,    // "coding_agent", "user", etc.
    payload: serde_json::Value, // structured JSON content
    related_to: Option<Vec<String>>,
    extends: Option<Vec<String>>,
    reinforces: Option<Vec<String>>,
    contradicts: Option<Vec<String>>,
    timestamp: Option<String>,
}
```

## Event Kinds

| Kind | When to Use |
|------|-------------|
| `fact` | Recallable knowledge: file edits, project facts, user preferences |
| `action` | Something that happened: command run, patch applied, commit |
| `decision` | Reasoning stored: architecture choice, approach selected |
| `observation` | Outcome: test pass/fail, build result, error encountered |
| `feedback` | User signal: approval, rejection, correction |

## Payload Convention

Payloads follow `kind` + data pattern:

```json
{
  "kind": "terminal_command",
  "cwd": "/project",
  "command": "cargo test",
  "exit_code": 0,
  "stdout_preview": "12 tests passed",
  "timestamp": "2026-02-14T10:00:00Z"
}
```

## context_id Conventions

Use stable labels for scoped retrieval:

| context_id | Use |
|------------|-----|
| `"terminal"` | Terminal commands and output |
| `"editor"` | File edits and changes |
| `"user_intent"` | User goals and requests |
| `"errors"` | Failures and error messages |
| `"project"` | Project-level facts |
| `"constraints"` | Rules the agent must follow |

## Relation Fields

| Field | Use |
|-------|-----|
| `related_to` | General association to other context_ids |
| `extends` | This event refines or updates another |
| `reinforces` | This outcome supports another event |
| `contradicts` | This event overrides or conflicts with another |
