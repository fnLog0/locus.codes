# Runtime Events

Events emitted by the runtime via Event Bus. Used by UI for real-time updates and by Event Extractor for learning.

## Event Types

| Event | Emitted When | Payload |
|-------|-------------|---------|
| `TaskStarted` | Orchestrator begins processing prompt | task_id, prompt, mode |
| `TaskCompleted` | All pipeline steps done | task_id, summary, duration |
| `TaskFailed` | Pipeline failed (max retries, user cancel) | task_id, error, step |
| `AgentSpawned` | Scheduler starts a subagent | agent_id, agent_type, task |
| `AgentCompleted` | Subagent finishes | agent_id, status, result |
| `ToolCalled` | ToolBus receives a tool call | tool, args, agent_id |
| `ToolResult` | ToolBus returns result | tool, success, result, duration |
| `DiffGenerated` | PatchAgent produces diffs | files, hunks_count |
| `DiffApproved` | User approves diff | files |
| `DiffRejected` | User rejects diff | files, reason |
| `TestResult` | TestAgent completes | passed, failed, total, output |
| `DebugIteration` | DebugAgent starts a retry | iteration, failure_summary |
| `CommitCreated` | Git commit made | hash, message, files |
| `MemoryRecalled` | MemoryRecallAgent returns | locus_count, top_confidence |
| `MemoryStored` | Event Extractor writes to LocusGraph | event_kind, context_id |
| `ModeChanged` | User switches mode | old_mode, new_mode |

## Consumers

| Consumer | Uses Events For |
|----------|----------------|
| **UI (Event Bus)** | Real-time view updates |
| **Event Extractor** | Learning from outcomes |
| **Orchestrator** | Pipeline control flow |
| **Logs View** | Display command output |
