# Component Map

| Component | Layer | Crate | Purpose |
|-----------|-------|-------|---------|
| View Router | A — UI | `locus-ui` | Switches between 6 views |
| Task Board | A — UI | `locus-ui` | Main screen: prompt history, task status |
| Plan View | A — UI | `locus-ui` | Execution DAG visualization |
| Agents View | A — UI | `locus-ui` | Active subagent cards |
| Diff Review | A — UI | `locus-ui` | PR-style patch approval |
| Logs View | A — UI | `locus-ui` | Command output display |
| Memory Trace | A — UI | `locus-ui` | Debug: LocusGraph activity |
| Prompt Bar | A — UI | `locus-ui` | Global input (bottom bar) |
| Session Manager | B — Runtime | `locus-runtime` | Repo, branch, git state |
| Mode Controller | B — Runtime | `locus-runtime` | Rush / Smart / Deep |
| Orchestrator | B — Runtime | `locus-runtime` | DAG builder, lifecycle controller |
| Scheduler | B — Runtime | `locus-runtime` | Parallel task dispatch |
| Event Bus | B — Runtime | `locus-runtime` | Pub/sub for runtime ↔ UI |
| RepoAgent | C — Subagents | `locus-agents` | Repo scan, file discovery |
| MemoryRecallAgent | C — Subagents | `locus-agents` | LocusGraph query |
| PatchAgent | C — Subagents | `locus-agents` | Code patch generation |
| TestAgent | C — Subagents | `locus-agents` | Test execution, reporting |
| DebugAgent | C — Subagents | `locus-agents` | Failure analysis, fix proposals |
| SearchAgent | C — Subagents | `locus-agents` | Codebase search |
| ConstraintAgent | C — Subagents | `locus-agents` | Constraint validation |
| ToolBus | D — ToolBus | `locus-toolbus` | Execution gateway, permissions |
| Model Router | E — Multi-Model | `locus-llm` | Mode-based model selection |
| Prompt Builder | E — Multi-Model | `locus-llm` | Template + memory bundle assembly |
| LocusGraph Client | F — LocusGraph | `locus-graph` | Locus CRUD, link management |
| Injection Engine | G — Injection | `locus-graph` | Memory → LLM context formatting |
| Event Extractor | H — Extractor | `locus-graph` | Action → deterministic events |
