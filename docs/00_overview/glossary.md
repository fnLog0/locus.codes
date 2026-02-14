# Glossary

| Term | Definition |
|------|------------|
| **LocusGraph** | Deterministic implicit memory system. Stores agent experience as events, retrieved via LocusGraph SDK. |
| **Event** | A stored memory unit: `event_kind` + `context_id` + `payload` + optional relations. |
| **event_kind** | Type of event: `fact`, `action`, `decision`, `observation`, `feedback`. |
| **context_id** | String label for scoping events (e.g. `"terminal"`, `"editor"`, `"constraints"`). |
| **Payload** | JSON content of an event. Contains the actual knowledge. |
| **Relation** | Link between events: `related_to`, `extends`, `reinforces`, `contradicts`. |
| **retrieve_memories** | LocusGraph SDK call → returns `memories` string + `items_found`. |
| **generate_insights** | SDK call: reasoning over stored memories → returns `insight` + `recommendation`. |
| **store_event** | SDK call: write one event to LocusGraph. |
| **graph_id** | Scoping identifier for the graph (one per agent or per user). |
| **ToolBus** | Execution gateway. All file, command, and git operations go through ToolBus. |
| **Subagent** | Independent agent spawned for a specific task. Has own context window. 7 types. |
| **Orchestrator** | Central coordinator. Builds DAG from prompt, manages full lifecycle. |
| **Scheduler** | Parallel task execution engine. Spawns subagents concurrently. |
| **DAG** | Directed acyclic graph. The execution plan built by the Orchestrator. |
| **Mode** | Execution mode: Rush (fast/cheap), Smart (balanced), Deep (strongest). |
| **Memory Injection** | Process of retrieving memories and inserting into LLM context transparently. |
| **Event Extractor** | Post-action system that stores events in LocusGraph after every action. |
| **Constraint** | Rule stored in LocusGraph (`context_id: "constraints"`) the agent must follow. |
| **Violation** | Event stored when an action violates a constraint. |
| **Reinforcement** | Using `reinforces` relation to signal success, boosting future retrieval rank. |
| **Session** | Current working context: repo, branch, mode, thread. |
| **Thread** | A saved interaction sequence. Can be shared and replayed. |
| **Patch** | Generated code change as unified diff. Applied atomically. |
| **Diff Review** | PR-style view for approving/rejecting patches. |
| **Event Bus** | Internal pub/sub for runtime ↔ UI communication. |
