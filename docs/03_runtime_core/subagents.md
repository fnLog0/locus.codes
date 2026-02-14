# Subagent System

Layer C — Parallel Intelligence. 7 frontier agents, spawned dynamically per task. These are not fixed skills — they are spawned based on task requirements.

## Agent Roster

### RepoAgent
- **Role**: Scans repo structure, finds relevant files
- **Input**: Task description, repo metadata
- **Output**: File tree, relevant file paths, file contents
- **Tools**: file_read, grep, glob

### MemoryRecallAgent
- **Role**: Queries LocusGraph for relevant memories
- **Input**: Task context, repo identifier
- **Output**: Memory bundle (ranked locuses)
- **Tools**: LocusGraph retrieval API

### PatchAgent
- **Role**: Generates code patches
- **Input**: Task, relevant files, memory bundle, search results
- **Output**: Unified diffs
- **Tools**: file_read, file_write (via ToolBus)
- **LLM**: Yes (primary consumer of model)

### TestAgent
- **Role**: Runs tests, reports results
- **Input**: Changed files, test framework info
- **Output**: Pass/fail per test, stdout/stderr
- **Tools**: run_cmd

### DebugAgent
- **Role**: Analyzes failures, proposes fixes
- **Input**: Test failure output, changed files, original task
- **Output**: Fix patch (unified diff)
- **Tools**: file_read, grep, run_cmd
- **LLM**: Yes

### SearchAgent
- **Role**: Searches codebase for patterns
- **Input**: Search queries (from task analysis)
- **Output**: Matching files, lines, context
- **Tools**: grep, glob, file_read

### ConstraintAgent
- **Role**: Checks constraints, validates against rules
- **Input**: Proposed changes, active constraints from LocusGraph
- **Output**: Pass/fail per constraint, violations list
- **Tools**: LocusGraph retrieval

## Lifecycle

1. Orchestrator determines which agents are needed
2. Scheduler spawns agents concurrently (where possible)
3. Each agent runs with its own context window
4. Results sent back to Orchestrator via channels
5. Agent resources freed on completion

## Communication

- Agents do NOT communicate with each other directly
- All coordination goes through the Orchestrator
- Results are structured reports (see `08_protocols/agent_reports.md`)
