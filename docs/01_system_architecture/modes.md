# Modes

locus.codes supports 3 execution modes. The Mode Controller selects the active mode, which affects model routing, context budget, and subagent behavior.

## Mode Definitions

| Mode | Model | Context Budget | Use Case |
|------|-------|---------------|----------|
| **Rush** | Cheap, fast model | Minimal | Small tasks, quick edits, simple fixes |
| **Smart** | Balanced SOTA model | Standard | Default mode, general development work |
| **Deep** | Strongest model | Maximum | Complex reasoning, architecture, hard bugs |

## Mode Effects

### Rush
- Routes to the cheapest/fastest available model
- Reduced context window (fewer memories injected)
- Fewer subagents spawned (skip SearchAgent if unnecessary)
- No debug loop (fail fast, report to user)
- Best for: rename variable, fix typo, add import, simple one-file edits

### Smart
- Routes to balanced SOTA model
- Full context window with memory injection
- All relevant subagents spawned
- Standard debug loop (3 retries)
- Best for: feature implementation, bug fixes, refactoring

### Deep
- Routes to the strongest available model
- Maximum context with deep memory recall
- All subagents spawned with extended context
- Extended debug loop (5+ retries)
- Extended thinking / chain-of-thought enabled
- Best for: architecture decisions, cross-file refactors, complex debugging

## Mode Selection

- User selects mode explicitly in the prompt bar
- Default mode is **Smart**
- Mode can be changed mid-session
- Mode indicator shown in UI prompt bar
