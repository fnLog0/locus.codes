# System Architecture

locus.codes is organized into 8 layers (A–H). Each layer has a single responsibility and communicates through well-defined interfaces.

## Layer A — UI Layer (Terminal + Editor UX)

Built with **ratatui + crossterm**. UI is mission control, not a text editor.

- 6 views: Task Board, Plan View, Agents View, Diff Review, Logs View, Memory Trace
- Global prompt input bar (always visible at bottom)
- Event-driven updates via Event Bus

## Layer B — Runtime Core (The OS)

The main engine. Receives prompts, builds plans, coordinates execution.

- **Session Manager** — repo, branch, working directory, git state
- **Mode Controller** — Rush / Smart / Deep selection
- **Orchestrator** — builds DAG, controls full lifecycle
- **Scheduler** — parallel task execution
- **Event Bus** — UI ↔ runtime communication

## Layer C — Subagent System (Parallel Intelligence)

7 frontier agents, spawned dynamically per task:

| Agent | Role |
|-------|------|
| RepoAgent | Scans repo structure, finds relevant files |
| MemoryRecallAgent | Queries LocusGraph for relevant memories |
| PatchAgent | Generates code patches |
| TestAgent | Runs tests, reports results |
| DebugAgent | Analyzes failures, proposes fixes |
| SearchAgent | Searches codebase (grep, LocusGraph SDK) |
| ConstraintAgent | Checks constraints, validates against rules |

## Layer D — ToolBus (Execution Gateway)

All actions go through ToolBus. Exposes:

- File read/write
- Repo search
- Command execution
- Git operations
- Diff generation
- Permission enforcement

This is where safety and determinism lives.

## Layer E — Multi-Model Engine

Routes tasks to different models based on mode:

| Mode | Model Selection | Use Case |
|------|----------------|----------|
| Rush | Cheap, fast model | Small tasks, quick edits |
| Smart | Balanced SOTA model | Default, general work |
| Deep | Strongest model | Complex reasoning, architecture |

Models are replaceable. Self-hosted.

## Layer F — LocusGraph (Deterministic Implicit Memory)

Stores everything as deterministic events:

- `context_id` = `type:typename`
- `payload` = fundamental truth
- Relations: RelatedTo, Extends, Contradicts, DerivedFrom, Reinforces
- Constraints and constraint violations
- Reinforcement: repeated success increases weight

This is the long-term intelligence.

## Layer G — Memory Injection Engine (Secret Weapon)

The LLM never "queries memory." Instead:

1. Orchestrator → asks MemoryRecallAgent
2. LocusGraph → returns relevant event bundle
3. Injection Engine → injects into LLM context

The LLM behaves like a human: it remembers without knowing the storage system exists.

## Layer H — Deterministic Event Extractor (Learning Engine)

After every action (diff/test/commit):

1. Logs + diffs → deterministic event extractor
2. Events written to LocusGraph
3. Relations + reinforcement updated
4. Constraints updated

This is how skills form naturally.

## Layer Interaction

```
User → [A: UI] → [B: Runtime/Orchestrator] → [C: Subagents]
                                             → [D: ToolBus] → filesystem/git/commands
                                             → [E: Multi-Model] → LLM
                        [G: Injection] ←→ [F: LocusGraph] ←→ [H: Extractor]
```
