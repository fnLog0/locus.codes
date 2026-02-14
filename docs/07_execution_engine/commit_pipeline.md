# Commit Pipeline

After tests pass and user approves, changes are committed.

## Flow

```
Tests pass → User confirms commit → LLM generates commit message
  → git add (changed files)
  → git commit -m "message"
  → Optional: git push (requires permission)
  → Commit event stored in LocusGraph
```

## Commit Message Generation

The LLM generates a commit message based on:
- Original task/prompt
- Files changed (diff summary)
- Test results

Format: conventional commits style (configurable).

## Git Operations

All git operations go through ToolBus:

| Operation | Permission |
|-----------|------------|
| `git add` | write |
| `git commit` | git_write (ask) |
| `git push` | git_write (always ask) |
| `git push --force` | blocked by default |

## Event Storage

After commit, the Event Extractor stores:

```json
{
  "event_kind": "action",
  "payload": {
    "kind": "commit",
    "data": {
      "hash": "abc123f",
      "message": "fix: validate token expiry in login handler",
      "files_changed": ["src/auth/login.rs"],
      "tests_passed": 12,
      "tests_failed": 0
    }
  },
  "context_id": "action:commit_abc123f"
}
```

## Rollback

If the user wants to undo:
- `git reset --soft HEAD~1` (via ToolBus, requires permission)
- Rollback event stored in LocusGraph
