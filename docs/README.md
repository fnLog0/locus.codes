# locus.codes Documentation

locus.codes is a frontier terminal-native coding agent where the LLM is stateless compute and LocusGraph is the permanent deterministic brain. No AGENTS.md. No skill files. The agent learns from experience.

---

## Sections

| # | Section | Purpose |
|---|---------|---------|
| 00 | [Overview](00_overview/) | Vision, principles, glossary |
| 01 | [System Architecture](01_system_architecture/) | Layers A–H, component map, dataflow, execution pipeline, modes |
| 02 | [UI Layer](02_ui_layer/) | Terminal UI (ratatui + crossterm), views, input, routing, keybindings |
| 03 | [Runtime Core](03_runtime_core/) | Orchestrator, scheduler, session manager, subagents, ToolBus |
| 04 | [LocusGraph Memory](04_locusgraph_memory/) | Deterministic memory, events, relations, constraints, reinforcement, injection |
| 05 | [Storage Layer](05_storage_layer/) | LocusGraph SDK, caching |
| 06 | [LLM Engine](06_llm_engine/) | Multi-model routing, prompt templates, memory bundles, response schema |
| 07 | [Execution Engine](07_execution_engine/) | Patch pipeline, diff generation, test runner, debug loop, commit |
| 08 | [Protocols](08_protocols/) | ToolBus API, runtime events, agent reports |
| 09 | [Security](09_security/) | Permission model, sandbox, secrets |
| 10 | [Examples](10_examples/) | Task flow, memory events, diff review, constraints |

## Build Order

1. **Phase 1 — Core Skeleton**: UI router + prompt input, Orchestrator + Scheduler, ToolBus (file read/write + run_cmd), Diff viewer
2. **Phase 2 — Agent Workflow**: RepoAgent + PatchAgent, TestAgent, DebugAgent loop
3. **Phase 3 — LocusGraph Integration**: Event schema, memory recall + injection engine, event extractor writeback
4. **Phase 4 — Constraints + Reinforcement**: Constraints engine, violation detection, reinforcement scoring
5. **Phase 5 — Multi-model + Optimization**: Model routing, caching, session replay, performance tuning
