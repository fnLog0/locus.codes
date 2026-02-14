# Memory Injection Engine

Layer G — The secret weapon. The LLM never "queries memory." Memories are injected transparently.

## Flow

```
Orchestrator
  → asks MemoryRecallAgent: "what's relevant for this task?"
  → MemoryRecallAgent calls: client.retrieve_memories(graph_id, query, limit, ...)
  → LocusGraph returns: ContextResult { memories: String, items_found: u64 }
  → Injection Engine inserts memories string into LLM prompt
  → LLM receives memories as if it "just knows"
```

## Why Implicit

The LLM behaves like a human: it remembers without knowing the storage system exists. There is no "query memory" tool. The memories simply appear in its context window.

This means:
- No wasted tokens on memory retrieval tool calls
- No hallucinated memory queries
- The model naturally uses relevant context

## Injection

The `memories` string from `retrieve_memories()` is inserted into the prompt between the system prompt and user prompt:

```
┌─────────────────────────────┐
│ System Prompt               │
├─────────────────────────────┤
│ {memories}                  │  ← from retrieve_memories()
├─────────────────────────────┤
│ Tool Definitions            │
├─────────────────────────────┤
│ User Prompt                 │
└─────────────────────────────┘
```

## Multiple Retrievals

For richer context, MemoryRecallAgent can make multiple calls:

```rust
let general = client.retrieve_memories(Some(gid), "project context", Some(5), None, None)?;
let constraints = client.retrieve_memories(Some(gid), "constraints", Some(10), Some(vec!["constraints".into()]), None)?;
let recent = client.retrieve_memories(Some(gid), &task_query, Some(5), None, None)?;
```

All results concatenated and injected.

## Insights as Context

For complex tasks, `generate_insights()` can also be injected:

```rust
let insight = client.generate_insights(Some(gid), "summarize relevant experience", None, None, None, None)?;
// insight.insight + insight.recommendation → inject into prompt
```
