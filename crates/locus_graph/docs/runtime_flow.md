# LocusGraph — Runtime Flow

How locus.codes thinks, acts, and learns. Not a pipeline — a cognitive loop.

---

## How a Human Developer Works

```
Sit down     → what was I doing? what do I know?
Read task    → what's being asked?
Think        → have I seen this before? what's relevant?
Plan         → what's my approach? am I confident?
Act          → execute
Check        → did it work? am I stuck?
Ask          → if stuck, escalate to someone
Reflect      → what did I learn?
Walk away    → save state for next time
```

The runtime follows the same loop.

---

## Three Layers

```
┌─────────────────────────────────┐
│  In-memory cache                │  nanoseconds
│  Safety data + trigger index    │  session-scoped, dies on exit
├─────────────────────────────────┤
│  cache.db                       │  microseconds
│  Read cache + write buffer      │  persistent, gRPC proxy to LocusGraph
├─────────────────────────────────┤
│  LocusGraph                     │  milliseconds
│  Source of truth + embeddings   │  semantic search, permanent storage
└─────────────────────────────────┘
```

`{type}:{name}` is just the indexing layer — how you find the drawer. The real knowledge is in the payloads, the links, and the act of digging deeper.

---

## Five Runtime Parts

| Part | What it does | I/O |
|---|---|---|
| **Memory Manager** | All LocusGraph communication. Exhaustive loads, semantic search, writes. Rest of runtime never touches graph directly. | Network |
| **Safety Cache** | In-memory snapshot. Constraints, rules, preferences, mistakes, trigger index. Populated once at session start. | None |
| **Context Builder** | Assembles LLM prompt. Always-block from cache + variable-block from search. Fills by priority until token budget runs out. | None |
| **Implicit Engine** | Sits between LLM and ToolBus. Checks every tool call against trigger index. Blocks or warns. | None |
| **Agent Loop** | The orchestrator. Drives the think→act→check→ask cycle. | Coordinates all |

---

## Sit Down — Session Start

**"What was I doing? What do I know?"**

### Step 1: Load safety data (1 exhaustive call)

```
Memory Manager → cache.db → LocusGraph (if stale)
  → all constraints, rules, preferences, mistakes
```

### Step 2: Build in-memory structures

| Data | Goes to |
|---|---|
| Rules with `trigger` field | Trigger index (HashMap by tool name) |
| Rules without `trigger` | Behavioral rules list (for LLM context) |
| Constraints | Both trigger index AND always-include list |
| Preferences | Always-include list |
| Mistakes | Filtered-include list (matched by task type) |

### Step 3: Load proficiency (1 call)

```
Memory Manager → proficiency levels per domain
  → stored as HashMap<domain, f64>
  → used as retrieval depth knob (NOT injected as text)
```

| Proficiency | Search depth |
|---|---|
| Low (< 0.3) | 15 results — agent needs more help |
| Medium (0.3-0.7) | 10 results — normal |
| High (> 0.7) | 5 results — agent knows this, less needed |

### Step 4: Resume or create session

```
Check session_anchor → active_session?
  → YES: resume, fetch existing turn contexts
  → NO: create new session
```

**Total: 2-3 reads through cache.db proxy. Zero remote writes.**

---

## Read Task — Turn Start

**"What's being asked?"**

```
User sends message
  → turn_sequence += 1
  → create turn anchor: turn:{session_id}_{turn_id}
  → store intent event to cache.db
```

No graph calls yet. Just bookkeeping.

---

## Think — Context Gathering

**"Have I seen this before? What's relevant?"**

### Semantic search (1 remote call)

```
Memory Manager → LocusGraph
  retrieve_memories(user_message, types: [skill, fact, pattern, turn], limit: by_proficiency)
```

This is the ONLY call that must go to LocusGraph — needs embeddings. Returns past turns, skills, facts, patterns relevant to this specific message.

**This is the "recognition" moment** — "I've seen this before." The surface. If the agent needs to dig deeper, it follows the links later (on demand, via tool_search or look-back).

### What's free (no call needed)

| Data | Source |
|---|---|
| Recent turns | Already in `Session.turns` in memory |
| Constraints, rules, prefs | In-memory cache from session start |
| Tool schemas | ToolBus.list_tools() in memory |

---

## Plan — Context Assembly

**"What's my approach?"**

Context Builder assembles the prompt:

```
ALWAYS INCLUDED (from in-memory cache):
  • Hard constraints        — what's forbidden
  • Behavioral rules        — how to work
  • User preferences        — style

VARIABLE (fill by priority, stop when budget runs out):
  1. Recent turn summaries  — continuity (from session memory, free)
  2. Semantic results       — relevant skills/facts/patterns (from search)
  3. Active mistakes        — anti-patterns for this task type (from cache)
  4. Soft rules             — best practices for tools in use (from cache)
```

Token budget adapts to model:

| Model | Available | What fits |
|---|---|---|
| 8k | ~5k for context | Always block + 2 turns + 3 results |
| 32k | ~25k | Always block + 5 turns + 10 results + mistakes |
| 128k+ | ~100k | Everything |

**Proficiency adjusts variable block** — low proficiency = more results pulled in = more tokens used = better answers for unfamiliar areas.

---

## Act — Tool Execution

**"Execute."**

LLM returns tool calls. For each:

### Implicit engine check (nanoseconds, zero I/O)

```
rules = trigger_index.get(tool_name) + trigger_index.get("*")

for rule in rules:
    check(rule.condition, tool_args)
```

| Result | Action |
|---|---|
| Pass | → ToolBus.call() |
| Soft violation | Queue warning for next LLM call, still execute |
| Hard violation | Block action, return error to LLM, skip ToolBus |

### ToolBus execution

```
ToolBus.call(tool_name, args) → result
```

### Store event (local write, microseconds)

```
action event → cache.db (WAL mode)
NOT synced to LocusGraph yet
```

### Return to LLM

LLM sees result → may make more tool calls → loop continues.

---

## Check — Am I Stuck?

**"Did it work?"**

The agent doesn't randomly retry. It watches for concrete signals:

| Signal | Source | Trigger |
|---|---|---|
| Same tool error repeated | Turn events in memory | 2+ times same error type |
| Hard constraint blocked | Implicit engine | Any hard block |
| Proficiency too low | Proficiency map | < 0.2 for the task domain |
| No relevant memories | Semantic search result | 0 items returned |
| Too many turns | Turn counter | 5+ turns, no resolution |
| LLM expresses uncertainty | Response text | "I'm not sure", "I think maybe" |

**Not stuck → continue acting.**

**Stuck → escalate.**

---

## Ask — Confirmation Required

**"I need help."**

When stuck, the agent doesn't guess. It shows its work:

```
"I tried X and hit Y.
 I think the options are A or B.
 I don't have enough context to choose.
 Which one?"
```

### When to ask

| Situation | What agent says |
|---|---|
| Hard constraint blocks | "I can't do X because of Y. What should I do instead?" |
| Same error twice | "I've failed at this twice. Should I try a different approach?" |
| Low proficiency + complex task | "I'm not familiar with this area. Here's my plan — confirm?" |
| Multiple valid approaches | "I see two ways: A or B. Which one?" |
| Decision contradicts past | "Last time we decided X. Now I'm thinking Y. Should I change?" |
| No semantic results | "I don't have context about this. Can you tell me more?" |

### What makes a good escalation

Bad: "What should I do?"
Good: "I tried X, hit Y. Options: A or B. I lean toward A because Z. Confirm?"

The agent presents: what it tried, what failed, what options it sees, which one it leans toward, and why. The user makes the final call.

### Confirmation feeds learning

```
User answers → feedback: event
  → linked to the decision
  → next time same situation → agent recalls feedback → doesn't need to ask
  → proficiency increases → fewer confirmations over time
```

**The agent that asks well today won't need to ask tomorrow.**

---

## Reflect — Turn End

**"What did I learn?"**

### Update turn anchor with summary

```
turn:{session_id}_{turn_id} → overwrite with:
  title, user_request, actions_taken, outcome, decisions,
  files_read, files_modified, event_count
```

### Batch sync to LocusGraph

```
All events queued in cache.db during this turn
  → single batch gRPC call → LocusGraph
  → mark synced in cache.db
```

### Update session totals

```
Increment: tool_calls, llm_calls, tokens, files_modified, errors
```

### Inject pending warnings

If soft violations occurred → queue for next LLM call:

```
[Rule Violation] rule:read_before_edit
You edited src/main.rs without reading it first.
```

**Total: 0 reads, 1 batch write.**

---

## Walk Away — Session End

**"Save state for next time."**

### Close session

```
session:{slug}_{id} → status: "closed", ended_at, summary, totals
session_anchor:{project}_{hash} → active_session: null
```

### Final sync

Flush any remaining cache.db events → LocusGraph.

### Background learning (async, never blocks)

| Check | Action | Link |
|---|---|---|
| Same constraint_violation > 2 times | Create `mistake:` | `related_to` violated rule |
| Same workflow across 3+ sessions | Create `pattern:` | `reinforces` itself |
| Pattern success > 80% for 5+ uses | Promote to `skill:` | `extends` skill_anchor |
| User approval feedback | Bump `proficiency:` | `reinforces` proficiency |
| User rejection feedback | Create/update `mistake:` | `contradicts` wrong pattern |

### Drop in-memory cache

Session over. Safety cache, trigger index, proficiency map — all freed. Next session starts fresh from cache.db.

---

## Digging Deeper (On Demand)

The surface flow handles most turns. But sometimes the agent needs to **dig** — follow the links to understand something deeply.

**Surface:** Semantic search returns `turn:a1b2c3d4_002` — "Fixed JWT validation"

**Dig one level:** Agent calls `tool_search("JWT validation details")` → retrieves events under that turn via `extends` chain → sees the full timeline: intent → read file → decision → edit → test → feedback

**Dig two levels:** Follows `contradicts` from `decision:..._002_005` → finds `decision:..._001_005` → reads WHY the first approach was wrong

**Dig three levels:** Follows `related_to` from the decision → finds `fact:auth_uses_jwt_refresh` → understands the domain

Each hop is a graph traversal. The agent decides how deep to dig based on:

| Situation | Depth |
|---|---|
| Simple task, high proficiency | Surface only (turn summaries) |
| Complex task, medium proficiency | One level (event timelines) |
| Debugging, low proficiency | Deep (decision chains, file history, error patterns) |
| User asks "why did we do X?" | As deep as needed (full traversal) |

Digging is not automatic. It's triggered when the surface isn't enough — when the agent is stuck, uncertain, or the user asks for history.

---

## Complete Turn — One Picture

```
USER MESSAGE
     │
     ▼
THINK ──→ Memory Manager (1 semantic search)
     │
     ▼
PLAN ──→ Context Builder (assemble prompt from cache + search)
     │
     ▼
LLM CALL ──→ stream response
     │
     ├── text response → done
     │
     └── tool call
           │
           ▼
     CHECK ──→ Implicit Engine (hash lookup, nanoseconds)
           │
           ├── hard block → error to LLM → loop
           ├── soft warn → queue warning, continue
           └── pass
                 │
                 ▼
           ACT ──→ ToolBus.call() → result
                 │
                 ▼
           STORE ──→ cache.db (local write)
                 │
                 ▼
           RETURN ──→ result to LLM → loop
                          │
                          ▼
                    STUCK? ──→ check signals
                          │
                          ├── no → LLM makes next call
                          └── yes → ASK user for confirmation
                                      │
                                      ▼
                                 USER RESPONDS
                                      │
                                      ▼
                                 feedback: stored → learning
```

**Per turn: 1 remote read, 1 batch write, everything else is in-memory.**

---

## First Session (Empty Graph)

| What | State | Effect |
|---|---|---|
| Constraints | Built-in defaults in binary | Safety works |
| Rules | Built-in defaults in binary | Best practices active |
| Preferences | Empty | No style adaptation yet |
| Proficiency | All 0.0 | Max search depth (most help) |
| Semantic search | Returns nothing | No enrichment |
| Stuck detection | All signals active | Agent asks more often |

Agent works fine — just no enrichment. Every action stores events. Graph fills up. Session 2 is better. Session 10 is much better. Session 100 — the agent rarely asks.

---

## LocusGraph Down

| Operation | Behavior |
|---|---|
| Session start load | Served from cache.db (last synced data) |
| Semantic search | Returns empty, `degraded = true` |
| Writes | Queued in cache.db, synced when reconnected |
| Implicit engine | Works normally (in-memory) |
| Stuck detection | Works normally |
| Learning | Deferred until reconnected |

Agent keeps working. Degraded but functional. Like a developer with amnesia — still knows how to code, just can't remember project specifics.

---

## Call Count Summary

| Phase | Remote reads | Remote writes |
|---|---|---|
| Session start | 2-3 (through cache.db) | 0 |
| Per turn | 1 (semantic search) | 0 |
| Per action | 0 | 0 |
| Turn end | 0 | 1 (batch) |
| Session end | 0 | 1-2 |
| Learning | 0 | 1-3 (async) |

**Hot path: 1 read per turn. Everything else is cached, local, or batched.**
