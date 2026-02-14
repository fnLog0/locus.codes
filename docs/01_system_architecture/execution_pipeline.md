# Execution Pipeline

The full 10-step lifecycle for every user prompt.

## Steps

### 1. User Prompt

User types prompt in the global prompt bar (UI Layer). Mode is selected (Rush/Smart/Deep). Prompt sent to Orchestrator.

### 2. Orchestrator Builds DAG

Orchestrator analyzes intent, breaks into subtasks, builds a DAG (directed acyclic graph) of tasks with dependencies and parallelizable branches.

### 3. Parallel Subagents Run

Scheduler dispatches parallelizable tasks simultaneously:
- **RepoAgent** scans repo structure, identifies relevant files
- **MemoryRecallAgent** queries LocusGraph for relevant context
- **SearchAgent** greps/searches codebase for patterns

### 4. Patch Generated

PatchAgent receives context (repo scan + memories + search results) and generates code changes via LLM. Output is unified diff format.

### 5. Diff Review Shown

Generated patches displayed in Diff Review view (PR-style). User can:
- Approve all changes
- Reject specific hunks
- Request modifications

### 6. Patch Applied

Approved patches applied atomically to the working directory. Rollback on failure.

### 7. Tests Run

TestAgent auto-detects test framework and runs project tests. Reports pass/fail per test.

### 8. Debug Loop (on failure)

If tests fail:
1. DebugAgent analyzes failure output
2. Proposes fix
3. Generates new patch
4. Tests again
5. Repeat until pass or max retries

### 9. Commit

After tests pass and user approves:
- LLM generates commit message from changes
- `git add` → `git commit` → optional `git push`

### 10. Event Extraction

Event Extractor processes the entire interaction:
- Diffs → fact events
- Test results → observation events
- Decisions → decision events
- All written to LocusGraph with relations and reinforcement

**This repeats forever, making the agent stronger with each interaction.**
