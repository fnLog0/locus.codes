# Response Schema

Structured output expected from LLM responses.

## Response Format

```json
{
  "reasoning": "Chain of thought explaining the approach",
  "tool_calls": [
    {
      "tool": "file_write",
      "args": {
        "path": "src/auth/login.rs",
        "content": "..."
      }
    }
  ],
  "confidence": 0.85
}
```

## Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `reasoning` | string | Yes | Explanation of approach (chain of thought) |
| `tool_calls` | array | Yes | Tool invocations to execute via ToolBus |
| `confidence` | float | No | Model's self-assessed confidence (0.0–1.0) |

## Tool Call Structure

```json
{
  "tool": "tool_name",
  "args": { ... }
}
```

Tool names must match ToolBus API. Args must match tool signatures.

## Validation

1. Response must be valid JSON
2. `tool_calls` must reference valid tools
3. Tool args must match expected schema
4. Invalid responses trigger retry (up to max retries per mode)

## Error Handling

| Error | Action |
|-------|--------|
| Invalid JSON | Retry with error feedback |
| Unknown tool | Retry with available tools reminder |
| Missing args | Retry with schema reminder |
| Max retries exceeded | Fail task, report to Orchestrator |
