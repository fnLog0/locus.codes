# Agents View

Shows all active subagents running in parallel.

## Agent Card

Each agent is displayed as a card:

| Field | Content |
|-------|---------|
| **Name** | Agent type (e.g. RepoAgent, PatchAgent) |
| **Task** | Current task description |
| **Status** | `running` / `complete` / `error` |
| **Output** | Live output stream (truncated) |
| **Duration** | Elapsed time |
| **Tokens** | Tokens consumed (if LLM-backed) |

## Layout

```
┌─────────────────────┬─────────────────────┐
│ RepoAgent           │ MemoryRecallAgent   │
│ Task: scan repo     │ Task: recall ctx    │
│ Status: ✓ complete  │ Status: ✓ complete  │
│ Time: 1.2s          │ Time: 0.8s          │
│ Files found: 3      │ Memories: 7         │
├─────────────────────┼─────────────────────┤
│ PatchAgent          │ SearchAgent         │
│ Task: generate fix  │ Task: grep patterns │
│ Status: ● running   │ Status: ✓ complete  │
│ Tokens: 1,204       │ Matches: 12         │
├─────────────────────┴─────────────────────┤
│ > _                                       │
└───────────────────────────────────────────┘
```

## Behavior

- Cards appear when agents are spawned
- Real-time status updates via Event Bus
- Completed agents show result summary
- Failed agents show error with stack trace
- User can select an agent card to see full output
