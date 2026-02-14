# Dataflow

## Prompt → Execution → Memory

```
┌─────────┐     prompt      ┌──────────────┐
│  UI      │ ──────────────→ │ Orchestrator │
│ (Layer A)│                 │ (Layer B)    │
└─────────┘                  └──────┬───────┘
                                    │ builds DAG
                                    ▼
                             ┌──────────────┐
                             │  Scheduler   │
                             │ (Layer B)    │
                             └──────┬───────┘
                                    │ dispatches parallel
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
             ┌───────────┐  ┌───────────┐  ┌───────────┐
             │ RepoAgent │  │MemRecall  │  │SearchAgent│
             │ (Layer C) │  │ (Layer C) │  │ (Layer C) │
             └─────┬─────┘  └─────┬─────┘  └─────┬─────┘
                   │              │               │
                   ▼              ▼               │
             ┌───────────┐  ┌───────────┐         │
             │  ToolBus  │  │ LocusGraph│         │
             │ (Layer D) │  │ (Layer F) │         │
             └───────────┘  └─────┬─────┘         │
                                  ▼               │
                           ┌───────────┐          │
                           │ Injection │          │
                           │ (Layer G) │          │
                           └─────┬─────┘          │
                                 │ memory bundle  │
                    ┌────────────┴────────────────┘
                    ▼
             ┌───────────┐     ┌───────────┐
             │ LLM Engine│ ──→ │PatchAgent │
             │ (Layer E) │     │ (Layer C) │
             └───────────┘     └─────┬─────┘
                                     │ patch
                                     ▼
                              ┌───────────┐
                              │Diff Review│ → user approve/reject
                              │ (Layer A) │
                              └─────┬─────┘
                                    │ apply
                                    ▼
                              ┌───────────┐
                              │ TestAgent │ → pass/fail
                              │ (Layer C) │
                              └─────┬─────┘
                                    │ if fail
                                    ▼
                              ┌───────────┐
                              │DebugAgent │ → fix loop
                              │ (Layer C) │
                              └─────┬─────┘
                                    │ on success
                                    ▼
                              ┌───────────┐
                              │  Commit   │
                              │ (Layer D) │
                              └─────┬─────┘
                                    │ events
                                    ▼
                              ┌───────────┐
                              │ Extractor │ → writes to LocusGraph
                              │ (Layer H) │
                              └───────────┘
```

## Data At Each Stage

| Stage | Data Produced |
|-------|---------------|
| Prompt | Raw user text, mode selection |
| Orchestrator | DAG of tasks with dependencies |
| RepoAgent | File tree, relevant file paths |
| MemoryRecallAgent | Memory bundle (relevant locuses) |
| SearchAgent | Grep/LocusGraph SDK search results |
| Injection Engine | Formatted context for LLM |
| LLM Engine | Tool calls, reasoning, confidence |
| PatchAgent | Unified diffs |
| Diff Review | User approval/rejection |
| TestAgent | Pass/fail results per test |
| DebugAgent | Failure analysis, fix patch |
| Commit | Commit hash, message |
| Event Extractor | Deterministic events → LocusGraph |
