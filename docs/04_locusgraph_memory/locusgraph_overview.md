# LocusGraph Overview

Layer F — Deterministic Implicit Memory. The long-term intelligence of locus.codes.

## What It Is

LocusGraph stores everything the agent learns as **deterministic events**. It replaces AGENTS.md, Skills, and any static configuration. The agent's behavior is shaped by accumulated experience, not hardcoded instructions.

## What It Stores

Events with `event_kind`:

| Kind | Use |
|------|-----|
| `fact` | Recallable knowledge (file edits, project facts, user preferences) |
| `action` | Something that happened (command run, patch applied, commit made) |
| `decision` | Reasoning or choice (architecture decision, approach chosen) |
| `observation` | Outcome observed (test pass/fail, build result) |
| `feedback` | User signal (approval, rejection, correction) |

## How It Works

1. **Store events** — `store_event()` after every meaningful action
2. **Retrieve memories** — `retrieve_memories(query)` before acting, returns relevant context as a string
3. **Generate insights** — `generate_insights(task)` for summaries and reasoning
4. **Relations form** — events link via `related_to`, `extends`, `reinforces`, `contradicts`

## Key Properties

- **Deterministic**: events are structured, reproducible, auditable
- **Implicit**: the LLM never knows LocusGraph exists — memories just appear in context
- **Scoped**: `context_id` labels group events (e.g. `"terminal"`, `"editor"`, `"errors"`)
- **Searchable**: retrieval via LocusGraph SDK queries
- **Linked**: 4 relation fields connect events into a knowledge graph

## SDK API

| Method | Purpose |
|--------|---------|
| `store_event(CreateEventRequest)` | Store one memory event |
| `retrieve_memories(graph_id, query, limit, context_ids, context_types)` | LocusGraph SDK search → returns `memories` string |
| `generate_insights(graph_id, task, ...)` | Get `insight`, `recommendation`, `confidence` |
| `list_context_types(graph_id, ...)` | List context types in the graph |
| `list_contexts_by_type(graph_id, type, ...)` | List contexts of a given type |
| `search_contexts(graph_id, q, ...)` | Search contexts by name |
