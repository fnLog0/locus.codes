# LocusGraph — Implicit Links (Rules, Constraints, Violations)

Explicit links (`extends`, `related_to`, `reinforces`, `contradicts`) are set by agent/user at event time. Implicit links are **system-inferred** — every action is checked against rules and constraints automatically.

---

## Explicit vs Implicit

| | Explicit | Implicit |
|---|---|---|
| Who creates | Agent/user at event time | System infers automatically |
| Links | `extends`, `related_to`, `reinforces`, `contradicts` | Matched against `rule:`, `constraint:` |
| When | On store | On every `action:`, `decision:`, `file:` event |
| Stored as | Fields on `CreateEventRequest` | `constraint_violation:` events |

---

## New Context Types

| Type | Severity | Purpose | Example |
|---|---|---|---|
| `rule:` | Soft — should follow | Best practices, conventions | `rule:read_before_edit` |
| `constraint:` | Hard — must not violate | Security, safety boundaries | `constraint:no_secrets_in_code` |
| `constraint_violation:` | — | System-detected breach | auto-created when rule/constraint breaks |

---

## Where They Live

### Global (universal rules)

```
agent:locus
  └── rule_anchor:locus
        ├── rule:read_before_edit
        ├── rule:verify_after_change
        ├── constraint:no_secrets_in_prompts
        ├── constraint:no_destructive_without_confirm
        └── constraint:max_create_file_8k
```

### Project (project-specific rules)

```
project:locuscodes_abc123
  └── knowledge_anchor:locuscodes_abc123
        ├── fact:rust_error_conventions
        ├── rule:cargo_check_after_edit
        ├── rule:use_anyhow_for_app_errors
        ├── constraint:files_within_repo_root
        ├── constraint:no_circular_crate_deps
        └── constraint:toolbus_api_stable
```

---

## Rule / Constraint Payload

### rule:

```json
{
  "context_id": "rule:read_before_edit",
  "event_kind": "fact",
  "source": "system",
  "payload": {
    "description": "Always read a file before editing it",
    "trigger": {
      "event_type": "action",
      "tool": "edit_file"
    },
    "check": {
      "condition": "file_was_read_this_turn",
      "field": "args.path"
    },
    "severity": "soft",
    "times_enforced": 0,
    "times_violated": 0
  },
  "extends": ["rule_anchor:locus"]
}
```

### constraint:

```json
{
  "context_id": "constraint:no_secrets_in_code",
  "event_kind": "fact",
  "source": "validator",
  "payload": {
    "description": "Never write API keys, tokens, or passwords into source files",
    "trigger": {
      "event_type": "file",
      "operation": "edit|create"
    },
    "check": {
      "condition": "content_matches_secret_pattern",
      "patterns": ["API_KEY=", "SECRET=", "password:", "Bearer "]
    },
    "severity": "hard",
    "times_enforced": 0,
    "times_violated": 0
  },
  "extends": ["rule_anchor:locus"]
}
```

---

## Implicit Matching Engine

Every `action:`, `decision:`, `file:` event passes through the matching engine before being stored:

```
Event arrives: action:a1b2c3d4_003_002 (edit_file src/main.rs)
         │
         ▼
┌─────────────────────────────┐
│  Load active rules +        │
│  constraints for this       │
│  project + global           │
└──────────┬──────────────────┘
           │
           ▼
┌─────────────────────────────┐
│  Check each rule/constraint │
│  against the event          │
└──────────┬──────────────────┘
           │
     ┌─────┴─────┐
     │            │
   PASS         FAIL
     │            │
     ▼            ▼
  increment    create
  times_       constraint_violation:
  enforced     a1b2c3d4_003_003
```

### Check: Pass

Rule/constraint `times_enforced` counter increments. No event created. Silent.

### Check: Fail (Violation)

System auto-creates a `constraint_violation:` event:

```json
{
  "context_id": "constraint_violation:a1b2c3d4_003_003",
  "event_kind": "observation",
  "source": "system",
  "payload": {
    "violated": "rule:read_before_edit",
    "severity": "soft",
    "action": "action:a1b2c3d4_003_002",
    "description": "edit_file called on src/main.rs without reading it first this turn",
    "seq": 3
  },
  "extends": ["turn:a1b2c3d4_003"],
  "related_to": ["rule:read_before_edit", "action:a1b2c3d4_003_002"]
}
```

Rule/constraint `times_violated` counter increments.

---

## Severity Behavior

| Severity | On violation | Agent behavior |
|---|---|---|
| `soft` (rule) | Log violation, notify agent, continue | Agent sees warning in next LLM context |
| `hard` (constraint) | Log violation, **block action**, notify | Action is rejected, agent must find alternative |

### Soft violation → LLM context injection

```
[Rule Violation] rule:read_before_edit
You edited src/main.rs without reading it first.
Read the file before making changes to avoid blind edits.
```

### Hard violation → Action blocked

```
[Constraint Violated] constraint:no_secrets_in_code
Blocked: create_file would write content matching secret pattern "API_KEY=".
Remove the secret and retry.
```

---

## Feed Loop: Violations → Learning

```
constraint:no_secrets_in_code               ← defined once (global)
  │
  ├── constraint_violation: (turn 3)        ← detected, action blocked
  ├── constraint_violation: (turn 7)        ← detected again
  │
  └── after 2+ violations:
        mistake:leaked_secret_in_code       ← auto-created in learning_anchor
          │
          └── after workflow learned:
                pattern:check_secrets_before_write  ← recognized
                  │
                  └── after 5+ successes:
                        skill:secret_management     ← graduated
```

| Violations count | System action |
|---|---|
| 1 | Log `constraint_violation:`, warn agent |
| 2 | Create `mistake:` in `mistake_anchor:` |
| 3+ | Inject mistake into every LLM context for this rule |
| 0 for 10+ sessions | Archive mistake (confidence decays) |

---

## Difference from Other Types

| Type | Source | Timing | Purpose |
|---|---|---|---|
| `rule:` | Defined (system/user) | Static | "you should do X" |
| `constraint:` | Defined (system/user) | Static | "you must not do Y" |
| `constraint_violation:` | Implicit (engine) | Real-time | "you just broke Y" |
| `mistake:` | Learned (from violations) | After repeated violations | "you keep breaking Y" |
| `pattern:` | Learned (from successes) | After repeated workflows | "when X, do Y" |
| `skill:` | Graduated (from patterns) | After high confidence | "I know how to Y" |
| `error:` | Observed (tool failure) | Real-time | "tool X failed" |

---

## Built-in Rules (Ship with locus.codes)

### Global (rule_anchor:locus)

| context_id | severity | description |
|---|---|---|
| `rule:read_before_edit` | soft | Always read a file before editing it |
| `rule:verify_after_change` | soft | Run tests/check after making changes |
| `rule:small_incremental_edits` | soft | Make small changes, verify, then continue |
| `rule:confirm_destructive_ops` | soft | Ask user before destructive operations |
| `constraint:no_secrets_in_code` | hard | Never write secrets into source files |
| `constraint:no_secrets_in_prompts` | hard | Never include secrets in LLM context |
| `constraint:no_destructive_without_confirm` | hard | Block rm -rf, git push --force without confirmation |
| `constraint:files_within_repo_root` | hard | All file operations must stay inside repo root |
| `constraint:max_create_file_8k` | hard | Single create_file content must not exceed ~8k chars |

### Project-specific (knowledge_anchor:{project})

Defined by user or learned from project conventions. Examples:

| context_id | severity | description |
|---|---|---|
| `rule:cargo_check_after_edit` | soft | Run cargo check after editing Rust files |
| `rule:use_anyhow_for_app_errors` | soft | Use anyhow::Result for application code |
| `constraint:no_circular_crate_deps` | hard | Crates must not have circular dependencies |
| `constraint:toolbus_api_stable` | hard | Do not change ToolBus public API without discussion |
