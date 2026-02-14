# Safety and Limits

## Token Budgets

| Mode | Input budget | Output budget | Total |
|------|-------------|---------------|-------|
| Rush | 4K | 2K | 6K |
| Smart | 16K | 8K | 24K |
| Deep | 32K | 16K | 48K |

## Timeouts

| Mode | Per-request timeout |
|------|-------------------|
| Rush | 30s |
| Smart | 120s |
| Deep | 300s |

## Retry Limits

| Mode | Max retries |
|------|-------------|
| Rush | 1 |
| Smart | 3 |
| Deep | 5 |

## Safety Rules

- **No secrets in prompts**: environment variables filtered before injection
- **Output sanitization**: responses scanned for accidental secret leaks
- **No network in commands**: sandboxed execution blocks network by default
- **Rate limiting**: max requests per minute per model (configurable)
- **Audit trail**: all LLM requests/responses logged (with secret redaction)

## Fallback Behavior

| Failure | Action |
|---------|--------|
| Model timeout | Retry once, then fail task |
| Model unavailable | Fall back to next available model (Rush/Smart only) |
| Token budget exceeded | Truncate context, retry |
| All retries failed | Report failure to Orchestrator, surface to user |
