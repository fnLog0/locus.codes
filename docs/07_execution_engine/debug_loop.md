# Debug Loop

When tests fail, the DebugAgent enters a fix-test loop.

## Flow

```
Test failure
  → DebugAgent receives: failure output + changed files + original task
  → Analyzes root cause
  → Generates fix patch
  → Fix applied
  → Tests re-run
  → Pass? → continue to commit
  → Fail? → loop again (up to max retries)
```

## Max Retries per Mode

| Mode | Max debug iterations |
|------|---------------------|
| Rush | 0 (no debug loop, fail immediately) |
| Smart | 3 |
| Deep | 5 |

## Each Iteration

1. DebugAgent analyzes failure output via LLM
2. Recalls relevant memories (previous similar failures)
3. Generates a fix patch
4. Patch shown in Diff Review (if configured) or auto-applied
5. TestAgent runs tests again
6. Results evaluated

## Learning

Every debug iteration is stored in LocusGraph:
- What failed and why (observation)
- What fix was attempted (action)
- Whether the fix worked (observation → reinforces or not)

This means the agent gets better at debugging over time — it remembers what fixes worked for similar failures.

## Exit Conditions

| Condition | Action |
|-----------|--------|
| Tests pass | Exit loop, continue pipeline |
| Max retries reached | Report failure to user, show last error |
| User cancels | Abort, rollback changes |
