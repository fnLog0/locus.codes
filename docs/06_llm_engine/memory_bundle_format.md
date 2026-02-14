# Memory Bundle Format

The memory bundle is the `memories` string returned by `retrieve_memories()`, injected into the LLM prompt.

## Format

The `memories` field from `ContextResult` is a pre-formatted string. The agent does not need to parse or restructure it — inject it directly into the prompt.

```rust
let result = client.retrieve_memories(Some("coding-agent"), query, Some(10), None, None)?;
// result.memories → ready to inject
// result.items_found → how many items matched
```

## Injection Position

```
┌─────────────────────────────┐
│ System Prompt               │
├─────────────────────────────┤
│ {memories from retrieval}   │  ← inject here
├─────────────────────────────┤
│ Tool Definitions            │
├─────────────────────────────┤
│ User Prompt                 │
└─────────────────────────────┘
```

## Multiple Retrievals

For richer context, the agent can make scoped retrievals and concatenate:

```rust
let context = client.retrieve_memories(Some(gid), "project context", Some(5), None, None)?;
let constraints = client.retrieve_memories(Some(gid), "rules", Some(10), Some(vec!["constraints".into()]), None)?;
let relevant = client.retrieve_memories(Some(gid), &task_text, Some(5), None, None)?;

let bundle = format!("{}\n\n{}\n\n{}", context.memories, constraints.memories, relevant.memories);
```

## Insights as Context

For complex tasks, `generate_insights()` provides summarized reasoning:

```rust
let insight = client.generate_insights(Some(gid), "relevant experience for this task", None, None, None, None)?;
// insight.insight → summary
// insight.recommendation → suggested approach
// insight.confidence → server confidence
```

## Budget

Total injection size depends on mode:

| Mode | Retrieval limit | Approximate tokens |
|------|----------------|--------------------|
| Rush | 5 | ~500 |
| Smart | 10 | ~2,000 |
| Deep | 20 | ~5,000 |
