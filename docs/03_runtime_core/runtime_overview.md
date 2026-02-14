# Runtime Core Overview

The Runtime Core is the **brain stem** of locus.codes. It receives prompts from the UI, builds execution plans, dispatches work to subagents, coordinates through ToolBus, and communicates state via Event Bus.

## Modules

| Module | Responsibility |
|--------|---------------|
| **Session Manager** | Repo context, branch, working directory, git state, threads |
| **Mode Controller** | Rush / Smart / Deep selection, affects model routing and budgets |
| **Orchestrator** | Central coordinator — builds DAG, manages full 10-step lifecycle |
| **Scheduler** | Parallel task execution — spawns subagents concurrently |
| **Event Bus** | Pub/sub for runtime ↔ UI real-time updates |

## Flow

```
UI → Orchestrator → Scheduler → Subagents → ToolBus → Results → Event Bus → UI
```

## Ownership

- The Runtime Core owns the execution lifecycle
- It does NOT own memory (that's LocusGraph)
- It does NOT own model selection details (that's LLM Engine)
- It coordinates between all other layers
