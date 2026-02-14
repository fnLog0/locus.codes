# Plan View

Shows the execution DAG built by the Orchestrator.

## Purpose

Visualizes the task decomposition: what subtasks exist, their dependencies, which can run in parallel, and current completion status.

## Node Display

Each DAG node shows:
- Task description
- Assigned agent
- Status: `pending` / `running` / `done` / `failed`
- Duration (when completed)

## Layout

```
┌────────────────────────────────────────────┐
│ Plan: "Fix auth bug in login.rs"           │
│                                            │
│  [RepoAgent: scan] ──┐                     │
│       ✓ done (1.2s)  │                     │
│                      ├──→ [PatchAgent: fix] │
│  [MemRecall: ctx]  ──┘      ● running      │
│       ✓ done (0.8s)                         │
│                          ↓                  │
│  [SearchAgent: grep] ──→ [TestAgent: test]  │
│       ✓ done (0.5s)      ○ pending          │
│                              ↓              │
│                          [Commit]           │
│                           ○ pending         │
├────────────────────────────────────────────┤
│ > _                                        │
└────────────────────────────────────────────┘
```

## Behavior

- Updates in real-time as agents complete
- Parallel branches shown side-by-side
- Failed nodes highlighted, show error summary
- User can inspect any node for details
