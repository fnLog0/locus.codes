# Task Board

The main/home screen. This is where the user starts.

## Sections

| Section | Content |
|---------|---------|
| **Active Task** | Current task: prompt text, status (running/waiting/done), progress |
| **Task Queue** | Pending tasks waiting for execution |
| **History** | Recent completed tasks with results (success/fail) |
| **Threads** | Saved interaction sequences, resumable |

## Behavior

- Displays real-time progress of the active task via Event Bus
- Shows which subagents are running and their status
- Completed tasks show summary: files changed, tests passed, events stored
- User can select a historical task to review its details
- New prompt input starts a new task (sent to Orchestrator)

## Layout

```
┌──────────────────────────────────────┐
│ ● Active: "Fix auth bug in login.rs" │
│   Status: PatchAgent running...      │
│   Files: src/auth/login.rs           │
├──────────────────────────────────────┤
│ Queue (2)                            │
│   → Add unit tests for parser        │
│   → Update README                    │
├──────────────────────────────────────┤
│ History                              │
│   ✓ Refactor database module  (2m)   │
│   ✓ Fix CI pipeline           (45s)  │
│   ✗ Migrate to async runtime  (err)  │
├──────────────────────────────────────┤
│ > _                                  │
└──────────────────────────────────────┘
```
