# Orchestrator

The central coordinator of locus.codes. Manages the full lifecycle from prompt to commit.

## Responsibilities

1. Receive user prompt from UI
2. Analyze intent
3. Build DAG of subtasks with dependencies
4. Coordinate with MemoryRecallAgent for context injection
5. Dispatch DAG to Scheduler
6. Monitor execution progress
7. Handle retries and debug loops
8. Trigger Event Extractor on completion

## DAG Construction

Given a prompt, the Orchestrator decomposes it into subtasks:

```
"Fix auth bug in login.rs"
│
├── [parallel]
│   ├── RepoAgent: scan repo, find login.rs and related files
│   ├── MemoryRecallAgent: recall auth-related memories
│   └── SearchAgent: grep for token validation patterns
│
├── [sequential]
│   ├── PatchAgent: generate fix (depends on parallel results)
│   ├── DiffReview: user approval
│   ├── TestAgent: run tests
│   ├── DebugAgent: fix failures (conditional)
│   └── Commit: git commit (conditional on user approval)
│
└── EventExtractor: write memories (always)
```

## Debug Loop

When TestAgent reports failure:

1. Orchestrator activates DebugAgent
2. DebugAgent analyzes failure → proposes fix
3. New patch generated → tested again
4. Loop until success or max retries (configurable per mode)

## Lifecycle Management

- Tracks state of every DAG node
- Emits events to Event Bus for UI updates
- Can cancel/abort on user request
- Handles graceful shutdown
