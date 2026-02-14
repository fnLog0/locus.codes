# Scheduler

Parallel task execution engine. Takes a DAG from the Orchestrator and executes it efficiently.

## Responsibilities

- Analyze DAG for parallelizable branches
- Spawn subagents concurrently for independent tasks
- Manage task queue and priority
- Enforce resource limits (max concurrent agents)
- Report completion/failure back to Orchestrator

## Execution Model

```
DAG received → identify ready tasks (no unmet dependencies) → spawn agents → wait
   → on completion: mark done, check dependents, spawn newly-ready tasks
   → on failure: report to Orchestrator, pause dependents
```

## Concurrency

- Subagents run as async tasks (tokio)
- Max concurrent agents configurable (default: 4)
- Each agent has its own context window (no shared state)
- Results collected via channels

## Priority

| Priority | Description |
|----------|-------------|
| High | MemoryRecallAgent (blocks context injection) |
| Normal | RepoAgent, SearchAgent, PatchAgent |
| Low | ConstraintAgent (validation, can run late) |

## Resource Limits

- Max concurrent agents per mode: Rush=2, Smart=4, Deep=6
- Total token budget per task (enforced by LLM Engine)
- Timeout per agent (configurable)
