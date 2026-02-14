# Design Principles

## 1. Memory Over Configuration

LocusGraph replaces static files (AGENTS.md, Skills). The agent's behavior is shaped by accumulated experience, not hardcoded instructions.

## 2. Deterministic Events

Every action produces a deterministic event written to LocusGraph. Logs, diffs, test results — all become structured memories. Nothing is lost.

## 3. Implicit Memory

The LLM never "queries memory" directly. The Injection Engine retrieves relevant memories and injects them into the LLM context transparently. The LLM behaves like a human — it remembers without knowing how.

## 4. Parallel Intelligence

Subagents run concurrently. The Scheduler identifies parallelizable branches in the DAG and spawns agents simultaneously. No unnecessary serialization.

## 5. Safety Through ToolBus

All execution goes through ToolBus. Every file write, command, and git operation is permission-checked, logged, and auditable. ToolBus is where safety and determinism lives.

## 6. Multi-Model Routing

The right model for the right task. Rush uses cheap/fast models. Smart uses balanced SOTA. Deep uses the strongest model available. Models are replaceable.

## 7. Opinionated

Only ship features that work well. No half-baked capabilities. Every feature is tested, polished, and reliable.

## 8. On the Frontier

Evolve with models. No legacy baggage. When better models arrive, the system adapts without carrying dead code.
