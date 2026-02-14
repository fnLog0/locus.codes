# Memory Trace

Optional debug view. Not shown by default. Shows LocusGraph activity during task execution.

## Content

- Memories recalled by MemoryRecallAgent (with confidence scores)
- Events extracted by Event Extractor (what was learned)
- Injection payloads sent to LLM (what context was provided)
- Link creation (new relations between locuses)

## Layout

```
┌──────────────────────────────────────────┐
│ Memory Trace  [debug]                    │
├──────────────────────────────────────────┤
│ RECALLED (7 locuses)                     │
│  0.92 fact:auth_design "Uses JWT RS256"  │
│  0.85 rule:test_first "Always run tests" │
│  0.71 fact:login_rs "Token validation"   │
│                                          │
│ INJECTED → LLM context (1,204 tokens)   │
│                                          │
│ EXTRACTED (3 events)                     │
│  + fact:token_expiry_check               │
│  + observation:test_auth_expired_fixed   │
│  ↗ reinforces fact:auth_design           │
├──────────────────────────────────────────┤
│ > _                                      │
└──────────────────────────────────────────┘
```

## Use Case

- Debugging memory behavior
- Understanding why the agent made certain decisions
- Verifying that relevant context was recalled
- Inspecting what the agent learned from an interaction
