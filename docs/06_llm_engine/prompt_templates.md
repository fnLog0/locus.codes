# Prompt Templates

Prompt structure for LLM requests. Templates are minimal — LocusGraph provides the context that would normally live in AGENTS.md.

## Structure

```
┌─────────────────────────────┐
│ System Prompt               │  ← Agent role definition
├─────────────────────────────┤
│ Memory Bundle (injected)    │  ← From Injection Engine
│  - Project context          │
│  - Active constraints       │
│  - Recent decisions         │
│  - Relevant experience      │
├─────────────────────────────┤
│ Tool Definitions            │  ← Available ToolBus tools
├─────────────────────────────┤
│ User Prompt                 │  ← The actual task
└─────────────────────────────┘
```

## System Prompts (per agent type)

| Agent | System Prompt Focus |
|-------|-------------------|
| PatchAgent | "You generate code patches. Output unified diffs." |
| DebugAgent | "You analyze test failures and propose fixes." |
| SearchAgent | "You search codebases to find relevant code." |
| TestAgent | "You run tests and report results." |
| RepoAgent | "You scan repository structure to find relevant files." |
| ConstraintAgent | "You validate changes against constraints." |

## Key Principle

System prompts are **short and focused**. Project-specific knowledge comes from the memory bundle, not from hardcoded prompt text. This is what makes locus.codes self-improving — the prompt gets better as LocusGraph accumulates experience.
