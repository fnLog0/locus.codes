# Agent Reports

Structured output from each subagent back to the Orchestrator.

## Report Format

```json
{
  "agent_id": "agent-uuid",
  "agent_type": "RepoAgent",
  "task_id": "task-uuid",
  "status": "success",
  "result": {
    "files": ["src/auth/login.rs", "src/auth/mod.rs"],
    "summary": "Found 2 relevant files for auth task"
  },
  "artifacts": [],
  "duration_ms": 1200,
  "tokens_used": 0
}
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `agent_id` | string | Unique agent instance ID |
| `agent_type` | string | RepoAgent, PatchAgent, etc. |
| `task_id` | string | Parent task this agent was spawned for |
| `status` | enum | `success`, `failure`, `partial` |
| `result` | object | Agent-specific structured result |
| `artifacts` | array | Files modified, diffs produced, test results |
| `duration_ms` | u64 | Execution time |
| `tokens_used` | u64 | LLM tokens consumed (0 for non-LLM agents) |

## Status Values

| Status | Meaning |
|--------|---------|
| `success` | Agent completed task fully |
| `failure` | Agent could not complete task |
| `partial` | Agent completed some work but not all (e.g. found 1 of 3 files) |

## Result Schemas (per agent)

| Agent | Result Contains |
|-------|----------------|
| RepoAgent | `files`, `tree_summary` |
| MemoryRecallAgent | `memories` (memory bundle), `count` |
| PatchAgent | `diffs` (unified diffs per file) |
| TestAgent | `passed`, `failed`, `total`, `failures` |
| DebugAgent | `fix_diff`, `root_cause` |
| SearchAgent | `matches` (file, line, text) |
| ConstraintAgent | `violations`, `passed_constraints` |
